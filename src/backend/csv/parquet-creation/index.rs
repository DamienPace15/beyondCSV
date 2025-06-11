use aws_lambda_events::{
    apigw::{ApiGatewayV2httpRequest, ApiGatewayV2httpResponse},
    encodings::Body,
    http::HeaderMap,
};
use aws_sdk_s3::Client as S3Client;
use lambda_runtime::{Error, LambdaEvent, service_fn};
use std::env;
use tracing::{error, info};

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
    #[serde(rename = "s3Key")]
    s3_key: String,
}

// Dynamic row structure to hold parsed CSV data
type DynamicRow = HashMap<String, Option<String>>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let handler = service_fn(handler);
    lambda_runtime::run(handler).await?;

    Ok(())
}

async fn handler(
    event: LambdaEvent<ApiGatewayV2httpRequest>,
) -> Result<ApiGatewayV2httpResponse, Error> {
    let body = event.payload.body.unwrap_or_default();
    let bucket_name = env::var("S3_UPLOAD_BUCKET_NAME")?;

    let request: ParquetCreationRequest = serde_json::from_str(&body)
        .map_err(|e| lambda_runtime::Error::from(format!("Failed to parse JSON: {}", e)))?;

    info!("Processing request with {} columns", request.payload.len());
    info!("S3 key: {}", request.s3_key);

    let csv_content = get_csv_from_s3(&bucket_name, &request.s3_key).await?;

    // Parse CSV and convert to Parquet
    let start_time = std::time::Instant::now();

    let rows = parse_csv_dynamic(&csv_content, &request.payload)?;
    info!("Parsed {} rows from CSV", rows.len());

    let parquet_data = convert_to_parquet(&rows, &request.payload)?;
    info!(
        "Converted to Parquet in {:.2} seconds",
        start_time.elapsed().as_secs_f64()
    );

    upload_to_s3(&bucket_name, "parquet/random.parquet", parquet_data).await?;

    // Here you would typically upload the parquet_data back to S3
    // upload_parquet_to_s3(&bucket_name, &parquet_key, parquet_data).await?;

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    Ok(ApiGatewayV2httpResponse {
        status_code: 200,
        headers,
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(format!(
            "{{\"message\": \"Successfully converted {} rows to Parquet\", \"processing_time\": {:.2}}}",
            rows.len(),
            start_time.elapsed().as_secs_f64()
        ))),
        is_base64_encoded: false,
        cookies: vec![],
    })
}

async fn get_csv_from_s3(
    bucket: &str,
    key: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let config = aws_config::load_from_env().await;
    let s3_client = S3Client::new(&config);

    info!("Fetching CSV from S3: bucket={}, key={}", bucket, key);

    let response = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    let data = response.body.collect().await?;
    let csv_content = String::from_utf8(data.into_bytes().to_vec())?;

    info!(
        "Successfully retrieved CSV file, size: {} bytes",
        csv_content.len()
    );

    Ok(csv_content)
}

async fn upload_to_s3(bucket: &str, key: &str, parquet_data: Vec<u8>) -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let s3_client = S3Client::new(&config);

    info!("Uploading parquet to S3: bucket={}, key={}", bucket, key);

    s3_client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(parquet_data.into()) // Convert Vec<u8> to ByteStream
        .content_type("application/octet-stream") // Set appropriate content type
        .send()
        .await?;

    info!("Successfully uploaded parquet file");

    Ok(())
}

fn parse_csv_dynamic(
    csv_content: &str,
    _column_definitions: &[ColumnDefinition],
) -> Result<Vec<DynamicRow>, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());
    let mut rows = Vec::new();
    let mut row_count = 0;

    // Get headers first, before iterating over records
    let headers = reader.headers()?.clone();

    for result in reader.records() {
        row_count += 1;

        if row_count % 50_000 == 0 {
            info!("Processed {} rows...", row_count);
        }

        match result {
            Ok(record) => {
                let mut row = HashMap::new();

                for (i, field) in record.iter().enumerate() {
                    if let Some(header) = headers.get(i) {
                        let value = if field.trim().is_empty() {
                            None
                        } else {
                            Some(field.to_string())
                        };
                        row.insert(header.to_string(), value);
                    }
                }

                rows.push(row);
            }
            Err(e) => {
                error!("Error parsing row {}: {}", row_count, e);
                continue;
            }
        }
    }

    Ok(rows)
}

