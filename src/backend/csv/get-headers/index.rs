use aws_lambda_events::encodings::Body;
use aws_lambda_events::event::apigw::ApiGatewayProxyRequest;
use aws_sdk_s3::Client;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_stream::StreamExt;

#[derive(Deserialize)]
struct RequestPayload {
    bucket: String,
    key: String,
}

#[derive(Serialize)]
struct ResponsePayload {
    headers: Vec<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn function_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<serde_json::Value, Error> {
    // Initialize S3 client
    let config = aws_config::load_from_env().await;
    let s3_client = Client::new(&config);

    // Parse request body
    let body = match event.payload.body {
        Some(Body::Text(body)) => body,
        _ => {
            return Ok(serde_json::json!({
                "statusCode": 400,
                "body": serde_json::to_string(&ErrorResponse {
                    error: "Request body is required".to_string()
                })?
            }));
        }
    };

    let request_payload: RequestPayload = match serde_json::from_str(&body) {
        Ok(payload) => payload,
        Err(_) => {
            return Ok(serde_json::json!({
                "statusCode": 400,
                "body": serde_json::to_string(&ErrorResponse {
                    error: "Invalid JSON in request body. Expected: {\"bucket\": \"bucket-name\", \"key\": \"file.csv\"}".to_string()
                })?
            }));
        }
    };

    // Get CSV file from S3
    let csv_content =
        match get_csv_from_s3(&s3_client, &request_payload.bucket, &request_payload.key).await {
            Ok(content) => content,
            Err(e) => {
                return Ok(serde_json::json!({
                    "statusCode": 500,
                    "body": serde_json::to_string(&ErrorResponse {
                        error: format!("Failed to retrieve CSV from S3: {}", e)
                    })?
                }));
            }
        };

    // Extract headers from CSV
    let headers = match extract_csv_headers(&csv_content) {
        Ok(headers) => headers,
        Err(e) => {
            return Ok(serde_json::json!({
                "statusCode": 500,
                "body": serde_json::to_string(&ErrorResponse {
                    error: format!("Failed to parse CSV headers: {}", e)
                })?
            }));
        }
    };

    // Return successful response
    let response = ResponsePayload { headers };
    Ok(serde_json::json!({
        "statusCode": 200,
        "headers": {
            "Content-Type": "application/json"
        },
        "body": serde_json::to_string(&response)?
    }))
}

async fn get_csv_from_s3(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let resp = client.get_object().bucket(bucket).key(key).send().await?;

    let mut body = resp.body;
    let mut content = Vec::new();

    while let Some(bytes) = body.try_next().await? {
        content.extend_from_slice(&bytes);
    }

    let csv_content = String::from_utf8(content)?;
    Ok(csv_content)
}

fn extract_csv_headers(
    csv_content: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());

    // Get headers
    let headers = reader.headers()?;
    let header_vec: Vec<String> = headers.iter().map(|h| h.to_string()).collect();

    if header_vec.is_empty() {
        return Err("CSV file appears to be empty or has no headers".into());
    }

    Ok(header_vec)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}
