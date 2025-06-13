use aws_config::BehaviorVersion;
use aws_lambda_events::{
    apigw::{ApiGatewayV2httpRequest, ApiGatewayV2httpResponse},
    encodings::Body,
    http::HeaderMap,
};
use aws_sdk_bedrockruntime::{
    Client as BedrockClient,
    operation::converse::{ConverseError, ConverseOutput},
    types::{ContentBlock, ConversationRole, Message, SystemContentBlock},
};
use aws_sdk_s3::Client as S3Client;
use lambda_runtime::{Error, LambdaEvent, service_fn};
use polars::prelude::*;
use polars_sql::SQLContext;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::fs;
use uuid::Uuid;

use std::path::PathBuf;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
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
    event: LambdaEvent<ApiGatewayV2httpRequest>,
) -> Result<ApiGatewayV2httpResponse, Error> {
    let body = event.payload.body.unwrap_or_default();
    let bucket_name = env::var("S3_UPLOAD_BUCKET_NAME")?;

    let request: GenerateParquetQuery = serde_json::from_str(&body)
        .map_err(|e| lambda_runtime::Error::from(format!("Failed to parse JSON: {}", e)))?;

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    // Initialize S3 client
    let config = aws_config::load_from_env().await;
    let s3_client = S3Client::new(&config);

    let temp_file_path =
        download_parquet_to_tmp(&s3_client, &bucket_name, &request.parquet_key).await?;

    let lf = LazyFrame::scan_parquet(&temp_file_path, Default::default())?;

    let collected_rows: DataFrame = lf.clone().limit(1).collect()?;
    let schema = collected_rows.schema();

    let schema_string = schema
        .iter()
        .map(|(name, dtype)| format!("  {}: {:?}", name, dtype))
        .collect::<Vec<_>>()
        .join("\n");
    println!("Schema:\n{}", schema_string);

    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region("ap-southeast-2")
        .load()
        .await;

    let bedrock_client = BedrockClient::new(&sdk_config);

    const USER_MESSAGE: &str = r#" You are going to be given a schema for a parquet file and a query from a user related to querying that schema.
    You will need to make an SQL query from that schema and only return the SQL query and nothing else. No reasoning as to why. Just an SQL query.
    I will be using that SQL in a polars sql query in rust.

    example schema would be
     Schema:
        Name: String
        Country: String
        Play: String

    The message might contain some details about things not related to the schema, only extract things from the message related to the schema.

    for example a dataset may be about cars, but if someone says select all red cars and tell me how hot the day is, only extract stuff related to the car. If you can't find a schema match, don't put it in.

    the user message might be like I want to know the name of all basketball players from Australia. Don't include any \n.

    understand that the polars is reading from a parquet file that is saved in memory of a lambda so there will be no table to select from

    Return an SQL statment like this nothing else. No extra special characters, make sure it's all on one line

    The table name must always be data, nothing else.

    SELECT * FROM data WHERE Country = 'Australia' AND Play = 'Basketball';
    "#;

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

    let output: String = match bedrock_response {
        Ok(output) => {
            let text = get_converse_output_text(output)?;
            text
        }
        Err(_) => todo!(),
    };

    println!("{:?}", output);

    let mut ctx = SQLContext::new();

    // Register the LazyFrame with a table name
    ctx.register("data", lf.clone());

    // Execute the SQL query (assuming 'output' contains your SQL string)
    let result_df = ctx.execute(&output)?.collect()?;

    let json_data = serde_json::to_string_pretty(&result_df.to_string())?;

    println!("{:?}", json_data);

    const MAKE_HUMAN_READABLE: &str = r#"You will be given some data extraced from a parquet file, make the data nice and presentable to an end user
    ensure that you are only returning things related to the data, I do not need a reason why you did it the way you did.
    Only return things related to the data.

    Using the user question and the information from the parquet file return a message that a user will udnerstand.

    E.G if someone asks I want to know the name of the most popular state in Australia, use that message as well as the information gathered to then present a message.

    respond with something that gives them an accurate reply to their message and don't output just the raw data
    "#;

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
        Ok(read_output) => {
            let text = get_converse_output_text(read_output)?;
            text
        }
        Err(_) => todo!(),
    };

    println!("{:?}", readable_output);

    let response_body = json!({
        "response_message": readable_output
    });

    Ok(ApiGatewayV2httpResponse {
        status_code: 200,
        headers,
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(response_body.to_string())),
        is_base64_encoded: false,
        cookies: vec![],
    })
}

async fn download_parquet_to_tmp(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // Generate random filename
    let random_id = Uuid::new_v4();
    let filename = format!("{}.parquet", random_id);
    let temp_path = PathBuf::from("/tmp").join(filename);

    // Download from S3
    let response = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    // Read the data
    let data = response.body.collect().await?;
    let bytes = data.into_bytes();

    // Write to temp file
    fs::write(&temp_path, bytes)?;

    Ok(temp_path)
}

fn get_converse_output_text(output: ConverseOutput) -> Result<String, Error> {
    let text = output
        .output()
        .ok_or("no output")?
        .as_message()
        .map_err(|_| "output not a message")?
        .content()
        .first()
        .ok_or("no content in message")?
        .as_text()
        .map_err(|_| "content is not text")?
        .to_string();
    Ok(text)
}
