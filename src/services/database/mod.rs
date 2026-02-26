mod connection;
mod migrations;
mod schema;

pub use connection::Database;

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
        assert!(
            Path::new(db_path_str).exists(),
            "Database file should exist"
        );
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
            |row| row.get(0),
        );

        assert!(result.is_ok(), "Should be able to query sqlite_master");
        assert_eq!(result.unwrap(), 1, "Settings table should exist");
    }

    #[test]
    fn test_default_settings_inserted() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();

        // Check if default settings row exists
        let result: Result<i64, rusqlite::Error> =
            db.connection()
                .query_row("SELECT id FROM settings WHERE id = 1", [], |row| row.get(0));

        assert!(result.is_ok(), "Default settings should be inserted");
        assert_eq!(result.unwrap(), 1, "Settings ID should be 1");
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let db = Database::new(":memory:").unwrap();

        let result: Result<i64, rusqlite::Error> =
            db.connection()
                .query_row("PRAGMA foreign_keys", [], |row| row.get(0));

        assert!(result.is_ok(), "Should be able to check foreign_keys");
        assert_eq!(result.unwrap(), 1, "Foreign keys should be enabled");
    }

    #[test]
    fn test_settings_table_schema() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();

        // Verify columns exist by querying default row
        let result: Result<(String, i64, String, String), rusqlite::Error> = db
            .connection()
            .query_row(
            "SELECT theme, first_day_of_week, time_format, date_format FROM settings WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        );

        assert!(result.is_ok(), "Should be able to query all columns");
        let (theme, first_day, time_fmt, date_fmt) = result.unwrap();
        assert_eq!(theme, "light");
        assert_eq!(first_day, 0);
        assert_eq!(time_fmt, "12h");
        assert_eq!(date_fmt, "MM/DD/YYYY");
    }

    #[test]
    fn test_events_table_exists() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();

        // Check if events table exists
        let result: Result<i64, rusqlite::Error> = db.connection().query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='events'",
            [],
            |row| row.get(0),
        );

        assert!(result.is_ok(), "Should be able to query sqlite_master");
        assert_eq!(result.unwrap(), 1, "Events table should exist");
    }

    #[test]
    fn test_events_table_schema() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();

        // Verify we can insert and query an event
        let result = db.connection().execute(
            "INSERT INTO events (title, start_datetime, end_datetime, is_all_day)
             VALUES (?, ?, ?, ?)",
            [
                "Test Event",
                "2025-11-07T10:00:00Z",
                "2025-11-07T11:00:00Z",
                "0",
            ],
        );

        assert!(result.is_ok(), "Should be able to insert an event");

        // Query the event back
        let event_result: Result<(i64, String, String, String, i64), rusqlite::Error> =
            db.connection().query_row(
                "SELECT id, title, start_datetime, end_datetime, is_all_day FROM events WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            );

        assert!(event_result.is_ok(), "Should be able to query the event");
        let (id, title, start, end, all_day) = event_result.unwrap();
        assert_eq!(id, 1);
        assert_eq!(title, "Test Event");
        assert_eq!(start, "2025-11-07T10:00:00Z");
        assert_eq!(end, "2025-11-07T11:00:00Z");
        assert_eq!(all_day, 0);
    }

    #[test]
    fn test_calendar_sources_table_exists() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();

        let result: Result<i64, rusqlite::Error> = db.connection().query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='calendar_sources'",
            [],
            |row| row.get(0),
        );

        assert!(result.is_ok(), "Should be able to query sqlite_master");
        assert_eq!(result.unwrap(), 1, "calendar_sources table should exist");
    }
}
