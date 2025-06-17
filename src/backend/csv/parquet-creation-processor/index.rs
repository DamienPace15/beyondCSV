use aws_lambda_events::event::sqs::{SqsEvent, SqsMessage};
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_s3::Client as S3Client;
use lambda_runtime::{Error, LambdaEvent, service_fn};
use std::env;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{error, warn};

use arrow::array::{
    ArrayRef, BooleanArray, Date32Array, Float64Array, Int64Array, StringArray,
    TimestampNanosecondArray,
};
use arrow::datatypes::{DataType as ArrowDataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::collections::HashMap;
use std::sync::Arc;

// Optimized constants for 3GB Lambda with 300MB files
const ROWS_PER_BATCH: usize = 50_000; // Much smaller batches to reduce memory usage
const S3_CHUNK_SIZE: usize = 8 * 1024 * 1024; // 8MB S3 read buffer (reduced)
const MAX_BATCH_MEMORY: usize = 100 * 1024 * 1024; // 100MB max per batch

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
enum DataType {
    String,
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    Timestamp,
}

impl DataType {
    fn to_arrow_type(&self) -> ArrowDataType {
        match self {
            DataType::String => ArrowDataType::Utf8,
            DataType::Integer => ArrowDataType::Int64,
            DataType::Float => ArrowDataType::Float64,
            DataType::Boolean => ArrowDataType::Boolean,
            DataType::Date => ArrowDataType::Date32,
            DataType::DateTime | DataType::Timestamp => {
                ArrowDataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into()))
            }
        }
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::String => write!(f, "string"),
            DataType::Integer => write!(f, "integer"),
            DataType::Float => write!(f, "float"),
            DataType::Boolean => write!(f, "boolean"),
            DataType::Date => write!(f, "date"),
            DataType::DateTime => write!(f, "datetime"),
            DataType::Timestamp => write!(f, "timestamp"),
        }
    }
}

#[derive(serde::Deserialize, Debug)]
struct ColumnDefinition {
    column: String,
    #[serde(rename = "type")]
    column_type: DataType,
}

#[derive(serde::Deserialize, Debug)]
struct ParquetCreationRequest {
    payload: Vec<ColumnDefinition>,
    s3_key: String,
    job_id: String,
}

