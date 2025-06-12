use aws_lambda_events::{
    apigw::{ApiGatewayV2httpRequest, ApiGatewayV2httpResponse},
    encodings::Body,
    http::HeaderMap,
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

    let response_body = json!({
        "response_message": "not working yet"
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