fn convert_to_parquet(
    rows: &[DynamicRow],
    column_definitions: &[ColumnDefinition],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    if rows.is_empty() {
        return Err("No data to convert".into());
    }

    // Create schema dynamically from column definitions
    let fields: Vec<Field> = column_definitions
        .iter()
        .map(|col| Field::new(&col.column, col.column_type.to_arrow_type(), true))
        .collect();

    let schema = Arc::new(Schema::new(fields));

    // Convert data to Arrow arrays
    let arrays = create_arrays_from_rows(rows, column_definitions)?;

    // Create RecordBatch
    let batch = RecordBatch::try_new(schema.clone(), arrays)?;

    // Write to Parquet
    let mut buffer = Vec::new();
    {
        let props = WriterProperties::builder().build();
        let mut writer = ArrowWriter::try_new(&mut buffer, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;
    }

    Ok(buffer)
}

fn create_arrays_from_rows(
    rows: &[DynamicRow],
    column_definitions: &[ColumnDefinition],
) -> Result<Vec<ArrayRef>, Box<dyn std::error::Error + Send + Sync>> {
    let mut arrays: Vec<ArrayRef> = Vec::new();

    for col_def in column_definitions {
        let column_name = &col_def.column;

        match &col_def.column_type {
            DataType::String => {
                let values: Vec<Option<String>> = rows
                    .iter()
                    .map(|row| row.get(column_name).cloned().flatten())
                    .collect();
                arrays.push(Arc::new(StringArray::from(values)));
            }
            DataType::Integer => {
                let values: Vec<Option<i64>> = rows
                    .iter()
                    .map(|row| {
                        row.get(column_name)
                            .and_then(|v| v.as_ref())
                            .and_then(|s| s.parse::<i64>().ok())
                    })
                    .collect();
                arrays.push(Arc::new(Int64Array::from(values)));
            }
            DataType::Float => {
                let values: Vec<Option<f64>> = rows
                    .iter()
                    .map(|row| {
                        row.get(column_name)
                            .and_then(|v| v.as_ref())
                            .and_then(|s| s.parse::<f64>().ok())
                    })
                    .collect();
                arrays.push(Arc::new(Float64Array::from(values)));
            }
            DataType::Boolean => {
                let values: Vec<Option<bool>> = rows
                    .iter()
                    .map(|row| {
                        row.get(column_name)
                            .and_then(|v| v.as_ref())
                            .and_then(|s| parse_boolean(s))
                    })
                    .collect();
                arrays.push(Arc::new(BooleanArray::from(values)));
            }
            DataType::Date => {
                let values: Vec<Option<i32>> = rows
                    .iter()
                    .map(|row| {
                        row.get(column_name)
                            .and_then(|v| v.as_ref())
                            .and_then(|s| parse_date_to_days(s))
                    })
                    .collect();
                arrays.push(Arc::new(Date32Array::from(values)));
            }
            DataType::DateTime | DataType::Timestamp => {
                let values: Vec<Option<i64>> = rows
                    .iter()
                    .map(|row| {
                        row.get(column_name)
                            .and_then(|v| v.as_ref())
                            .and_then(|s| parse_datetime_to_nanos(s))
                    })
                    .collect();
                arrays.push(Arc::new(TimestampNanosecondArray::from(values)));
            }
        }
    }

    Ok(arrays)
}

fn parse_boolean(s: &str) -> Option<bool> {
    match s.to_lowercase().trim() {
        "true" | "1" | "yes" | "y" | "t" => Some(true),
        "false" | "0" | "no" | "n" | "f" => Some(false),
        _ => None,
    }
}

fn parse_date_to_days(s: &str) -> Option<i32> {
    // Try different date formats
    let formats = ["%Y-%m-%d", "%m/%d/%Y", "%d/%m/%Y", "%Y/%m/%d"];

    for format in &formats {
        // Simple date parsing - convert to days since Unix epoch
        if let Ok(parsed) = parse_date_string(s.trim(), format) {
            return Some(parsed);
        }
    }
    None
}

fn parse_date_string(date_str: &str, format: &str) -> Result<i32, Box<dyn std::error::Error>> {
    // Basic date parsing logic
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

    // Calculate days since Unix epoch (1970-01-01)
    // This is a simplified calculation
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

    Ok(days)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn parse_datetime_to_nanos(s: &str) -> Option<i64> {
    // Try parsing as Unix timestamp first (seconds or milliseconds)
    if let Ok(timestamp) = s.trim().parse::<i64>() {
        // Assume milliseconds if > 1e10, otherwise seconds
        if timestamp > 10_000_000_000 {
            return Some(timestamp * 1_000_000); // Convert ms to ns
        } else {
            return Some(timestamp * 1_000_000_000); // Convert s to ns
        }
    }

    // Try basic ISO 8601 format: YYYY-MM-DD HH:MM:SS
    if let Some(nanos) = parse_iso_datetime(s.trim()) {
        return Some(nanos);
    }

    None
}

fn parse_iso_datetime(datetime_str: &str) -> Option<i64> {
    // Handle basic ISO format: "YYYY-MM-DD HH:MM:SS" or "YYYY-MM-DDTHH:MM:SS"
    let datetime_str = datetime_str.replace('T', " ");
    let parts: Vec<&str> = datetime_str.split(' ').collect();

    if parts.len() != 2 {
        return None;
    }

    let date_part = parts[0];
    let time_part = parts[1].trim_end_matches('Z'); // Remove Z if present

    // Parse date part
    let date_parts: Vec<&str> = date_part.split('-').collect();
    if date_parts.len() != 3 {
        return None;
    }

    let year = date_parts[0].parse::<i32>().ok()?;
    let month = date_parts[1].parse::<u32>().ok()?;
    let day = date_parts[2].parse::<u32>().ok()?;

    // Parse time part
    let time_parts: Vec<&str> = time_part.split(':').collect();
    if time_parts.len() < 2 {
        return None;
    }

    let hour = time_parts[0].parse::<u32>().ok()?;
    let minute = time_parts[1].parse::<u32>().ok()?;
    let second = if time_parts.len() > 2 {
        // Handle fractional seconds
        let sec_parts: Vec<&str> = time_parts[2].split('.').collect();
        let whole_seconds = sec_parts[0].parse::<u32>().ok()?;
        let nanos = if sec_parts.len() > 1 {
            // Parse fractional part and convert to nanoseconds
            let frac_str = sec_parts[1];
            let frac_str = if frac_str.len() > 9 {
                &frac_str[..9] // Truncate to nanoseconds
            } else {
                frac_str
            };
            let frac_str = format!("{:0<9}", frac_str); // Pad with zeros
            frac_str.parse::<u32>().unwrap_or(0)
        } else {
            0
        };
        (whole_seconds, nanos)
    } else {
        (0, 0)
    };

    // Calculate total nanoseconds since Unix epoch
    let days = calculate_days_since_epoch(year, month, day)?;
    let total_seconds =
        days as i64 * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second.0 as i64;
    let total_nanos = total_seconds * 1_000_000_000 + second.1 as i64;

    Some(total_nanos)
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
