use aws_lambda_events::{
    encodings::Body,
    event::apigw::{ApiGatewayV2httpRequest, ApiGatewayV2httpResponse},
    http::HeaderMap,
};
use aws_sdk_dynamodb::Client;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use serde_json::json;

async fn function_handler(
    event: LambdaEvent<ApiGatewayV2httpRequest>,
) -> Result<ApiGatewayV2httpResponse, Error> {
    // Extract job_id from path parameters
    let job_id = match event.payload.path_parameters.get("job_id") {
        Some(id) => id,
        None => {
            return Ok(create_error_response(400, "Missing job_id in path"));
        }
    };

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let table_name = std::env::var("DYNAMODB_NAME")?;

    let pk = format!("JOB-{}", job_id);
    let sk = job_id.clone();

    println!("what is the job id? {:?}", pk);

    let result = client
        .get_item()
        .table_name(&table_name)
        .key("service", aws_sdk_dynamodb::types::AttributeValue::S(pk))
        .key("serviceId", aws_sdk_dynamodb::types::AttributeValue::S(sk))
        .send()
        .await;

    match result {
        Ok(output) => {
            match output.item {
                Some(item) => {
                    // Extract status from the item
                    let status = match item.get("status") {
                        Some(aws_sdk_dynamodb::types::AttributeValue::S(status_value)) => {
                            status_value.as_str()
                        }
                        _ => {
                            return Ok(create_error_response(
                                500,
                                "Status field not found or invalid type",
                            ));
                        }
                    };

                    // Determine parquet_complete based on status
                    let parquet_complete = match status {
                        "success" => true,
                        "pending" => false,
                        _ => {
                            return Ok(create_error_response(400, "Invalid status value"));
                        }
                    };

                    // Return flat JSON structure to match TypeScript expectations
                    let response_body = json!({
                        "statusCode": 200,
                        "parquet_complete": parquet_complete
                    });

                    let mut headers = HeaderMap::new();
                    headers.insert("Content-Type", "application/json".parse().unwrap());

                    Ok(ApiGatewayV2httpResponse {
                        status_code: 200,
                        headers,
                        body: Some(Body::Text(response_body.to_string())), // Convert to String
                        is_base64_encoded: false,
                        multi_value_headers: HeaderMap::new(),
                        cookies: vec![],
                    })
                }
                None => Ok(create_error_response(404, "Job not found")),
            }
        }
        Err(e) => {
            eprintln!("DynamoDB error: {:?}", e);
            Ok(create_error_response(500, "Internal server error"))
        }
    }
}

fn create_error_response(status_code: i64, message: &str) -> ApiGatewayV2httpResponse {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    ApiGatewayV2httpResponse {
        status_code,
        headers,
        body: Some(Body::Text(json!({"error": message}).to_string())), // Convert to String
        is_base64_encoded: false,
        cookies: vec![],
        multi_value_headers: HeaderMap::new(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}
