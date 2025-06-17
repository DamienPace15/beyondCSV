use aws_lambda_events::{event::sqs::SqsEvent, sqs::SqsMessage};
use common::parquet_creation_processor::{
    ColumnDefinition, stream_csv_to_parquet_multipart, update_job_status_to_success,
};
use lambda_runtime::{Error, LambdaEvent, service_fn};
use std::env;
use tracing::error;

#[derive(serde::Deserialize, Debug)]
struct ParquetCreationRequest {
    payload: Vec<ColumnDefinition>,
    s3_key: String,
    job_id: String,
}

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

async fn handler(event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
    let bucket_name = env::var("S3_UPLOAD_BUCKET_NAME")?;
    let table_name = env::var("DYNAMODB_NAME")?;

    for record in event.payload.records {
        if let Err(e) = process_sqs_message(&record, &bucket_name, &table_name).await {
            error!(
                "Failed to process SQS message {}: {}",
                record.message_id.unwrap_or_default(),
                e
            );
            continue;
        }
    }
    Ok(())
}

async fn process_sqs_message(
    record: &SqsMessage,
    bucket_name: &str,
    table_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = record.body.as_ref().ok_or("SQS message has no body")?;

    let request: ParquetCreationRequest = serde_json::from_str(body)
        .map_err(|e| format!("Failed to parse JSON from SQS message: {}", e))?;

    println!(
        "Processing job {} with {} columns",
        request.job_id,
        request.payload.len()
    );

    let start_time = std::time::Instant::now();

    let parquet_key = format!("parquet/{}.parquet", request.job_id);

    stream_csv_to_parquet_multipart(
        bucket_name,
        &request.s3_key,
        &request.payload,
        &parquet_key,
        &request.job_id,
    )
    .await?;

    println!(
        "Job {} converted to Parquet in {:.2} seconds",
        request.job_id,
        start_time.elapsed().as_secs_f64()
    );

    update_job_status_to_success(table_name, &request.job_id).await?;

    Ok(())
}
