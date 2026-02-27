use anyhow::{Context, Result};
use rusqlite::Connection;

use super::schema;

/// Thin wrapper around the application's SQLite connection.
pub struct Database {
    conn: Connection,
    path: String,
}

impl Database {
    /// Opens (or creates) a SQLite database at the provided path and
    /// enables foreign keys immediately.
    pub fn new(path: &str) -> Result<Self> {
        let conn =
            Connection::open(path).context(format!("Failed to open database at {}", path))?;

        conn.execute("PRAGMA foreign_keys = ON", [])
            .context("Failed to enable foreign keys")?;

        Ok(Self {
            conn,
            path: path.to_string(),
        })
    }

    /// Provides read/write access to the underlying `rusqlite::Connection`.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Returns the source path used to open this database.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Creates tables, runs migrations, and seeds default data.
    pub fn initialize_schema(&self) -> Result<()> {
        schema::initialize_schema(self.connection())
    }
}
