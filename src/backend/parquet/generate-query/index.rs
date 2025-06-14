use aws_config::BehaviorVersion;
use aws_lambda_events::{
    apigw::{ApiGatewayV2httpRequest, ApiGatewayV2httpResponse},
    encodings::Body,
    http::HeaderMap,
};
use aws_sdk_bedrockruntime::{
    Client as BedrockClient,
    operation::converse::ConverseOutput,
    types::{ContentBlock, ConversationRole, Message, SystemContentBlock},
};
use aws_sdk_s3::Client as S3Client;
use lambda_runtime::{Error, LambdaEvent, service_fn};
use polars::prelude::*;
use polars_sql::SQLContext;
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::fs;
use uuid::Uuid;

use std::path::PathBuf;

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

    Make sure to use the best SQL queries, and use correct formatting

    Make sure to use the best SQL queries, and use correct formatting

    IMPORTANT RULES:
   1. For counting queries (like "how many", "count of", etc.), use COUNT(*) or COUNT(column_name)
    2. For aggregate functions, use appropriate functions: SUM(), AVG(), MIN(), MAX(), COUNT()
    3. When using COUNT(*), the result should be a single number
    4. Use GROUP BY when counting by categories (e.g., "count by state")
    5. Use DISTINCT when counting unique values
    6. Always use single quotes for string literals, not double quotes
    7. Column names with spaces must be quoted with double quotes
    8. The table name must always be 'data'
    9. Use LIMIT when appropriate to avoid returning excessive rows
    10. Use WHERE clauses before GROUP BY for better performance
    11. Use specific column names instead of SELECT * when possible
    12. Use UPPER() or LOWER() for case-insensitive string comparisons when needed
    13. Use ORDER BY for sorted results when logical
    14. Avoid unnecessary JOINs and subqueries
    15. Use IN() for multiple value comparisons instead of multiple OR conditions
    16. Use EXISTS instead of IN for better performance with large datasets
    17. Use appropriate comparison operators (=, >, <, >=, <=, LIKE, BETWEEN)

    Examples:
    - "How many Tesla cars?" → SELECT COUNT(*) FROM data WHERE make = 'Tesla'
    - "Count by state" → SELECT state, COUNT(*) FROM data GROUP BY state ORDER BY COUNT(*) DESC
    - "How many unique makes?" → SELECT COUNT(DISTINCT make) FROM data
    - "Top 10 most expensive cars" → SELECT make, model, price FROM data ORDER BY price DESC LIMIT 10

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

    The table name must always be data, nothing else. Only parse one statement at a time.

    Please make sure to qoute anything that has spaces properly. For example if you see a field in the Schema and use it to query something. give me the most common clean alternative fuel would become "clean alternative fuel" in a query

    ANything that has spaces in the query needs to be quoted.

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
        Err(e) => {
            eprintln!("Bedrock converse error: {:?}", e);
            return Ok(ApiGatewayV2httpResponse {
                status_code: 500,
                headers,
                multi_value_headers: HeaderMap::new(),
                body: Some(Body::Text(
                    json!({
                        "error": "Failed to generate SQL query",
                        "details": format!("Bedrock API error: {}", e)
                    })
                    .to_string(),
                )),
                is_base64_encoded: false,
                cookies: vec![],
            });
        }
    };

    println!("{:?}", output);

    let mut ctx = SQLContext::new();

    // Register the LazyFrame with a table name
    ctx.register("data", lf.clone());

    // Execute the SQL query (assuming 'output' contains your SQL string)
    let result_df = ctx.execute(&output)?.collect()?;

    let column_names: Vec<&PlSmallStr> = result_df.get_column_names();
    let mut rows = Vec::new();

    for i in 0..result_df.height() {
        let mut row = serde_json::Map::new();
        for (col_idx, &col_name) in column_names.iter().enumerate() {
            let column = result_df.get_columns()[col_idx].clone();
            let value = column.get(i).unwrap();
            row.insert(col_name.to_string(), json!(value.to_string()));
        }
        rows.push(json!(row));
    }

    let structured_data = json!({
        "metadata": {
            "rows": result_df.height(),
            "columns": column_names
        },
        "data": rows
    });

    let json_data = serde_json::to_string_pretty(&structured_data)?;

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
        Err(e) => {
            format!("Bedrock make readable error: {:?}", e)
        }
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