// More efficient row representation using Vec instead of HashMap
type EfficientRow = Vec<Option<String>>;

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

    // Process each SQS message
    for record in event.payload.records {
        if let Err(e) = process_sqs_message(&record, &bucket_name, &table_name).await {
            error!(
                "Failed to process SQS message {}: {}",
                record.message_id.unwrap_or_default(),
                e
            );
            // Continue processing other messages instead of failing the entire batch
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

    println!("What is the body {:?}", body);

    let request: ParquetCreationRequest = serde_json::from_str(body)
        .map_err(|e| format!("Failed to parse JSON from SQS message: {}", e))?;

    println!("What is the request {:?}", request);

    println!(
        "Processing job {} with {} columns",
        request.job_id,
        request.payload.len()
    );
    println!("S3 key: {}", request.s3_key);

    let start_time = std::time::Instant::now();

    // Use job_id in the parquet filename for better traceability
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

    println!("Am I getting there?");

    // Update DynamoDB record to success
    update_job_status_to_success(table_name, &request.job_id).await?;

    Ok(())
}

async fn stream_csv_to_parquet_multipart(
    bucket: &str,
    key: &str,
    column_definitions: &[ColumnDefinition],
    output_key: &str,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = aws_config::load_from_env().await;
    let s3_client = S3Client::new(&config);

    println!(
        "Job {}: Starting memory-efficient streaming from S3: bucket={}, key={}",
        job_id, bucket, key
    );

    // Get file size for progress tracking
    let head_response = s3_client
        .head_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;
    let content_length = head_response.content_length().unwrap_or(0);

    println!(
        "Job {}: File size: {:.2} MB",
        job_id,
        content_length as f64 / (1024.0 * 1024.0)
    );

    // Get the S3 object stream
    let response = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    // Convert ByteStream to AsyncRead with smaller buffer
    let byte_stream = response.body.into_async_read();
    let mut lines = BufReader::with_capacity(S3_CHUNK_SIZE, byte_stream).lines();

    // Create Parquet schema
    let fields: Vec<Field> = column_definitions
        .iter()
        .map(|col| Field::new(&col.column, col.column_type.to_arrow_type(), true))
        .collect();
    let schema = Arc::new(Schema::new(fields));

    // Create column index mapping for efficient access
    let column_indices: HashMap<String, usize> = column_definitions
        .iter()
        .enumerate()
        .map(|(i, col)| (col.column.clone(), i))
        .collect();

    // Initialize Parquet writer with smaller buffer
    let mut buffer = Vec::with_capacity(10 * 1024 * 1024); // Start with 10MB
    let props = WriterProperties::builder()
        .set_compression(parquet::basic::Compression::SNAPPY)
        .set_write_batch_size(ROWS_PER_BATCH)
        .set_data_page_size_limit(1024 * 1024) // 1MB pages (reduced)
        .set_dictionary_page_size_limit(1024 * 1024) // 1MB dictionary (reduced)
        .set_max_row_group_size(100_000) // Smaller row groups
        .build();

    let mut writer = ArrowWriter::try_new(&mut buffer, schema.clone(), Some(props))?;

    let mut headers: Option<Vec<String>> = None;
    let mut header_indices: HashMap<String, usize> = HashMap::new();
    let mut batch_rows = Vec::with_capacity(ROWS_PER_BATCH);
    let mut total_rows = 0;
    let mut estimated_batch_size = 0;
    let start_time = std::time::Instant::now();

    // Process lines as they stream in
    while let Some(line_result) = lines.next_line().await.transpose() {
        let line = match line_result {
            Ok(line) => line,
            Err(e) => {
                error!("Job {}: Error reading line: {}", job_id, e);
                continue;
            }
        };

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Parse headers from first line
        if headers.is_none() {
            match parse_csv_headers(&line) {
                Ok(parsed_headers) => {
                    // Create header index mapping
                    for (idx, header) in parsed_headers.iter().enumerate() {
                        header_indices.insert(header.clone(), idx);
                    }
                    headers = Some(parsed_headers);
                    println!(
                        "Job {}: Headers parsed: {} columns",
                        job_id,
                        headers.as_ref().unwrap().len()
                    );
                    continue;
                }
                Err(e) => {
                    error!("Job {}: Failed to parse headers: {}", job_id, e);
                    return Err("Invalid CSV headers".into());
                }
            }
        }

        // Parse data row efficiently
        if let Some(ref _headers) = headers {
            match parse_csv_row_efficient(
                &line,
                &column_indices,
                &header_indices,
                column_definitions.len(),
            ) {
                Ok(row) => {
                    estimated_batch_size += estimate_row_size_efficient(&row);
                    batch_rows.push(row);
                    total_rows += 1;

                    // Write batch when full or approaching memory limit
                    if batch_rows.len() >= ROWS_PER_BATCH || estimated_batch_size > MAX_BATCH_MEMORY
                    {
                        let batch_start = std::time::Instant::now();
                        let batch = create_record_batch_efficient(
                            &batch_rows,
                            column_definitions,
                            schema.clone(),
                        )?;
                        writer.write(&batch)?;

                        if total_rows % 50_000 == 0 {
                            let elapsed = start_time.elapsed().as_secs_f64();
                            let throughput = (total_rows as f64 / elapsed) / 1000.0;
                            println!(
                                "Job {}: Processed {} rows in {:.2}s, batch: {:.2}s, {:.1}K rows/s",
                                job_id,
                                total_rows,
                                elapsed,
                                batch_start.elapsed().as_secs_f64(),
                                throughput
                            );
                        }
                        batch_rows.clear();
                        estimated_batch_size = 0;
                    }
                }
                Err(e) => {
                    warn!(
                        "Job {}: Failed to parse row {}: {}",
                        job_id,
                        total_rows + 1,
                        e
                    );
                    continue;
                }
            }
        }
    }

    // Write remaining rows
    if !batch_rows.is_empty() {
        let batch = create_record_batch_efficient(&batch_rows, column_definitions, schema.clone())?;
        writer.write(&batch)?;
        println!(
            "Job {}: Wrote final batch: {} rows",
            job_id,
            batch_rows.len()
        );
    }

    let total_time = start_time.elapsed().as_secs_f64();
    println!(
        "Job {}: Total rows processed: {}, total time: {:.2}s, avg: {:.1}K rows/s",
        job_id,
        total_rows,
        total_time,
        (total_rows as f64 / total_time) / 1000.0
    );

    writer.close()?;

    // Upload the parquet file
    upload_to_s3(bucket, output_key, buffer, job_id).await?;

    Ok(())
}

fn parse_csv_headers(line: &str) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .buffer_capacity(8 * 1024)
        .from_reader(line.as_bytes());

    if let Some(result) = reader.records().next() {
        let record = result?;
        Ok(record
            .iter()
            .map(|field| field.trim().to_string())
            .collect())
    } else {
        Err("No header row found".into())
    }
}

