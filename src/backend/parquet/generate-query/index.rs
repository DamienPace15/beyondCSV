use aws_config::BehaviorVersion;
use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_sdk_bedrockruntime::{
    Client as BedrockClient,
    types::{ContentBlock, ConversationRole, Message, SystemContentBlock},
};
use common::{
    cors::create_cors_response,
    duck_db::{get_schema_from_parquet, setup_duckdb_connection},
    dynamo::get_job_by_id,
    parquet_query::{execute_sql_query, get_converse_output_text},
    query_prompts::{MAKE_HUMAN_READABLE, USER_MESSAGE},
};
use lambda_runtime::{Error, LambdaEvent, service_fn};
use serde::Deserialize;
use serde_json::json;
use std::env;

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

#[derive(Deserialize, Debug)]
struct GenerateParquetQuery {
    message: String,
    parquet_key: String,
    job_id: String,
}

async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    // Handle OPTIONS requests for CORS preflight
    if event.payload.http_method == "OPTIONS" {
        return Ok(create_cors_response(200, None));
    }

    let body = event.payload.body.unwrap_or_default();
    let bucket_name = env::var("S3_UPLOAD_BUCKET_NAME")?;
    let table_name = env::var("DYNAMODB_NAME")?;

    let request: GenerateParquetQuery = match serde_json::from_str(&body) {
        Ok(req) => req,
        Err(e) => {
            return Ok(create_cors_response(
                400,
                Some(
                    json!({
                        "error": "Failed to parse JSON",
                        "details": format!("{}", e)
                    })
                    .to_string(),
                ),
            ));
        }
    };

    // Initialize clients
    let sdk_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
    let bedrock_client = BedrockClient::new(&sdk_config);

    // Setup DuckDB connection with S3 support
    let conn = match setup_duckdb_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Ok(create_cors_response(
                500,
                Some(
                    json!({
                        "error": "Failed to setup DuckDB connection",
                        "details": format!("{}", e)
                    })
                    .to_string(),
                ),
            ));
        }
    };

    // Construct S3 path for direct querying
    let s3_path = format!("s3://{}/{}", bucket_name, request.parquet_key);

    // Get schema directly from S3 parquet file
    let schema_string = match get_schema_from_parquet(&conn, &s3_path) {
        Ok(schema) => schema,
        Err(e) => {
            return Ok(create_cors_response(
                500,
                Some(
                    json!({
                        "error": "Failed to get schema from parquet file",
                        "details": format!("{}", e)
                    })
                    .to_string(),
                ),
            ));
        }
    };

    println!("Schema: {}", schema_string);

    // Generate SQL query using Bedrock
    let bedrock_response = bedrock_client
        .converse()
        .model_id("apac.amazon.nova-pro-v1:0")
        .system(SystemContentBlock::Text(USER_MESSAGE.to_string()))
        .messages(
            Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(format!(
                    "schema: {}, question: {}",
                    schema_string, request.message
                )))
                .build()
                .unwrap(),
        )
        .send()
        .await;

    let sql_query: String = match bedrock_response {
        Ok(output) => match get_converse_output_text(output) {
            Ok(text) => text,
            Err(e) => {
                return Ok(create_cors_response(
                    500,
                    Some(
                        json!({
                            "error": "Failed to extract text from Bedrock response",
                            "details": format!("{}", e)
                        })
                        .to_string(),
                    ),
                ));
            }
        },
        Err(e) => {
            eprintln!("Bedrock converse error: {:?}", e);
            return Ok(create_cors_response(
                500,
                Some(
                    json!({
                        "error": "Failed to generate SQL query",
                        "details": format!("Bedrock API error: {}", e)
                    })
                    .to_string(),
                ),
            ));
        }
    };

    println!("Generated SQL Query: {}", sql_query);

    // Execute SQL query directly on S3 parquet file
    let structured_data = match execute_sql_query(&conn, &s3_path, &sql_query) {
        Ok(data) => data,
        Err(e) => {
            return Ok(create_cors_response(
                500,
                Some(
                    json!({
                        "error": "Failed to execute SQL query",
                        "details": format!("{}", e)
                    })
                    .to_string(),
                ),
            ));
        }
    };

    let json_data = match serde_json::to_string_pretty(&structured_data) {
        Ok(data) => data,
        Err(e) => {
            return Ok(create_cors_response(
                500,
                Some(
                    json!({
                        "error": "Failed to serialize query results",
                        "details": format!("{}", e)
                    })
                    .to_string(),
                ),
            ));
        }
    };

    println!("{:?}", json_data);

    // Get job context for human-readable output
    let job_record = get_job_by_id(&table_name, &request.job_id).await?.unwrap();

    // Make output human-readable using Bedrock
    let make_human_presentable = bedrock_client
        .converse()
        .model_id("apac.amazon.nova-pro-v1:0")
        .system(SystemContentBlock::Text(MAKE_HUMAN_READABLE.to_string()))
        .messages(
            Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(format!(
                    "data that needs to be presentable: {}, user question: {}, dataset context: {}",
                    json_data, request.message, job_record.context
                )))
                .build()
                .unwrap(),
        )
        .send()
        .await;

    let readable_output: String = match make_human_presentable {
        Ok(read_output) => match get_converse_output_text(read_output) {
            Ok(text) => text,
            Err(e) => format!("Failed to extract readable output: {}", e),
        },
        Err(e) => format!("Bedrock make readable error: {}", e),
    };

    println!("Human readable output: {}", readable_output);

    let response_body = json!({
        "response_message": readable_output
    });

    Ok(create_cors_response(200, Some(response_body.to_string())))
}
