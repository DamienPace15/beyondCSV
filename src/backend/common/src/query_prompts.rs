pub const USER_MESSAGE: &str = r#"You are going to be given a schema for a parquet file and a query from a user related to querying that schema.
You will need to make an SQL query from that schema and only return the SQL query and nothing else. No reasoning as to why. Just an SQL query.
I will be using that SQL in a polars sql query in rust.

CRITICAL SQL OPTIMIZATION RULES FOR MINIMUM LATENCY:

**COLUMN NAME HANDLING (CRITICAL - MOST COMMON ERROR SOURCE):**
1. Use EXACT column names from the schema - match case precisely
2. Columns with spaces MUST be enclosed in double quotes: "Sales Revenue", "Product ID"
3. Columns with underscores don't need quotes: Sales_Revenue, Product_ID
4. Single word columns don't need quotes: State, City, Country
5. ALWAYS check if column names contain spaces before writing the query
6. Common column patterns in your schema:
   - "Sales Volume" (needs quotes)
   - "Sales Revenue" (needs quotes)
   - "Product ID" (needs quotes)
   - "Product Category" (needs quotes)
   - State, City, Country, Date (no quotes needed)
7. When referencing columns with spaces in WHERE, GROUP BY, ORDER BY - use quotes there too

**PROJECTION OPTIMIZATION (MOST IMPORTANT):**
1. NEVER use SELECT * - always specify exact columns needed
2. Only select columns that are directly required for the output
3. For COUNT(*) queries without WHERE, this is already optimized - don't add columns
4. For aggregations, only select the grouping columns and aggregated values
5. Minimize column width - if you only need part of a string, consider using SUBSTR()

**PREDICATE PUSHDOWN & FILTERING:**
1. Apply WHERE filters as early as possible - they reduce data scanned
2. For string matches, ALWAYS use LOWER() for case-insensitive search: WHERE LOWER(column) = 'lowercase_value'
3. For range queries, use BETWEEN instead of >= AND <=
4. Place most selective filters first in WHERE clause
5. Use IS NULL/IS NOT NULL efficiently - these are fast operations
6. For user searches, default to case-insensitive matching unless explicitly requested otherwise

**AGGREGATION OPTIMIZATION (POLARS-SPECIFIC RULES):**
1. For existence checks, use LIMIT 1 instead of COUNT(*)
2. When counting distinct values, consider if approximate counts are acceptable
3. Aggregate early - if you need SUM of SUMs, do it in one pass
4. For COUNT(*) with GROUP BY, always alias it (e.g., COUNT(*) as count)
5. Prefer COUNT(*) over COUNT(column) unless checking for non-nulls
6. ALWAYS use COUNT(*) for counting - Polars SQL does NOT support COUNT(1) and will throw "cannot aggregate a literal" error

**LIMIT OPTIMIZATION:**
1. ALWAYS use LIMIT for non-aggregated queries unless specifically asked for all
2. Default limits: 20 for detail records, 50 for summaries, 100 maximum
3. For "show me" or "list" queries without specific count: LIMIT 20
4. For existence queries: LIMIT 1
5. Apply LIMIT after ORDER BY for correct results

**ORDERING OPTIMIZATION:**
1. Only ORDER BY when explicitly requested or needed for LIMIT
2. Avoid ORDER BY on calculated fields - compute in projection if needed
3. For TOP N queries, push LIMIT close to ORDER BY
4. Skip ORDER BY for COUNT(*) queries unless grouped
5. Use column position for ORDER BY when possible: ORDER BY 1, 2

**GENERAL PERFORMANCE RULES:**
1. The table name must always be 'data'
2. Minimize string operations - they're expensive
3. Use single quotes for string literals
4. Return SQL on a single line with no line breaks
5. Avoid DISTINCT unless specifically requested
6. Never use subqueries unless absolutely necessary

**DATA TYPE OPTIMIZATION:**
1. Compare numbers as numbers, not strings
2. Use appropriate operators: =, >, <, >=, <=, BETWEEN
3. For date comparisons, use date functions efficiently
4. Cast only when necessary - casting is expensive

**POLARS SQL COMPATIBILITY RULES:**
1. NEVER use COUNT(1) - use COUNT(*) instead
2. NEVER use COUNT(0) - use COUNT(*) instead
3. Polars SQL is stricter than standard SQL - stick to basic aggregate functions
4. Avoid complex window functions unless absolutely necessary
5. Use standard SQL syntax - avoid database-specific extensions

**EXAMPLE QUERIES WITH PROPER COLUMN QUOTING:**
- "Show me sales data" → SELECT City, State, "Product ID", "Sales Revenue" FROM data LIMIT 20
- "Total revenue by state" → SELECT State, SUM("Sales Revenue") as total FROM data GROUP BY State
- "Top selling products" → SELECT "Product ID", SUM("Sales Volume") as volume FROM data GROUP BY "Product ID" ORDER BY volume DESC LIMIT 10
- "Sales in California" → SELECT "Product Category", "Sales Revenue" FROM data WHERE State = 'California' LIMIT 20
- "Average sales volume" → SELECT AVG("Sales Volume") FROM data
- "Products by category" → SELECT "Product Category", COUNT(*) as cnt FROM data GROUP BY "Product Category"
- "State with most records" → SELECT State, COUNT(*) as cnt FROM data GROUP BY State ORDER BY cnt DESC LIMIT 1

**ANTI-PATTERNS TO AVOID (WILL CAUSE ERRORS):**
- SELECT * FROM data WHERE x = y (use specific columns)
- SELECT Sales Revenue FROM data (missing quotes - should be "Sales Revenue")
- SELECT Product ID FROM data (missing quotes - should be "Product ID")
- WHERE Product Category = 'Electronics' (missing quotes - should be "Product Category")
- GROUP BY Sales Volume (missing quotes - should be "Sales Volume")
- COUNT(1) FROM data (Polars error - use COUNT(*))
- COUNT(0) FROM data (Polars error - use COUNT(*))

Remember: Column names with spaces are the #1 source of SQL errors. Always use double quotes around them! COUNT(1) will fail in Polars - always use COUNT(*).
"#;
