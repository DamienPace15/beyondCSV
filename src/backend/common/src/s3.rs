use aws_sdk_s3::Client as S3Client;
use lambda_runtime::Error;

pub async fn upload_to_s3(
    bucket: &str,
    key: &str,
    parquet_data: Vec<u8>,
    job_id: &str,
) -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let s3_client = S3Client::new(&config);

    println!(
        "Job {}: Uploading parquet to S3: bucket={}, key={}, size={:.2} MB",
        job_id,
        bucket,
        key,
        parquet_data.len() as f64 / (1024.0 * 1024.0)
    );

    s3_client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(parquet_data.into())
        .content_type("application/octet-stream")
        .send()
        .await?;

    println!("Job {}: Successfully uploaded parquet file", job_id);
    Ok(())
}
