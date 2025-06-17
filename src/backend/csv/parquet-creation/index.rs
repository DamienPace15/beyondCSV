use aws_config::BehaviorVersion;
use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::{Client as DynamoClient, Error as DynamoError};
use aws_sdk_sqs::Client as SqsClient;
use common::cors::create_cors_response;
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
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    // Handle OPTIONS requests for CORS preflight
    if event.payload.http_method == "OPTIONS" {
        return Ok(create_cors_response(200, None));
    }

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

    Ok(create_cors_response(
        200,
        Some(
            json!({
                "job_id": request.job_id
            })
            .to_string(),
        ),
    ))
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
