use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_sdk_dynamodb::Client;
use common::cors::create_cors_response;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Debug)]
struct UpdateContextRequest {
    context: String,
    job_id: String,
}

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

    // Parse request body
    let body = event.payload.body.unwrap_or_default();
    let request: UpdateContextRequest = match serde_json::from_str(&body) {
        Ok(req) => req,
        Err(e) => {
            return Ok(create_cors_response(
                400,
                Some(json!({"error": format!("Invalid request body: {}", e)}).to_string()),
            ));
        }
    };

    println!("{:?}", request);

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let table_name = std::env::var("DYNAMODB_NAME")?;

    let pk = format!("JOB-{}", request.job_id);

    // Update the context field in DynamoDB
    let result = client
        .update_item()
        .table_name(&table_name)
        .key(
            "service",
            aws_sdk_dynamodb::types::AttributeValue::S(pk.to_string()),
        )
        .key(
            "serviceId",
            aws_sdk_dynamodb::types::AttributeValue::S(request.job_id.to_string()),
        )
        .update_expression("SET #ctx = :context")
        .expression_attribute_names("#ctx", "context")
        .expression_attribute_values(
            ":context",
            aws_sdk_dynamodb::types::AttributeValue::S(request.context.to_string()),
        )
        .send()
        .await;

    match result {
        Ok(_) => {
            let response_body = json!({
                "statusCode": 200,
                "message": "Context updated successfully"
            });

            Ok(create_cors_response(200, Some(response_body.to_string())))
        }
        Err(e) => {
            eprintln!("DynamoDB error: {:?}", e);
            Ok(create_cors_response(
                500,
                Some(json!({"error": "Failed to update context"}).to_string()),
            ))
        }
    }
}
