use aws_config::BehaviorVersion;
use aws_lambda_events::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_sdk_bedrockruntime::{
    Client as BedrockClient,
    types::{ContentBlock, ConversationRole, Message, SystemContentBlock},
};
use aws_sdk_s3::Client as S3Client;
use common::{
    cors::create_cors_response,
    parquet_query::{get_converse_output_text, stream_parquet_from_s3},
    query_prompts::USER_MESSAGE,
};
use lambda_runtime::{Error, LambdaEvent, service_fn};
use polars::prelude::*;
use polars_sql::SQLContext;
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::io::Cursor;

#[tokio::main]
async fn main() -> Result<(), Error> {
    unsafe { std::env::set_var("POLARS_MAX_THREADS", "2") };
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
    let config = aws_config::load_from_env().await;
    let s3_client = S3Client::new(&config);
    let sdk_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
    let bedrock_client = BedrockClient::new(&sdk_config);

    // OPTIMIZATION 1: Single-pass parquet processing
    // Stream parquet data directly into memory once
    let parquet_bytes =
        match stream_parquet_from_s3(&s3_client, &bucket_name, &request.parquet_key).await {
            Ok(bytes) => bytes,
            Err(e) => {
                return Ok(create_cors_response(
                    500,
                    Some(
                        json!({
                            "error": "Failed to read parquet file from S3",
                            "details": format!("{}", e)
                        })
                        .to_string(),
                    ),
                ));
            }
        };

    // OPTIMIZATION 2: Create LazyFrame directly and extract schema without materialization
    let df = match ParquetReader::new(Cursor::new(parquet_bytes)).finish() {
        Ok(df) => df,
        Err(e) => {
            return Ok(create_cors_response(
                500,
                Some(
                    json!({
                        "error": "Failed to read parquet data",
                        "details": format!("{}", e)
                    })
                    .to_string(),
                ),
            ));
        }
    };

    // Convert to LazyFrame immediately to avoid keeping DataFrame in memory
    let mut lf = df.lazy();

    // OPTIMIZATION 3: Get schema from LazyFrame metadata without collecting
    let schema = match lf.collect_schema() {
        Ok(schema) => schema,
        Err(e) => {
            return Ok(create_cors_response(
                500,
                Some(
                    json!({
                        "error": "Failed to collect schema information",
                        "details": format!("{}", e)
                    })
                    .to_string(),
                ),
            ));
        }
    };

    let schema_string = schema
        .iter()
        .map(|(name, dtype)| format!("  {}: {:?}", name, dtype))
        .collect::<Vec<_>>()
        .join("\n");

    println!("{:?}", schema_string);

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

    println!("What is query? {:?}", sql_query);

    // OPTIMIZATION 4: Keep everything in lazy evaluation until final collection
    let mut ctx = SQLContext::new();
    ctx.register("data", lf);

    // Execute SQL query and collect only once at the very end
    let result_df = match ctx.execute(&sql_query) {
        Ok(lazy_frame) => {
            // CRITICAL: Only materialize the data once here at the final step
            match lazy_frame.collect() {
                Ok(df) => df,
                Err(e) => {
                    return Ok(create_cors_response(
                        500,
                        Some(
                            json!({
                                "error": "Failed to collect SQL query results",
                                "details": format!("{}", e)
                            })
                            .to_string(),
                        ),
                    ));
                }
            }
        }
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

    println!("{:?}", result_df);

    // OPTIMIZATION 5: Efficient result serialization
    let column_names: Vec<&PlSmallStr> = result_df.get_column_names();
    let mut rows = Vec::with_capacity(result_df.height());

    for i in 0..result_df.height() {
        let mut row = serde_json::Map::with_capacity(column_names.len());
        for (col_idx, &col_name) in column_names.iter().enumerate() {
            let column = &result_df.get_columns()[col_idx];
            let value = column.get(i).unwrap();
            row.insert(col_name.to_string(), json!(value.to_string()));
        }
        rows.push(json!(row));
    }

    let structured_data = json!({
        "metadata": {
            "columns": column_names
        },
        "data": rows
    });

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

    // Make results human-readable
    const MAKE_HUMAN_READABLE: &str = r#"You will be given some data extraced from a parquet file, make the data nice and presentable to an end user
    ensure that you are only returning things related to the data, I do not need a reason why you did it the way you did.
    Only return things related to the data.

    Using the user question and the information from the parquet file return a message that a user will udnerstand.

    E.G if someone asks I want to know the name of the most popular state in Australia, use that message as well as the information gathered to then present a message.

    respond with something that gives them an accurate reply to their message and don't output just the raw data
    "#;

    println!("{:?}", json_data);
    println!("{:?}", request.message);

    let make_human_presentable = bedrock_client
        .converse()
        .model_id("apac.amazon.nova-pro-v1:0")
        .system(SystemContentBlock::Text(MAKE_HUMAN_READABLE.to_string()))
        .messages(
            Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(format!(
                    "data that needs to be presentable: {}, user question: {}",
                    json_data, request.message
                )))
                .build()
                .unwrap(),
        )
        .send()
        .await;

    let readable_output: String = match make_human_presentable {
        Ok(read_output) => match get_converse_output_text(read_output) {
            Ok(text) => text,
            Err(e) => {
                format!("Failed to extract readable output: {}", e)
            }
        },
        Err(e) => {
            format!("Bedrock make readable error: {}", e)
        }
    };

    println!("{:?}", readable_output);

    let response_body = json!({
        "response_message": readable_output
    });

    Ok(create_cors_response(200, Some(response_body.to_string())))
}
