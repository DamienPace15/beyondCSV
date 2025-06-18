use common::{
    creation_types::{ColumnDefinition, DataType},
    dynamo::update_job_status_to_success,
    test_creation_processor::stream_csv_to_parquet_optimized,
};
use lambda_runtime::{Error, LambdaEvent, service_fn};
use std::env;
use tracing::info;

// Empty event type since we're hardcoding everything
#[derive(serde::Deserialize, Debug)]
struct EmptyEvent {}

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

async fn handler(_event: LambdaEvent<EmptyEvent>) -> Result<(), Error> {
    info!("Starting optimized parquet conversion test");

    let bucket_name = env::var("S3_UPLOAD_BUCKET_NAME")?;
    let table_name = env::var("DYNAMODB_NAME")?;

    // Hardcoded payload from your JSON
    let hardcoded_payload = vec![
        ColumnDefinition {
            column: "City".to_string(),
            column_type: DataType::String,
        },
        ColumnDefinition {
            column: "State".to_string(),
            column_type: DataType::String,
        },
        ColumnDefinition {
            column: "Country".to_string(),
            column_type: DataType::String,
        },
        ColumnDefinition {
            column: "Product ID".to_string(),
            column_type: DataType::String,
        },
        ColumnDefinition {
            column: "Product Category".to_string(),
            column_type: DataType::String,
        },
        ColumnDefinition {
            column: "Sales Volume".to_string(),
            column_type: DataType::Float,
        },
        ColumnDefinition {
            column: "Sales Revenue".to_string(),
            column_type: DataType::Float,
        },
        ColumnDefinition {
            column: "Date".to_string(),
            column_type: DataType::Date,
        },
    ];

    // Hardcoded values
    let hardcoded_s3_key = "csvUpload/9a621683-8f57-4f50-99c5-607554fb85df.csv";
    let hardcoded_job_id = "9a621683-8f57-4f50-99c5-607554fb85df";

    info!(
        "Processing hardcoded job {} with {} columns using OPTIMIZED approach",
        hardcoded_job_id,
        hardcoded_payload.len()
    );

    let start_time = std::time::Instant::now();

    let parquet_key = format!("parquet/{}.parquet", hardcoded_job_id);

    // Call the OPTIMIZED function with hardcoded values
    match stream_csv_to_parquet_optimized(
        &bucket_name,
        hardcoded_s3_key,
        &hardcoded_payload,
        &parquet_key,
        hardcoded_job_id,
    )
    .await
    {
        Ok(_) => {
            let duration = start_time.elapsed().as_secs_f64();
            info!(
                "Job {} converted to Parquet using OPTIMIZED processing in {:.2} seconds",
                hardcoded_job_id, duration
            );

            // Update job status to success
            match update_job_status_to_success(&table_name, hardcoded_job_id).await {
                Ok(_) => info!("Successfully updated job status to success"),
                Err(e) => {
                    tracing::error!("Failed to update job status: {}", e);
                    return Err(format!("Failed to update job status: {}", e).into());
                }
            }

            info!("Optimized test completed successfully!");
        }
        Err(e) => {
            tracing::error!("Failed to process CSV to Parquet: {}", e);
            return Err(format!("Failed to process CSV to Parquet: {}", e).into());
        }
    }

    Ok(())
}
