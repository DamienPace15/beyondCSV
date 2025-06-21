use aws_sdk_bedrockruntime::operation::converse::ConverseOutput;
use duckdb::{Connection, Result as DuckResult};
use lambda_runtime::Error;
use serde_json::{Value, json};

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

pub fn execute_sql_query(conn: &Connection, s3_path: &str, sql_query: &str) -> DuckResult<Value> {
    println!("Creating view for parquet file...");

    // First, create a view of the parquet file
    let create_view_query = format!(
        "CREATE OR REPLACE VIEW data AS SELECT * FROM read_parquet('{}')",
        s3_path
    );
    conn.execute(&create_view_query, [])?;

    println!("Executing SQL query: {}", sql_query);

    // Use DuckDB's JSON export functionality instead
    let json_query = format!("SELECT * FROM ({}) AS result", sql_query);

    // Get the result as JSON directly from DuckDB
    let mut stmt = conn.prepare(&json_query)?;
    let mut rows_json = Vec::new();

    let rows = stmt.query_map([], |row| {
        // Get the entire row as a string representation
        let column_count = row.as_ref().column_count();
        let mut row_obj = serde_json::Map::new();

        for i in 0..column_count {
            let col_name = format!("col_{}", i); // Simple column naming
            let value = match row.get_ref(i)? {
                duckdb::types::ValueRef::Null => Value::Null,
                duckdb::types::ValueRef::Boolean(b) => Value::Bool(b),
                duckdb::types::ValueRef::TinyInt(n) => json!(n),
                duckdb::types::ValueRef::SmallInt(n) => json!(n),
                duckdb::types::ValueRef::Int(n) => json!(n),
                duckdb::types::ValueRef::BigInt(n) => json!(n),
                duckdb::types::ValueRef::Float(f) => json!(f),
                duckdb::types::ValueRef::Double(f) => json!(f),
                duckdb::types::ValueRef::Text(s) => {
                    json!(std::str::from_utf8(s).unwrap_or(""))
                }
                _ => json!(null),
            };
            row_obj.insert(col_name, value);
        }
        Ok(json!(row_obj))
    })?;

    for row in rows {
        rows_json.push(row?);
    }

    Ok(json!({
        "data": rows_json
    }))
}
