use duckdb::{Connection, Result as DuckResult};

/* pub fn setup_duckdb_connection() -> DuckResult<Connection> {
    let conn = Connection::open_in_memory()?;
    println!("passing connection?");

    // Set home directory for Lambda environment
    conn.execute("SET home_directory='/tmp';", [])?;

    // Install and configure DuckDB with S3 support
    conn.execute_batch(
        r#"
        INSTALL parquet;
        INSTALL httpfs;
        LOAD parquet;
        LOAD httpfs;
        SET threads = 2;
        SET memory_limit = '1GB';
        SET enable_progress_bar = false;
        SET enable_object_cache = false;
        SET temp_directory = '/tmp';
        SET s3_region = 'ap-southeast-2';
        "#,
    )?;

    println!("duckdb connected");

    Ok(conn)
}
 */
pub fn setup_duckdb_connection() -> DuckResult<Connection> {
    println!("Starting DuckDB connection setup...");

    let conn = Connection::open_in_memory()?;
    println!("✓ In-memory connection created successfully");

    // Try auto-install approach first
    println!("Enabling auto-install and auto-load...");
    match conn.execute("SET autoinstall_known_extensions = true;", []) {
        Ok(_) => println!("✓ Auto-install enabled"),
        Err(e) => println!("⚠ Auto-install not available: {}", e),
    }

    match conn.execute("SET autoload_known_extensions = true;", []) {
        Ok(_) => println!("✓ Auto-load enabled"),
        Err(e) => println!("⚠ Auto-load not available: {}", e),
    }

    // Basic S3 configuration
    println!("Setting S3 region...");
    conn.execute("SET s3_region = 'ap-southeast-2';", [])?;
    println!("✓ S3 region set");

    println!("🎉 DuckDB connection setup completed!");
    Ok(conn)
}

pub fn get_schema_from_parquet(conn: &Connection, s3_path: &str) -> DuckResult<String> {
    println!("Getting schema from: {}", s3_path);

    let query = format!("DESCRIBE SELECT * FROM read_parquet('{}') LIMIT 0", s3_path);
    println!("Executing query: {}", query);

    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map([], |row| {
        let column_name: String = row.get(0)?;
        let column_type: String = row.get(1)?;
        Ok(format!("  {}: {}", column_name, column_type))
    })?;

    let schema_lines: Result<Vec<String>, _> = rows.collect();
    let schema = schema_lines?.join("\n");

    println!(
        "Schema successfully retrieved: {} columns",
        schema.lines().count()
    );
    Ok(schema)
}