fn parse_csv_row_efficient(
    line: &str,
    column_indices: &HashMap<String, usize>,
    header_indices: &HashMap<String, usize>,
    num_columns: usize,
) -> Result<EfficientRow, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .buffer_capacity(8 * 1024)
        .from_reader(line.as_bytes());

    if let Some(result) = reader.records().next() {
        let record = result?;
        let mut row = vec![None; num_columns];

        for (csv_col_name, csv_col_idx) in header_indices.iter() {
            if let Some(&schema_col_idx) = column_indices.get(csv_col_name) {
                if let Some(field) = record.get(*csv_col_idx) {
                    let value = if field.trim().is_empty() {
                        None
                    } else {
                        Some(field.to_string())
                    };
                    row[schema_col_idx] = value;
                }
            }
        }

        Ok(row)
    } else {
        Err("No data in row".into())
    }
}

fn estimate_row_size_efficient(row: &EfficientRow) -> usize {
    row.iter()
        .map(|v| v.as_ref().map_or(8, |s| s.len()) + 24) // Vec overhead is less than HashMap
        .sum()
}

fn create_record_batch_efficient(
    rows: &[EfficientRow],
    column_definitions: &[ColumnDefinition],
    schema: Arc<Schema>,
) -> Result<RecordBatch, Box<dyn std::error::Error + Send + Sync>> {
    if rows.is_empty() {
        return Err("No data to convert".into());
    }

    let arrays = create_arrays_from_rows_efficient(rows, column_definitions)?;
    Ok(RecordBatch::try_new(schema, arrays)?)
}

