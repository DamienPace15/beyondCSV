use aws_sdk_s3::Client as S3Client;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;
use tokio::task;
use tracing::error;

use arrow::array::ArrayRef;
use arrow::datatypes::{Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::{EnabledStatistics, WriterProperties};
use std::collections::HashMap;

use crate::creation_parsing::{parse_boolean, parse_date_to_days, parse_datetime_to_nanos};
use crate::creation_types::{ColumnDefinition, DataType};
use crate::s3::upload_to_s3;

// Optimized constants for 2.6GB memory utilization
const ROWS_PER_BATCH: usize = 3_500_000; // 75% larger batches
const S3_CHUNK_SIZE: usize = 512 * 1024 * 1024; // 512MB read buffer
const MAX_BATCH_MEMORY: usize = 1800 * 1024 * 1024; // 1.8GB per batch
const CHANNEL_BUFFER_SIZE: usize = 8; // Fewer but larger batches
const STRING_POOL_SIZE: usize = 50000; // Larger string pool for deduplication
const PARQUET_BUFFER_SIZE: usize = 512 * 1024 * 1024; // 512MB for parquet writing

// Optimized row representation - avoid strings for numeric types
#[derive(Debug, Clone)]
pub enum FieldValue {
    Null,
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Date(i32),
    Timestamp(i64),
}

pub type OptimizedRow = Vec<FieldValue>;

#[derive(Debug)]
struct BatchBuilder {
    rows: Vec<OptimizedRow>,
    estimated_size: usize,
    string_pool: HashMap<String, Arc<String>>,
}

impl BatchBuilder {
    fn new(capacity: usize) -> Self {
        Self {
            rows: Vec::with_capacity(capacity),
            estimated_size: 0,
            string_pool: HashMap::with_capacity(STRING_POOL_SIZE),
        }
    }

    fn add_row(&mut self, row: OptimizedRow) {
        self.estimated_size += estimate_row_size(&row);
        self.rows.push(row);
    }

    fn clear(&mut self) {
        self.rows.clear();
        self.estimated_size = 0;
        // Keep string pool but clear if too large
        if self.string_pool.len() > STRING_POOL_SIZE * 2 {
            self.string_pool.clear();
        }
    }

    fn is_full(&self) -> bool {
        self.rows.len() >= ROWS_PER_BATCH || self.estimated_size >= MAX_BATCH_MEMORY
    }
}

pub async fn stream_csv_to_parquet_optimized(
    bucket: &str,
    key: &str,
    column_definitions: &[ColumnDefinition],
    output_key: &str,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = aws_config::load_from_env().await;
    let s3_client = S3Client::new(&config);

    println!(
        "Job {}: Starting optimized streaming from S3: bucket={}, key={}",
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

    // Create channels
    let (batch_tx, batch_rx) = mpsc::channel::<RecordBatch>(CHANNEL_BUFFER_SIZE);

    // Create shared data
    let column_definitions = Arc::new(column_definitions.to_vec());
    let job_id = Arc::new(job_id.to_string());

    // Create Parquet schema
    let fields: Vec<Field> = column_definitions
        .iter()
        .map(|col| Field::new(&col.column, col.column_type.to_arrow_type(), true))
        .collect();
    let schema = Arc::new(Schema::new(fields));

    // Spawn CSV processor task
    let processor_handle = {
        let s3_client = s3_client.clone();
        let bucket = bucket.to_string();
        let key = key.to_string();
        let column_definitions = column_definitions.clone();
        let schema = schema.clone();
        let job_id = job_id.clone();

        task::spawn(async move {
            if let Err(e) = process_csv_optimized(
                s3_client,
                &bucket,
                &key,
                batch_tx,
                &column_definitions,
                schema,
                &job_id,
            )
            .await
            {
                error!("Job {}: CSV processor failed: {}", job_id, e);
            }
        })
    };

    // Main thread: Parquet writer
    let write_result =
        write_parquet_optimized(batch_rx, bucket, output_key, schema.clone(), &job_id).await;

    // Wait for processor to complete
    processor_handle.await?;

    write_result
}

async fn process_csv_optimized(
    s3_client: S3Client,
    bucket: &str,
    key: &str,
    batch_tx: mpsc::Sender<RecordBatch>,
    column_definitions: &[ColumnDefinition],
    schema: Arc<Schema>,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    let byte_stream = response.body.into_async_read();
    let buf_reader = tokio::io::BufReader::with_capacity(S3_CHUNK_SIZE, byte_stream);

    // Read CSV using tokio's BufReader with manual parsing
    let mut lines = buf_reader.lines();

    // Read headers
    let header_line = match lines.next_line().await? {
        Some(line) => line,
        None => return Err("Empty CSV file".into()),
    };

    let headers = parse_csv_line(&header_line)?;
    let header_map: HashMap<String, usize> = headers
        .iter()
        .enumerate()
        .map(|(idx, h)| (h.trim().to_string(), idx))
        .collect();

    let column_map: HashMap<String, (usize, &ColumnDefinition)> = column_definitions
        .iter()
        .enumerate()
        .map(|(idx, col)| (col.column.clone(), (idx, col)))
        .collect();

    // Process records in batches
    let mut batch_builder = BatchBuilder::new(ROWS_PER_BATCH);
    let mut total_rows = 0;
    let start_time = std::time::Instant::now();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let fields = parse_csv_line(&line)?;

        // Parse row directly into typed values
        let row = parse_row_from_fields(&fields, &header_map, &column_map)?;
        batch_builder.add_row(row);
        total_rows += 1;

        // Send batch when full
        if batch_builder.is_full() {
            let batch = create_record_batch_optimized(
                &batch_builder.rows,
                column_definitions,
                schema.clone(),
            )?;

            if batch_tx.send(batch).await.is_err() {
                break; // Writer dropped
            }

            if total_rows % 100_000 == 0 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let throughput = (total_rows as f64 / elapsed) / 1000.0;
                println!(
                    "Job {}: Processed {} rows, {:.1}K rows/s",
                    job_id, total_rows, throughput
                );
            }

            batch_builder.clear();
        }
    }

    // Send final batch
    if !batch_builder.rows.is_empty() {
        let batch =
            create_record_batch_optimized(&batch_builder.rows, column_definitions, schema.clone())?;
        let _ = batch_tx.send(batch).await;
    }

    let total_time = start_time.elapsed().as_secs_f64();
    println!(
        "Job {}: Finished processing {} rows in {:.2}s, avg: {:.1}K rows/s",
        job_id,
        total_rows,
        total_time,
        (total_rows as f64 / total_time) / 1000.0
    );

    Ok(())
}

