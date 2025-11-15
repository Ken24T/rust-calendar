use anyhow::{Context, Result};
use rusqlite::Connection;

/// Checks whether a column exists on a table.
pub fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let query = format!(
        "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name='{}'",
        table, column
    );

    let exists: i32 = conn
        .query_row(&query, [], |row| row.get(0))
        .context("Failed to inspect table info")?;

    Ok(exists > 0)
}

/// Adds a column if it does not already exist.
pub fn ensure_column(conn: &Connection, table: &str, column: &str, ddl: &str) -> Result<()> {
    if column_exists(conn, table, column)? {
        return Ok(());
    }

    conn.execute(ddl, [])
        .with_context(|| format!("Failed to add {}.{}", table, column))?;
    Ok(())
}

/// Copies data from an old column into a new destination column.
pub fn copy_column(conn: &Connection, table: &str, from: &str, to: &str) -> Result<()> {
    let stmt = format!(
        "UPDATE {table} SET {to} = {from} WHERE {from} IS NOT NULL",
        table = table,
        from = from,
        to = to
    );
    conn.execute(&stmt, []).with_context(|| {
        format!(
            "Failed to copy {from} to {to} on {table}",
            from = from,
            to = to
        )
    })?;
    Ok(())
}