fn create_arrays_from_rows_efficient(
    rows: &[EfficientRow],
    column_definitions: &[ColumnDefinition],
) -> Result<Vec<ArrayRef>, Box<dyn std::error::Error + Send + Sync>> {
    let arrays: Result<Vec<ArrayRef>, Box<dyn std::error::Error + Send + Sync>> =
        column_definitions
            .iter()
            .enumerate()
            .map(
                |(col_idx, col_def)| -> Result<ArrayRef, Box<dyn std::error::Error + Send + Sync>> {
                    let array: ArrayRef = match &col_def.column_type {
                        DataType::String => {
                            let values: Vec<Option<String>> = rows
                                .iter()
                                .map(|row| row.get(col_idx).cloned().flatten())
                                .collect();
                            Arc::new(StringArray::from(values))
                        }
                        DataType::Integer => {
                            let values: Vec<Option<i64>> = rows
                                .iter()
                                .map(|row| {
                                    row.get(col_idx)
                                        .and_then(|v| v.as_ref())
                                        .and_then(|s| s.parse::<i64>().ok())
                                })
                                .collect();
                            Arc::new(Int64Array::from(values))
                        }
                        DataType::Float => {
                            let values: Vec<Option<f64>> = rows
                                .iter()
                                .map(|row| {
                                    row.get(col_idx)
                                        .and_then(|v| v.as_ref())
                                        .and_then(|s| s.parse::<f64>().ok())
                                })
                                .collect();
                            Arc::new(Float64Array::from(values))
                        }
                        DataType::Boolean => {
                            let values: Vec<Option<bool>> = rows
                                .iter()
                                .map(|row| {
                                    row.get(col_idx)
                                        .and_then(|v| v.as_ref())
                                        .and_then(|s| parse_boolean(s))
                                })
                                .collect();
                            Arc::new(BooleanArray::from(values))
                        }
                        DataType::Date => {
                            let values: Vec<Option<i32>> = rows
                                .iter()
                                .map(|row| {
                                    row.get(col_idx)
                                        .and_then(|v| v.as_ref())
                                        .and_then(|s| parse_date_to_days(s))
                                })
                                .collect();
                            Arc::new(Date32Array::from(values))
                        }
                        DataType::DateTime | DataType::Timestamp => {
                            let values: Vec<Option<i64>> = rows
                                .iter()
                                .map(|row| {
                                    row.get(col_idx)
                                        .and_then(|v| v.as_ref())
                                        .and_then(|s| parse_datetime_to_nanos(s))
                                })
                                .collect();
                            Arc::new(TimestampNanosecondArray::from(values))
                        }
                    };

                    Ok(array)
                },
            )
            .collect();

    arrays
}

fn parse_boolean(s: &str) -> Option<bool> {
    match s.to_lowercase().trim() {
        "true" | "1" | "yes" | "y" | "t" => Some(true),
        "false" | "0" | "no" | "n" | "f" => Some(false),
        _ => None,
    }
}

fn parse_date_to_days(s: &str) -> Option<i32> {
    let s = s.trim();

    // Fast path for ISO format (YYYY-MM-DD)
    if s.len() == 10 && s.chars().nth(4) == Some('-') && s.chars().nth(7) == Some('-') {
        if let Ok(parsed) = parse_date_string(s, "%Y-%m-%d") {
            return Some(parsed);
        }
    }

    // Try other formats
    let formats = ["%m/%d/%Y", "%d/%m/%Y", "%Y/%m/%d"];
    for format in &formats {
        if let Ok(parsed) = parse_date_string(s, format) {
            return Some(parsed);
        }
    }
    None
}

fn parse_date_string(date_str: &str, format: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = match format {
        "%Y-%m-%d" => date_str.split('-').collect(),
        "%m/%d/%Y" | "%d/%m/%Y" | "%Y/%m/%d" => date_str.split('/').collect(),
        _ => return Err("Unsupported format".into()),
    };

    if parts.len() != 3 {
        return Err("Invalid date format".into());
    }

    let (year, month, day) = match format {
        "%Y-%m-%d" | "%Y/%m/%d" => (
            parts[0].parse::<i32>()?,
            parts[1].parse::<u32>()?,
            parts[2].parse::<u32>()?,
        ),
        "%m/%d/%Y" => (
            parts[2].parse::<i32>()?,
            parts[0].parse::<u32>()?,
            parts[1].parse::<u32>()?,
        ),
        "%d/%m/%Y" => (
            parts[2].parse::<i32>()?,
            parts[1].parse::<u32>()?,
            parts[0].parse::<u32>()?,
        ),
        _ => return Err("Unsupported format".into()),
    };

    Ok(calculate_days_since_epoch(year, month, day).unwrap_or(0))
}

