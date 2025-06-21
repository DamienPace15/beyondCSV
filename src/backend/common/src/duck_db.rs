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

    // Set home directory for Lambda environment
    println!("Setting home directory...");
    match conn.execute("SET home_directory='/tmp';", []) {
        Ok(_) => println!("✓ Home directory set to /tmp"),
        Err(e) => {
            println!("✗ Failed to set home directory: {}", e);
            return Err(e);
        }
    }

    // Install extensions one by one with debugging
    println!("Installing parquet extension...");
    match conn.execute("INSTALL parquet;", []) {
        Ok(_) => println!("✓ Parquet extension installed"),
        Err(e) => {
            println!("✗ Failed to install parquet extension: {}", e);
            return Err(e);
        }
    }

    println!("Installing httpfs extension...");
    match conn.execute("INSTALL httpfs;", []) {
        Ok(_) => println!("✓ HTTPFS extension installed"),
        Err(e) => {
            println!("✗ Failed to install httpfs extension: {}", e);
            return Err(e);
        }
    }

    println!("Loading parquet extension...");
    match conn.execute("LOAD parquet;", []) {
        Ok(_) => println!("✓ Parquet extension loaded"),
        Err(e) => {
            println!("✗ Failed to load parquet extension: {}", e);
            return Err(e);
        }
    }

    println!("Loading httpfs extension...");
    match conn.execute("LOAD httpfs;", []) {
        Ok(_) => println!("✓ HTTPFS extension loaded"),
        Err(e) => {
            println!("✗ Failed to load httpfs extension: {}", e);
            return Err(e);
        }
    }

    // Set configuration options one by one
    println!("Setting thread count...");
    match conn.execute("SET threads = 2;", []) {
        Ok(_) => println!("✓ Thread count set to 2"),
        Err(e) => {
            println!("✗ Failed to set thread count: {}", e);
            return Err(e);
        }
    }

    println!("Setting memory limit...");
    match conn.execute("SET memory_limit = '1GB';", []) {
        Ok(_) => println!("✓ Memory limit set to 1GB"),
        Err(e) => {
            println!("✗ Failed to set memory limit: {}", e);
            return Err(e);
        }
    }

    println!("Disabling progress bar...");
    match conn.execute("SET enable_progress_bar = false;", []) {
        Ok(_) => println!("✓ Progress bar disabled"),
        Err(e) => {
            println!("✗ Failed to disable progress bar: {}", e);
            return Err(e);
        }
    }

    println!("Disabling object cache...");
    match conn.execute("SET enable_object_cache = false;", []) {
        Ok(_) => println!("✓ Object cache disabled"),
        Err(e) => {
            println!("✗ Failed to disable object cache: {}", e);
            return Err(e);
        }
    }

    println!("Setting temp directory...");
    match conn.execute("SET temp_directory = '/tmp';", []) {
        Ok(_) => println!("✓ Temp directory set to /tmp"),
        Err(e) => {
            println!("✗ Failed to set temp directory: {}", e);
            return Err(e);
        }
    }

    println!("Setting S3 region...");
    match conn.execute("SET s3_region = 'ap-southeast-2';", []) {
        Ok(_) => println!("✓ S3 region set to ap-southeast-2"),
        Err(e) => {
            println!("✗ Failed to set S3 region: {}", e);
            return Err(e);
        }
    }

    // Test basic functionality
    println!("Testing basic query execution...");
    match conn.execute("SELECT 1 as test;", []) {
        Ok(_) => println!("✓ Basic query test passed"),
        Err(e) => {
            println!("✗ Basic query test failed: {}", e);
            return Err(e);
        }
    }

    // Test parquet functionality (without S3)
    println!("Testing parquet functionality...");
    match conn.execute("SELECT 1 as test_col", []) {
        Ok(_) => println!("✓ Parquet functionality test passed"),
        Err(e) => {
            println!("✗ Parquet functionality test failed: {}", e);
            return Err(e);
        }
    }

    println!("DuckDB connection setup completed successfully!");
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