// Efficient CSV line parser that avoids creating a full CSV reader per line
fn parse_csv_line(line: &str) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    // Escaped quote
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(field);
                field = String::new();
            }
            _ => field.push(ch),
        }
    }
    fields.push(field);

    Ok(fields)
}

fn parse_row_from_fields(
    fields: &[String],
    header_map: &HashMap<String, usize>,
    column_map: &HashMap<String, (usize, &ColumnDefinition)>,
) -> Result<OptimizedRow, Box<dyn std::error::Error + Send + Sync>> {
    let mut row = vec![FieldValue::Null; column_map.len()];

    for (col_name, &(output_idx, col_def)) in column_map.iter() {
        if let Some(&csv_idx) = header_map.get(col_name) {
            if let Some(field) = fields.get(csv_idx) {
                let value = if field.trim().is_empty() {
                    FieldValue::Null
                } else {
                    parse_field_value(field.trim(), &col_def.column_type)?
                };
                row[output_idx] = value;
            }
        }
    }

    Ok(row)
}

fn parse_field_value(
    field: &str,
    data_type: &DataType,
) -> Result<FieldValue, Box<dyn std::error::Error + Send + Sync>> {
    Ok(match data_type {
        DataType::String => FieldValue::String(field.to_string()),
        DataType::Integer => match field.parse::<i64>() {
            Ok(v) => FieldValue::Integer(v),
            Err(_) => FieldValue::Null,
        },
        DataType::Float => match field.parse::<f64>() {
            Ok(v) => FieldValue::Float(v),
            Err(_) => FieldValue::Null,
        },
        DataType::Boolean => match parse_boolean(field) {
            Some(v) => FieldValue::Boolean(v),
            None => FieldValue::Null,
        },
        DataType::Date => match parse_date_to_days(field) {
            Some(v) => FieldValue::Date(v),
            None => FieldValue::Null,
        },
        DataType::DateTime | DataType::Timestamp => match parse_datetime_to_nanos(field) {
            Some(v) => FieldValue::Timestamp(v),
            None => FieldValue::Null,
        },
    })
}

fn estimate_row_size(row: &OptimizedRow) -> usize {
    row.iter()
        .map(|v| match v {
            FieldValue::Null => 1,
            FieldValue::String(s) => s.len() + 24,
            FieldValue::Integer(_) => 8,
            FieldValue::Float(_) => 8,
            FieldValue::Boolean(_) => 1,
            FieldValue::Date(_) => 4,
            FieldValue::Timestamp(_) => 8,
        })
        .sum()
}

fn create_record_batch_optimized(
    rows: &[OptimizedRow],
    column_definitions: &[ColumnDefinition],
    schema: Arc<Schema>,
) -> Result<RecordBatch, Box<dyn std::error::Error + Send + Sync>> {
    if rows.is_empty() {
        return Err("No data to convert".into());
    }

    let arrays = create_arrays_optimized(rows, column_definitions)?;
    Ok(RecordBatch::try_new(schema, arrays)?)
}

