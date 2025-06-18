use aws_sdk_s3::Client as S3Client;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio::task;
use tracing::{error, warn};

use arrow::array::{
    ArrayRef, BooleanArray, Date32Array, Float64Array, Int64Array, StringArray,
    TimestampNanosecondArray,
};
use arrow::datatypes::{Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::collections::HashMap;

use crate::creation_parsing::{parse_boolean, parse_date_to_days, parse_datetime_to_nanos};
use crate::creation_types::{ColumnDefinition, DataType, EfficientRow};
use crate::s3::upload_to_s3;

// Optimized constants for 3GB memory environment (using ~2.5GB)
const ROWS_PER_BATCH: usize = 1_500_000; // 3x increase - larger batches for better throughput
const S3_CHUNK_SIZE: usize = 128 * 1024 * 1024; // 128MB S3 read buffer (4x increase)
const MAX_BATCH_MEMORY: usize = 800 * 1024 * 1024; // 800MB max per batch (4x increase)
const CHANNEL_BUFFER_SIZE: usize = 5000; // 5x increase - more lines buffered between threads
const BATCH_CHANNEL_SIZE: usize = 12; // 3x increase - more batches in pipeline

#[derive(Debug)]
enum ProcessingMessage {
    Line(String),
    Headers(Vec<String>),
    EndOfFile,
}

pub async fn stream_csv_to_parquet_multipart_threaded(
    bucket: &str,
    key: &str,
    column_definitions: &[ColumnDefinition],
    output_key: &str,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = aws_config::load_from_env().await;
    let s3_client = S3Client::new(&config);

    println!(
        "Job {}: Starting multithreaded streaming from S3: bucket={}, key={}",
        job_id, bucket, key
    );

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

    // Create channels for communication between threads
    let (line_tx, line_rx) = mpsc::channel::<ProcessingMessage>(CHANNEL_BUFFER_SIZE);
    let (batch_tx, batch_rx) = mpsc::channel::<RecordBatch>(BATCH_CHANNEL_SIZE);

    // Create shared data structures
    let column_definitions = Arc::new(column_definitions.to_vec());
    let job_id = Arc::new(job_id.to_string());

    // Create Parquet schema
    let fields: Vec<Field> = column_definitions
        .iter()
        .map(|col| Field::new(&col.column, col.column_type.to_arrow_type(), true))
        .collect();
    let schema = Arc::new(Schema::new(fields));

    // Thread 1: CSV Reading (Producer)
    let reader_handle = {
        let s3_client = s3_client.clone();
        let bucket = bucket.to_string();
        let key = key.to_string();
        let job_id = job_id.clone();

        task::spawn(async move {
            if let Err(e) = csv_reader_task(s3_client, &bucket, &key, line_tx, &job_id).await {
                error!("Job {}: CSV reader task failed: {}", job_id, e);
            }
        })
    };

    // Thread 2: Processing (Consumer/Producer)
    let processor_handle = {
        let column_definitions = column_definitions.clone();
        let schema = schema.clone();
        let job_id = job_id.clone();

        task::spawn(async move {
            if let Err(e) =
                csv_processor_task(line_rx, batch_tx, &column_definitions, schema, &job_id).await
            {
                error!("Job {}: CSV processor task failed: {}", job_id, e);
            }
        })
    };

    // Main thread: Parquet Writing (Consumer)
    let write_result =
        parquet_writer_task(batch_rx, bucket, output_key, schema.clone(), &job_id).await;

    // Wait for all threads to complete
    let _ = tokio::try_join!(reader_handle, processor_handle)?;

    write_result
}

async fn csv_reader_task(
    s3_client: S3Client,
    bucket: &str,
    key: &str,
    line_tx: mpsc::Sender<ProcessingMessage>,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    let byte_stream = response.body.into_async_read();
    let mut lines = BufReader::with_capacity(S3_CHUNK_SIZE, byte_stream).lines();

    let mut is_first_line = true;
    let mut lines_read = 0;

    while let Some(line_result) = lines.next_line().await.transpose() {
        let line = match line_result {
            Ok(line) => line,
            Err(e) => {
                error!("Job {}: Error reading line: {}", job_id, e);
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        if is_first_line {
            // Parse and send headers
            match parse_csv_headers(&line) {
                Ok(headers) => {
                    if line_tx
                        .send(ProcessingMessage::Headers(headers))
                        .await
                        .is_err()
                    {
                        break; // Receiver dropped
                    }
                }
                Err(e) => {
                    error!("Job {}: Failed to parse headers: {}", job_id, e);
                    return Err("Invalid CSV headers".into());
                }
            }
            is_first_line = false;
        } else {
            // Send data line
            if line_tx.send(ProcessingMessage::Line(line)).await.is_err() {
                break; // Receiver dropped
            }
            lines_read += 1;

            if lines_read % 100_000 == 0 {
                println!("Job {}: Reader processed {} lines", job_id, lines_read);
            }
        }
    }

    // Signal end of file
    let _ = line_tx.send(ProcessingMessage::EndOfFile).await;
    println!(
        "Job {}: Reader finished, total lines: {}",
        job_id, lines_read
    );
    Ok(())
}

async fn csv_processor_task(
    mut line_rx: mpsc::Receiver<ProcessingMessage>,
    batch_tx: mpsc::Sender<RecordBatch>,
    column_definitions: &[ColumnDefinition],
    schema: Arc<Schema>,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut headers: Option<Vec<String>> = None;
    let mut header_indices: HashMap<String, usize> = HashMap::new();
    let mut column_indices: HashMap<String, usize> = HashMap::new();
    let mut batch_rows = Vec::with_capacity(ROWS_PER_BATCH);
    let mut total_rows = 0;
    let mut estimated_batch_size = 0;
    let start_time = std::time::Instant::now();

    // Create column index mapping
    for (i, col) in column_definitions.iter().enumerate() {
        column_indices.insert(col.column.clone(), i);
    }

    while let Some(message) = line_rx.recv().await {
        match message {
            ProcessingMessage::Headers(parsed_headers) => {
                // Create header index mapping
                for (idx, header) in parsed_headers.iter().enumerate() {
                    header_indices.insert(header.clone(), idx);
                }
                headers = Some(parsed_headers);
                println!(
                    "Job {}: Processor received headers: {} columns",
                    job_id,
                    headers.as_ref().unwrap().len()
                );
            }
            ProcessingMessage::Line(line) => {
                if headers.is_none() {
                    warn!("Job {}: Received data line before headers", job_id);
                    continue;
                }

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

                        // Create batch when full or approaching memory limit
                        if batch_rows.len() >= ROWS_PER_BATCH
                            || estimated_batch_size > MAX_BATCH_MEMORY
                        {
                            let batch_start = std::time::Instant::now();
                            let batch = create_record_batch_efficient(
                                &batch_rows,
                                column_definitions,
                                schema.clone(),
                            )?;

                            if batch_tx.send(batch).await.is_err() {
                                break; // Writer dropped
                            }

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
            ProcessingMessage::EndOfFile => {
                // Process final batch if any
                if !batch_rows.is_empty() {
                    let batch = create_record_batch_efficient(
                        &batch_rows,
                        column_definitions,
                        schema.clone(),
                    )?;
                    if batch_tx.send(batch).await.is_err() {
                        break; // Writer dropped
                    }
                    println!(
                        "Job {}: Processor sent final batch: {} rows",
                        job_id,
                        batch_rows.len()
                    );
                }
                break;
            }
        }
    }

    let total_time = start_time.elapsed().as_secs_f64();
    println!(
        "Job {}: Processor finished - {} rows in {:.2}s, avg: {:.1}K rows/s",
        job_id,
        total_rows,
        total_time,
        (total_rows as f64 / total_time) / 1000.0
    );

    Ok(())
}

async fn parquet_writer_task(
    mut batch_rx: mpsc::Receiver<RecordBatch>,
    bucket: &str,
    output_key: &str,
    schema: Arc<Schema>,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = Vec::with_capacity(100 * 1024 * 1024); // 100MB initial capacity
    let props = WriterProperties::builder()
        .set_compression(parquet::basic::Compression::SNAPPY)
        .set_write_batch_size(ROWS_PER_BATCH)
        .set_data_page_size_limit(4 * 1024 * 1024)
        .set_dictionary_page_size_limit(4 * 1024 * 1024)
        .set_max_row_group_size(1_000_000)
        .build();

    let mut writer = ArrowWriter::try_new(&mut buffer, schema, Some(props))?;
    let mut batches_written = 0;
    let start_time = std::time::Instant::now();

    while let Some(batch) = batch_rx.recv().await {
        writer.write(&batch)?;
        batches_written += 1;

        if batches_written % 10 == 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            println!(
                "Job {}: Writer processed {} batches in {:.2}s",
                job_id, batches_written, elapsed
            );
        }
    }

    writer.close()?;

    println!(
        "Job {}: Writer finished - {} batches, uploading to S3",
        job_id, batches_written
    );

    upload_to_s3(bucket, output_key, buffer, job_id).await?;

    let total_time = start_time.elapsed().as_secs_f64();
    println!(
        "Job {}: Upload completed in {:.2}s total",
        job_id, total_time
    );

    Ok(())
}

// Keep your existing helper functions unchanged
pub fn parse_csv_headers(
    line: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
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

pub fn parse_csv_row_efficient(
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

pub fn estimate_row_size_efficient(row: &EfficientRow) -> usize {
    row.iter()
        .map(|v| v.as_ref().map_or(8, |s| s.len()) + 24)
        .sum()
}

pub fn create_record_batch_efficient(
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

pub fn create_arrays_from_rows_efficient(
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
