// Database service module
// SQLite database connection and schema management

use anyhow::{Context, Result};
use rusqlite::Connection;

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Create a new database connection
    /// 
    /// # Arguments
    /// * `path` - Path to the SQLite database file (or ":memory:" for in-memory)
    /// 
    /// # Examples
    /// ```
    /// use rust_calendar::services::database::Database;
    /// let db = Database::new(":memory:").unwrap();
    /// ```
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .context(format!("Failed to open database at {}", path))?;
        
        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])
            .context("Failed to enable foreign keys")?;
        
        Ok(Self { conn })
    }
    
    /// Initialize the database schema
    /// Creates all required tables if they don't exist
    pub fn initialize_schema(&self) -> Result<()> {
        // Settings table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                theme TEXT NOT NULL DEFAULT 'light',
                first_day_of_week INTEGER NOT NULL DEFAULT 0,
                time_format TEXT NOT NULL DEFAULT '12h',
                date_format TEXT NOT NULL DEFAULT 'MM/DD/YYYY',
                show_my_day INTEGER NOT NULL DEFAULT 0,
                my_day_position_right INTEGER NOT NULL DEFAULT 0,
                show_ribbon INTEGER NOT NULL DEFAULT 0,
                current_view TEXT NOT NULL DEFAULT 'Month',
                time_slot_interval INTEGER NOT NULL DEFAULT 60,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).context("Failed to create settings table")?;
        
        // Migrate: Add time_slot_interval if it doesn't exist
        let column_exists = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('settings') WHERE name='time_slot_interval'",
            [],
            |row| row.get::<_, i32>(0)
        ).unwrap_or(0) > 0;
        
        if !column_exists {
            self.conn.execute(
                "ALTER TABLE settings ADD COLUMN time_slot_interval INTEGER NOT NULL DEFAULT 60",
                [],
            ).context("Failed to add time_slot_interval column")?;
        }
        
        // Custom themes table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS custom_themes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                is_dark INTEGER NOT NULL DEFAULT 0,
                app_background TEXT NOT NULL,
                calendar_background TEXT NOT NULL,
                weekend_background TEXT NOT NULL,
                today_background TEXT NOT NULL,
                today_border TEXT NOT NULL,
                day_background TEXT NOT NULL,
                day_border TEXT NOT NULL,
                text_primary TEXT NOT NULL,
                text_secondary TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).context("Failed to create custom_themes table")?;
        
        // Insert default settings if not exists
        self.conn.execute(
            "INSERT OR IGNORE INTO settings (id, theme, first_day_of_week, time_format, date_format, time_slot_interval)
             VALUES (1, 'light', 0, '12h', 'MM/DD/YYYY', 60)",
            [],
        ).context("Failed to insert default settings")?;
        
        // Insert default custom themes if not exists
        self.conn.execute(
            "INSERT OR IGNORE INTO custom_themes (name, is_dark, app_background, calendar_background, 
             weekend_background, today_background, today_border, day_background, day_border, 
             text_primary, text_secondary)
             VALUES ('Light', 0, '0.98,0.98,0.98', '1.0,1.0,1.0', '0.96,0.96,0.96', 
                     '0.85,0.92,1.0', '0.3,0.5,0.9', '1.0,1.0,1.0', '0.85,0.85,0.85',
                     '0.1,0.1,0.1', '0.4,0.4,0.4')",
            [],
        ).context("Failed to insert light theme")?;
        
        self.conn.execute(
            "INSERT OR IGNORE INTO custom_themes (name, is_dark, app_background, calendar_background, 
             weekend_background, today_background, today_border, day_background, day_border, 
             text_primary, text_secondary)
             VALUES ('Dark', 1, '0.12,0.12,0.12', '0.15,0.15,0.15', '0.18,0.18,0.18', 
                     '0.2,0.3,0.5', '0.4,0.6,1.0', '0.15,0.15,0.15', '0.3,0.3,0.3',
                     '0.95,0.95,0.95', '0.7,0.7,0.7')",
            [],
        ).context("Failed to insert dark theme")?;
        
        Ok(())
    }
    
    /// Get a reference to the database connection
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    
    #[test]
    fn test_new_database_in_memory() {
        let result = Database::new(":memory:");
        assert!(result.is_ok(), "Should create in-memory database");
    }
    
    #[test]
    fn test_new_database_with_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();
        
        let result = Database::new(db_path_str);
        assert!(result.is_ok(), "Should create file-based database");
        assert!(Path::new(db_path_str).exists(), "Database file should exist");
    }
    
    #[test]
    fn test_initialize_schema() {
        let db = Database::new(":memory:").unwrap();
        let result = db.initialize_schema();
        assert!(result.is_ok(), "Schema initialization should succeed");
    }
    
    #[test]
    fn test_settings_table_exists() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        
        // Check if settings table exists
        let result: Result<i64, rusqlite::Error> = db.connection().query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='settings'",
            [],
            |row| row.get(0)
        );
        
        assert!(result.is_ok(), "Should be able to query sqlite_master");
        assert_eq!(result.unwrap(), 1, "Settings table should exist");
    }
    
    #[test]
    fn test_default_settings_inserted() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        
        // Check if default settings row exists
        let result: Result<i64, rusqlite::Error> = db.connection().query_row(
            "SELECT id FROM settings WHERE id = 1",
            [],
            |row| row.get(0)
        );
        
        assert!(result.is_ok(), "Default settings should be inserted");
        assert_eq!(result.unwrap(), 1, "Settings ID should be 1");
    }
    
    #[test]
    fn test_foreign_keys_enabled() {
        let db = Database::new(":memory:").unwrap();
        
        let result: Result<i64, rusqlite::Error> = db.connection().query_row(
            "PRAGMA foreign_keys",
            [],
            |row| row.get(0)
        );
        
        assert!(result.is_ok(), "Should be able to check foreign_keys");
        assert_eq!(result.unwrap(), 1, "Foreign keys should be enabled");
    }
    
    #[test]
    fn test_settings_table_schema() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        
        // Verify columns exist by querying default row
        let result: Result<(String, i64, String, String), rusqlite::Error> = 
            db.connection().query_row(
                "SELECT theme, first_day_of_week, time_format, date_format FROM settings WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            );
        
        assert!(result.is_ok(), "Should be able to query all columns");
        let (theme, first_day, time_fmt, date_fmt) = result.unwrap();
        assert_eq!(theme, "light");
        assert_eq!(first_day, 0);
        assert_eq!(time_fmt, "12h");
        assert_eq!(date_fmt, "MM/DD/YYYY");
    }
}