fn calculate_days_since_epoch(year: i32, month: u32, day: u32) -> Option<i32> {
    let epoch_year = 1970;
    let mut days = 0;

    // Add days for complete years
    for y in epoch_year..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Add days for complete months in the current year
    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += if m == 2 && is_leap_year(year) {
            29
        } else {
            days_in_month[(m - 1) as usize]
        };
    }

    // Add remaining days
    days += (day - 1) as i32;
    Some(days)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn parse_datetime_to_nanos(s: &str) -> Option<i64> {
    let s = s.trim();

    // Fast path for Unix timestamps
    if let Ok(timestamp) = s.parse::<i64>() {
        return if timestamp > 10_000_000_000 {
            Some(timestamp * 1_000_000) // ms to ns
        } else {
            Some(timestamp * 1_000_000_000) // s to ns
        };
    }

    // ISO datetime parsing
    parse_iso_datetime(s)
}

fn parse_iso_datetime(datetime_str: &str) -> Option<i64> {
    let datetime_str = datetime_str.replace('T', " ");
    let parts: Vec<&str> = datetime_str.split(' ').collect();

    if parts.len() != 2 {
        return None;
    }

    let date_part = parts[0];
    let time_part = parts[1].trim_end_matches('Z');

    // Parse date
    let date_parts: Vec<&str> = date_part.split('-').collect();
    if date_parts.len() != 3 {
        return None;
    }

    let year = date_parts[0].parse::<i32>().ok()?;
    let month = date_parts[1].parse::<u32>().ok()?;
    let day = date_parts[2].parse::<u32>().ok()?;

    // Parse time
    let time_parts: Vec<&str> = time_part.split(':').collect();
    if time_parts.len() < 2 {
        return None;
    }

    let hour = time_parts[0].parse::<u32>().ok()?;
    let minute = time_parts[1].parse::<u32>().ok()?;
    let (second, nanos) = if time_parts.len() > 2 {
        let sec_parts: Vec<&str> = time_parts[2].split('.').collect();
        let whole_seconds = sec_parts[0].parse::<u32>().ok()?;
        let nanos = if sec_parts.len() > 1 {
            let frac_str = sec_parts[1];
            let frac_str = if frac_str.len() > 9 {
                &frac_str[..9]
            } else {
                frac_str
            };
            let frac_str = format!("{:0<9}", frac_str);
            frac_str.parse::<u32>().unwrap_or(0)
        } else {
            0
        };
        (whole_seconds, nanos)
    } else {
        (0, 0)
    };

    let days = calculate_days_since_epoch(year, month, day)?;
    let total_seconds =
        days as i64 * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64;
    Some(total_seconds * 1_000_000_000 + nanos as i64)
}

async fn upload_to_s3(
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

async fn update_job_status_to_success(
    table_name: &str,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = aws_config::load_from_env().await;
    let dynamodb_client = DynamoDbClient::new(&config);

    let pk = format!("JOB-{}", job_id);

    println!("Job {}: Updating DynamoDB status to success", job_id);

    let result = dynamodb_client
        .update_item()
        .table_name(table_name)
        .key("service", aws_sdk_dynamodb::types::AttributeValue::S(pk))
        .key(
            "serviceId",
            aws_sdk_dynamodb::types::AttributeValue::S(job_id.to_string()),
        )
        .update_expression("SET #status = :status")
        .expression_attribute_names("#status", "status")
        .expression_attribute_values(
            ":status",
            aws_sdk_dynamodb::types::AttributeValue::S("success".to_string()),
        )
        .send()
        .await;

    match result {
        Ok(_) => {
            println!(
                "Job {}: Successfully updated DynamoDB status to success",
                job_id
            );
            Ok(())
        }
        Err(e) => {
            error!("Job {}: Failed to update DynamoDB status: {}", job_id, e);
            Err(format!("DynamoDB update failed: {}", e).into())
        }
    }
}
