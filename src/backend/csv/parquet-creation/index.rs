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
use std::env;

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
struct ColumnDefinition {
    column: String,
    #[serde(rename = "type")]
    column_type: DataType,
}

#[derive(serde::Deserialize, Debug)]
struct ParquetCreationRequest {
    payload: Vec<ColumnDefinition>,
    #[serde(rename = "s3Key")]
    s3_key: String,
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
    let sqs_client = SqsClient::new(&config);
    let dynamo_client = DynamoClient::new(&config);

    let dynamo_name = env::var("DYNAMODB_NAME")?;
    let queue_url = env::var("PARQUET_QUEUE_URL")?;

    let body = event.payload.body.unwrap_or_default();

    let request: ParquetCreationRequest = serde_json::from_str(&body)
        .map_err(|e| lambda_runtime::Error::from(format!("Failed to parse JSON: {}", e)))?;

    sqs_client
        .send_message()
        .queue_url(&queue_url)
        .message_body(body)
        .send()
        .await?;

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    Ok(ApiGatewayV2httpResponse {
        status_code: 200,
        headers,
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(
            json!({
                "parquet_key": "key"
            })
            .to_string(),
        )),
        is_base64_encoded: false,
        cookies: vec![],
    })
}
