use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_sdk_dynamodb::Client;
use common::cors::create_cors_response;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}

async fn function_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    if event.payload.http_method == "OPTIONS" {
        return Ok(create_cors_response(200, None));
    }

    let job_id = match event.payload.path_parameters.get("job_id") {
        Some(id) => id,
        None => {
            return Ok(create_cors_response(
                400,
                Some(json!({"error": "Missing job_id in path"}).to_string()),
            ));
        }
    };

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let table_name = std::env::var("DYNAMODB_NAME")?;

    let pk = format!("JOB-{}", job_id);
    let sk = job_id.clone();

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
                            return Ok(create_cors_response(
                                500,
                                Some(
                                    json!({"error": "Status field not found or invalid type"})
                                        .to_string(),
                                ),
                            ));
                        }
                    };

                    // Extract context from the item
                    let context = match item.get("context") {
                        Some(aws_sdk_dynamodb::types::AttributeValue::S(context_value)) => {
                            context_value.clone()
                        }
                        _ => String::new(), // Default to empty string if not found
                    };

                    // Extract schema from the item (DynamoDB Map)
                    let schema = match item.get("schema") {
                        Some(aws_sdk_dynamodb::types::AttributeValue::M(schema_map)) => {
                            let mut result_map = HashMap::new();
                            for (key, value) in schema_map {
                                if let aws_sdk_dynamodb::types::AttributeValue::S(string_value) =
                                    value
                                {
                                    result_map.insert(key.clone(), string_value.clone());
                                }
                            }
                            result_map
                        }
                        _ => HashMap::new(), // Default to empty map if not found
                    };

                    // Determine parquet_complete based on status
                    let parquet_complete = match status {
                        "success" => true,
                        "pending" => false,
                        _ => {
                            return Ok(create_cors_response(
                                400,
                                Some(json!({"error": "Invalid status value"}).to_string()),
                            ));
                        }
                    };

                    // Return flat JSON structure with context and schema
                    let response_body = json!({
                        "statusCode": 200,
                        "parquet_complete": parquet_complete,
                        "context": context,
                        "schema": schema
                    });

                    Ok(create_cors_response(200, Some(response_body.to_string())))
                }
                None => Ok(create_cors_response(
                    404,
                    Some(json!({"error": "Job not found"}).to_string()),
                )),
            }
        }
        Err(e) => {
            eprintln!("DynamoDB error: {:?}", e);
            Ok(create_cors_response(
                500,
                Some(json!({"error": "Internal server error"}).to_string()),
            ))
        }
    }
}