fn create_arrays_optimized(
    rows: &[OptimizedRow],
    column_definitions: &[ColumnDefinition],
) -> Result<Vec<ArrayRef>, Box<dyn std::error::Error + Send + Sync>> {
    column_definitions
        .iter()
        .enumerate()
        .map(|(col_idx, col_def)| {
            let array: ArrayRef = match &col_def.column_type {
                DataType::String => {
                    // Estimate better capacity for string columns
                    let total_chars: usize = rows
                        .iter()
                        .filter_map(|row| match &row[col_idx] {
                            FieldValue::String(s) => Some(s.len()),
                            _ => None,
                        })
                        .sum();

                    let mut builder = arrow::array::StringBuilder::with_capacity(
                        rows.len(),
                        total_chars + rows.len() * 4, // Add some buffer
                    );
                    for row in rows {
                        match &row[col_idx] {
                            FieldValue::String(s) => builder.append_value(s),
                            _ => builder.append_null(),
                        }
                    }
                    Arc::new(builder.finish())
                }
                DataType::Integer => {
                    let mut builder = arrow::array::Int64Builder::with_capacity(rows.len());
                    for row in rows {
                        match &row[col_idx] {
                            FieldValue::Integer(v) => builder.append_value(*v),
                            _ => builder.append_null(),
                        }
                    }
                    Arc::new(builder.finish())
                }
                DataType::Float => {
                    let mut builder = arrow::array::Float64Builder::with_capacity(rows.len());
                    for row in rows {
                        match &row[col_idx] {
                            FieldValue::Float(v) => builder.append_value(*v),
                            _ => builder.append_null(),
                        }
                    }
                    Arc::new(builder.finish())
                }
                DataType::Boolean => {
                    let mut builder = arrow::array::BooleanBuilder::with_capacity(rows.len());
                    for row in rows {
                        match &row[col_idx] {
                            FieldValue::Boolean(v) => builder.append_value(*v),
                            _ => builder.append_null(),
                        }
                    }
                    Arc::new(builder.finish())
                }
                DataType::Date => {
                    let mut builder = arrow::array::Date32Builder::with_capacity(rows.len());
                    for row in rows {
                        match &row[col_idx] {
                            FieldValue::Date(v) => builder.append_value(*v),
                            _ => builder.append_null(),
                        }
                    }
                    Arc::new(builder.finish())
                }
                DataType::DateTime | DataType::Timestamp => {
                    let mut builder =
                        arrow::array::TimestampNanosecondBuilder::with_capacity(rows.len());
                    for row in rows {
                        match &row[col_idx] {
                            FieldValue::Timestamp(v) => builder.append_value(*v),
                            _ => builder.append_null(),
                        }
                    }
                    Arc::new(builder.finish())
                }
            };
            Ok(array)
        })
        .collect()
}

async fn write_parquet_optimized(
    mut batch_rx: mpsc::Receiver<RecordBatch>,
    bucket: &str,
    output_key: &str,
    schema: Arc<Schema>,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = Vec::with_capacity(PARQUET_BUFFER_SIZE); // 512MB initial

    let props = WriterProperties::builder()
        .set_compression(parquet::basic::Compression::SNAPPY)
        .set_write_batch_size(ROWS_PER_BATCH)
        .set_data_page_size_limit(16 * 1024 * 1024) // 16MB pages for larger batches
        .set_dictionary_page_size_limit(16 * 1024 * 1024)
        .set_max_row_group_size(3_500_000) // Match batch size
        .set_column_index_truncate_length(Some(64))
        .set_statistics_enabled(EnabledStatistics::Chunk)
        .build();

    let mut batches_written = 0;
    let start_time = std::time::Instant::now();

    // Create writer in a scope so it's dropped before we use buffer
    {
        let mut writer = ArrowWriter::try_new(&mut buffer, schema, Some(props))?;

        while let Some(batch) = batch_rx.recv().await {
            writer.write(&batch)?;
            batches_written += 1;

            if batches_written % 5 == 0 {
                println!("Job {}: Written {} batches", job_id, batches_written);
            }
        }

        writer.close()?;
    } // writer is dropped here, releasing the mutable borrow on buffer

    println!(
        "Job {}: Writing complete - {} batches, uploading {:.2} MB to S3",
        job_id,
        batches_written,
        buffer.len() as f64 / (1024.0 * 1024.0)
    );

    upload_to_s3(bucket, output_key, buffer, job_id).await?;

    let total_time = start_time.elapsed().as_secs_f64();
    println!(
        "Job {}: Upload completed in {:.2}s total",
        job_id, total_time
    );

    Ok(())
}
