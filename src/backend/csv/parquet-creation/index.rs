use aws_config::BehaviorVersion;
use aws_lambda_events::{
    apigw::{ApiGatewayV2httpRequest, ApiGatewayV2httpResponse},
    encodings::Body,
    http::HeaderMap,
};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::{Client as DynamoClient, Error as DynamoError};
use aws_sdk_sqs::Client as SqsClient;
use lambda_runtime::{Error, LambdaEvent, service_fn};
use serde_json::json;
use std::{collections::HashMap, env};

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
enum DataType {
    String,
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    Timestamp,
}

#[derive(serde::Deserialize, Debug)]
struct ParquetCreationRequest {
    job_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .init();

    let handler = service_fn(handler);
    lambda_runtime::run(handler).await?;
    Ok(())
}

async fn handler(
    event: LambdaEvent<ApiGatewayV2httpRequest>,
) -> Result<ApiGatewayV2httpResponse, Error> {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;

    let dynamo_name = env::var("DYNAMODB_NAME")?;
    let queue_url = env::var("PARQUET_QUEUE_URL")?;

    let sqs_client = SqsClient::new(&config);
    let dynamo_client = DynamoClient::new(&config);

    let body = event.payload.body.unwrap_or_default();

    let request: ParquetCreationRequest = serde_json::from_str(&body)
        .map_err(|e| lambda_runtime::Error::from(format!("Failed to parse JSON: {}", e)))?;

    sqs_client
        .send_message()
        .queue_url(&queue_url)
        .message_body(body)
        .send()
        .await?;

    let service = format!("JOB-{}", request.job_id);

    put_job_status(
        &dynamo_client,
        &dynamo_name,
        &service,
        &request.job_id,
        "pending",
    )
    .await?;

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    Ok(ApiGatewayV2httpResponse {
        status_code: 200,
        headers,
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(
            json!({
                "job_id": request.job_id
            })
            .to_string(),
        )),
        is_base64_encoded: false,
        cookies: vec![],
    })
}

async fn put_job_status(
    dynamo_client: &DynamoClient,
    table_name: &str,
    service: &str,
    service_id: &str,
    status: &str,
) -> Result<(), DynamoError> {
    let mut item = HashMap::new();

    item.insert(
        "service".to_string(),
        AttributeValue::S(service.to_string()),
    );
    item.insert(
        "serviceId".to_string(),
        AttributeValue::S(service_id.to_string()),
    );
    item.insert("status".to_string(), AttributeValue::S(status.to_string()));

    dynamo_client
        .put_item()
        .table_name(table_name)
        .set_item(Some(item))
        .send()
        .await?;

    Ok(())
}
