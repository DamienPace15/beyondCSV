use duckdb::{Connection, Result};

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
) -> Result<String> {
    println!("[DEBUG] Entering 'execute_sql_on_parquet_file'");
    println!("[DEBUG]   - Received file_path: '{}'", file_path);
    println!("[DEBUG]   - Received original SQL query: '{}'", sql_query);

    let full_sql = sql_query.replace("data", &format!("read_parquet('{}')", file_path));
    println!("[DEBUG] Executing full transformed SQL: {}", full_sql);

    // DuckDB can output JSON directly!
    let json_sql = format!(
        "SELECT to_json(array_agg(row_to_json(t))) FROM ({}) t",
        full_sql
    );

    let mut stmt = conn.prepare(&json_sql)?;
    let rows = stmt.query_row([], |row| Ok(row.get::<_, String>(0)?))?;

    Ok(rows)
}
