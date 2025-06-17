use aws_sdk_bedrockruntime::operation::converse::ConverseOutput;
use aws_sdk_s3::Client as S3Client;
use lambda_runtime::Error;

pub async fn stream_parquet_from_s3(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Stream from S3 directly into memory
    let response = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    // Collect the bytes into a Vec<u8>
    let data = response.body.collect().await?;
    let bytes = data.into_bytes().to_vec();

    Ok(bytes)
}

pub fn get_converse_output_text(output: ConverseOutput) -> Result<String, Error> {
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
