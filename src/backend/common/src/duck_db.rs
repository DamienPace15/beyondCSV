use duckdb::{Connection, Result};
use serde_json::{Value, json};

pub fn setup_duckdb_connection() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    println!("Connected to duckdb");
    Ok(conn)
}

/// Retrieves the schema from a local Parquet file.
/// It uses DuckDB's `parquet_schema` function for efficient metadata reading.
pub fn get_schema_from_parquet_file(conn: &Connection, file_path: &str) -> Result<String> {
    // Use DESCRIBE on a SELECT statement for a more reliable schema introspection
    let describe_sql = format!("DESCRIBE SELECT * FROM read_parquet('{}')", file_path);

    let mut stmt = conn.prepare(&describe_sql).map_err(|e| {
        println!("[ERROR] Failed to prepare the DESCRIBE statement: {:?}", e);
        e
    })?;

    let rows = stmt.query_map([], |row| {
        let column_name: String = row.get("column_name")?;
        let column_type: String = row.get("column_type")?;
        let part = format!("{}: {}", column_name, column_type);
        Ok(part)
    }).map_err(|e| {
        println!("[ERROR] Failed to execute query_map for DESCRIBE. This often means the file path is incorrect, the file is not a valid Parquet file, or there are permission issues. Error: {:?}", e);
        e
    })?;

    let mut schema_parts = Vec::new();
    for row_result in rows {
        match row_result {
            Ok(part) => schema_parts.push(part),
            Err(e) => {
                println!(
                    "[ERROR] Failed to process a row from the DESCRIBE query: {:?}",
                    e
                );
                return Err(e);
            }
        }
    }

    // Add a check to ensure we actually got a schema. If not, the file might be invalid.
    if schema_parts.is_empty() {
        println!(
            "[ERROR] The DESCRIBE query returned no rows. The file might be empty or invalid."
        );
        return Err(duckdb::Error::QueryReturnedNoRows);
    }

    let final_schema = schema_parts.join(", ");
    Ok(final_schema)
}

/// Executes a given SQL query against a local Parquet file.
/// DuckDB's `read_parquet` function reads the data from the local path.
pub fn execute_sql_on_parquet_file(
    conn: &Connection,
    file_path: &str,
    sql_query: &str,
) -> Result<Value> {
    // We replace the placeholder 'FROM my_parquet_file' in the Bedrock-generated
    // query with the actual local file path using DuckDB's `read_parquet` function.
    let full_sql = sql_query.replace("my_parquet_file", &format!("read_parquet('{}')", file_path));

    println!("Executing full SQL: {}", full_sql);

    let mut stmt = conn.prepare(&full_sql)?;
    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let mut results: Vec<serde_json::Map<String, Value>> = Vec::new();

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let mut map = serde_json::Map::new();
        for (i, name) in column_names.iter().enumerate() {
            let val: duckdb::types::Value = row.get(i)?;
            // This conversion logic remains the same
            let json_val = match val {
                duckdb::types::Value::Null => Value::Null,
                duckdb::types::Value::Boolean(b) => Value::Bool(b),
                duckdb::types::Value::TinyInt(i) => json!(i),
                duckdb::types::Value::SmallInt(i) => json!(i),
                duckdb::types::Value::Int(i) => json!(i),
                duckdb::types::Value::BigInt(i) => json!(i),
                duckdb::types::Value::Float(f) => json!(f),
                duckdb::types::Value::Double(d) => json!(d),
                duckdb::types::Value::Text(s) => Value::String(s.to_string()),
                _ => Value::String(format!("{:?}", val)),
            };
            map.insert(name.clone(), json_val);
        }
        results.push(map);
    }
    Ok(Value::Array(
        results.into_iter().map(Value::Object).collect(),
    ))
}
