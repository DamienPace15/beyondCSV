use aws_config::BehaviorVersion;
use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_sdk_bedrockruntime::{
    Client as BedrockClient,
    types::{ContentBlock, ConversationRole, Message, SystemContentBlock},
};
use aws_sdk_s3::Client as S3Client;
use common::{
    cors::create_cors_response,
    duck_db::{execute_sql_on_parquet_file, get_schema_from_parquet_file, setup_duckdb_connection},
    dynamo::get_job_by_id,
    parquet_query::get_converse_output_text,
    query_prompts::{MAKE_HUMAN_READABLE, USER_MESSAGE},
};
use lambda_runtime::{Error, LambdaEvent, service_fn};
use serde::Deserialize;
use serde_json::json;
use std::env;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

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
                    json!({"error": "Failed to parse JSON", "details": e.to_string()}).to_string(),
                ),
            ));
        }
    };

    let sdk_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
    let bedrock_client = BedrockClient::new(&sdk_config);
    let s3_client = S3Client::new(&sdk_config);

    let temp_file_path = format!(
        "/tmp/{}",
        request
            .parquet_key
            .split('/')
            .last()
            .unwrap_or("temp.parquet")
    );
    println!(
        "Downloading S3 object s3://{}/{} to {}",
        bucket_name, request.parquet_key, temp_file_path
    );

    match s3_client
        .get_object()
        .bucket(&bucket_name)
        .key(&request.parquet_key)
        .send()
        .await
    {
        Ok(s3_output) => {
            let mut byte_stream = s3_output.body;
            let mut file = File::create(&temp_file_path).await?;
            while let Some(chunk) = byte_stream.try_next().await? {
                file.write_all(&chunk).await?;
            }
            println!("Successfully downloaded file to {}", temp_file_path);
        }
        Err(e) => {
            eprintln!("Failed to download from S3: {:?}", e);
            return Ok(create_cors_response(500, Some(json!({"error": "Failed to download Parquet file from S3", "details": e.to_string()}).to_string())));
        }
    }

    let conn = match setup_duckdb_connection() {
        Ok(conn) => conn,
        Err(e) => {
            return Ok(create_cors_response(
                500,
                Some(
                    json!({"error": "Failed to setup DuckDB connection", "details": e.to_string()})
                        .to_string(),
                ),
            ));
        }
    };

    let schema_string = match get_schema_from_parquet_file(&conn, &temp_file_path) {
        Ok(schema) => schema,
        Err(e) => {
            return Ok(create_cors_response(500, Some(json!({"error": "Failed to get schema from local parquet file", "details": e.to_string()}).to_string())));
        }
    };

    println!("Schema: {}", schema_string);

    let bedrock_response = bedrock_client
        .converse()
        .model_id("apac.anthropic.claude-sonnet-4-20250514-v1:0")
        .system(SystemContentBlock::Text(USER_MESSAGE.to_string()))
        .messages(
            Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(format!(
                    "schema: {}, question: {}",
                    schema_string, request.message
                )))
                .build()?,
        )
        .send()
        .await;

    let sql_query: String = match bedrock_response {
        Ok(output) => get_converse_output_text(output)?,
        Err(e) => {
            eprintln!("Bedrock converse error: {:?}", e);
            return Ok(create_cors_response(500, Some(json!({"error": "Failed to generate SQL query", "details": format!("Bedrock API error: {}", e)}).to_string())));
        }
    };

    println!("Generated SQL Query: {}", sql_query);

    let structured_data = match execute_sql_on_parquet_file(&conn, &temp_file_path, &sql_query) {
        Ok(data) => data,
        Err(e) => {
            return Ok(create_cors_response(500, Some(json!({"error": "Failed to execute SQL query on local data", "details": e.to_string()}).to_string())));
        }
    };

    let json_data = serde_json::to_string_pretty(&structured_data)?;
    println!("{:?}", json_data);

    let job_record = get_job_by_id(&table_name, &request.job_id).await?.unwrap();

    let make_human_presentable = bedrock_client
        .converse()
        .model_id("apac.anthropic.claude-sonnet-4-20250514-v1:0")
        .system(SystemContentBlock::Text(MAKE_HUMAN_READABLE.to_string()))
        .messages(
            Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(format!(
                    "data that needs to be presentable: {}, user question: {}, dataset context: {}",
                    json_data, request.message, job_record.context
                )))
                .build()?,
        )
        .send()
        .await;

    let readable_output = match make_human_presentable {
        Ok(output) => get_converse_output_text(output)?,
        Err(e) => format!("Bedrock make readable error: {}", e),
    };

    println!("Human readable output: {}", readable_output);

    let response_body = json!({ "response_message": readable_output });
    Ok(create_cors_response(200, Some(response_body.to_string())))
}
