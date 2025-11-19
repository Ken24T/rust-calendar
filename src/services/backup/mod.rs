use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use std::fs;
use std::path::{Path, PathBuf};

/// Information about a backup file
#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub filename: String,
    pub created_at: DateTime<Local>,
    pub size_bytes: u64,
}

impl BackupInfo {
    /// Parse backup info from a file path
    fn from_path(path: PathBuf) -> Result<Self> {
        let metadata = fs::metadata(&path)
            .with_context(|| format!("Failed to get metadata for {:?}", path))?;

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let created_at = metadata
            .created()
            .or_else(|_| metadata.modified())
            .with_context(|| format!("Failed to get creation time for {:?}", path))?;

        let created_at = DateTime::<Local>::from(created_at);
        let size_bytes = metadata.len();

        Ok(BackupInfo {
            path,
            filename,
            created_at,
            size_bytes,
        })
    }
}

/// Service for managing database backups
pub struct BackupService;

impl BackupService {
    /// Get the default backup directory in AppData
    pub fn default_backup_dir() -> Result<PathBuf> {
        let app_data = directories::BaseDirs::new()
            .context("Failed to get base directories")?
            .data_dir()
            .to_path_buf();
        
        let backup_dir = app_data.join("rust-calendar").join("backups");
        
        // Create directory if it doesn't exist
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir)
                .with_context(|| format!("Failed to create backup directory: {:?}", backup_dir))?;
        }
        
        Ok(backup_dir)
    }

    /// Create a backup of the database file
    /// 
    /// # Arguments
    /// * `db_path` - Path to the database file to backup
    /// * `backup_dir` - Directory to store the backup (uses default if None)
    /// 
    /// # Returns
    /// Path to the created backup file
    pub fn create_backup(db_path: &Path, backup_dir: Option<&Path>) -> Result<PathBuf> {
        // Ensure database file exists
        if !db_path.exists() {
            anyhow::bail!("Database file does not exist: {:?}", db_path);
        }

        // Determine backup directory
        let backup_dir = if let Some(dir) = backup_dir {
            dir.to_path_buf()
        } else {
            Self::default_backup_dir()?
        };

        // Ensure backup directory exists
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir)
                .with_context(|| format!("Failed to create backup directory: {:?}", backup_dir))?;
        }

        // Generate backup filename with timestamp
        let timestamp = Local::now().format("%Y-%m-%d_%H%M%S");
        let backup_filename = format!("calendar_backup_{}.db", timestamp);
        let backup_path = backup_dir.join(&backup_filename);

        // Copy database file to backup location
        fs::copy(db_path, &backup_path)
            .with_context(|| format!("Failed to copy database from {:?} to {:?}", db_path, backup_path))?;

        log::info!("Created backup: {:?}", backup_path);
        Ok(backup_path)
    }

    /// Restore a database from a backup file
    /// 
    /// # Arguments
    /// * `backup_path` - Path to the backup file to restore from
    /// * `db_path` - Path where the database should be restored
    /// 
    /// # Safety
    /// This will overwrite the existing database file. The caller should ensure
    /// the database is not in use and ideally create a backup before restoring.
    pub fn restore_backup(backup_path: &Path, db_path: &Path) -> Result<()> {
        // Ensure backup file exists
        if !backup_path.exists() {
            anyhow::bail!("Backup file does not exist: {:?}", backup_path);
        }

        // Verify backup file is a valid SQLite database
        Self::verify_sqlite_file(backup_path)?;

        // Create backup of current database before overwriting (safety measure)
        if db_path.exists() {
            let safety_backup = db_path.with_extension("db.before_restore");
            fs::copy(db_path, &safety_backup)
                .context("Failed to create safety backup before restore")?;
            log::info!("Created safety backup: {:?}", safety_backup);
        }

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create database directory: {:?}", parent))?;
        }

        // Copy backup file to database location
        fs::copy(backup_path, db_path)
            .with_context(|| format!("Failed to restore backup from {:?} to {:?}", backup_path, db_path))?;

        log::info!("Restored backup from {:?} to {:?}", backup_path, db_path);
        Ok(())
    }

    /// List all backups in the specified directory
    /// 
    /// # Arguments
    /// * `backup_dir` - Directory to search for backups (uses default if None)
    /// 
    /// # Returns
    /// Vector of BackupInfo sorted by creation date (newest first)
    pub fn list_backups(backup_dir: Option<&Path>) -> Result<Vec<BackupInfo>> {
        let backup_dir = if let Some(dir) = backup_dir {
            dir.to_path_buf()
        } else {
            Self::default_backup_dir()?
        };

        if !backup_dir.exists() {
            return Ok(Vec::new());
        }

        let mut backups = Vec::new();

        // Read directory entries
        let entries = fs::read_dir(&backup_dir)
            .with_context(|| format!("Failed to read backup directory: {:?}", backup_dir))?;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            // Only include .db files
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("db") {
                if let Ok(info) = BackupInfo::from_path(path) {
                    backups.push(info);
                }
            }
        }

        // Sort by creation date, newest first
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Delete old backups, keeping only the specified number of most recent backups
    /// 
    /// # Arguments
    /// * `backup_dir` - Directory containing backups (uses default if None)
    /// * `keep_count` - Number of recent backups to keep
    pub fn cleanup_old_backups(backup_dir: Option<&Path>, keep_count: usize) -> Result<usize> {
        let mut backups = Self::list_backups(backup_dir)?;

        if backups.len() <= keep_count {
            return Ok(0);
        }

        // Remove oldest backups
        let to_remove = backups.split_off(keep_count);
        let removed_count = to_remove.len();

        for backup in to_remove {
            if let Err(e) = fs::remove_file(&backup.path) {
                log::warn!("Failed to delete old backup {:?}: {}", backup.path, e);
            } else {
                log::info!("Deleted old backup: {:?}", backup.path);
            }
        }

        Ok(removed_count)
    }

    /// Create a backup on startup and clean up old backups
    /// 
    /// # Arguments
    /// * `db_path` - Path to the database file
    /// * `keep_count` - Number of backups to keep (default: 5)
    /// 
    /// # Returns
    /// Path to the created backup file, or None if backup creation was skipped
    pub fn auto_backup_on_startup(db_path: &Path, keep_count: Option<usize>) -> Result<Option<PathBuf>> {
        let keep_count = keep_count.unwrap_or(5);

        // Only create backup if database exists
        if !db_path.exists() {
            log::info!("Skipping auto-backup: database file does not exist yet");
            return Ok(None);
        }

        // Create backup
        let backup_path = Self::create_backup(db_path, None)?;

        // Clean up old backups
        let removed = Self::cleanup_old_backups(None, keep_count)?;
        if removed > 0 {
            log::info!("Cleaned up {} old backup(s)", removed);
        }

        Ok(Some(backup_path))
    }

    /// Delete a specific backup file
    pub fn delete_backup(backup_path: &Path) -> Result<()> {
        if !backup_path.exists() {
            anyhow::bail!("Backup file does not exist: {:?}", backup_path);
        }

        fs::remove_file(backup_path)
            .with_context(|| format!("Failed to delete backup: {:?}", backup_path))?;

        log::info!("Deleted backup: {:?}", backup_path);
        Ok(())
    }

    /// Verify that a file is a valid SQLite database
    fn verify_sqlite_file(path: &Path) -> Result<()> {
        let file = fs::File::open(path)
            .with_context(|| format!("Failed to open file: {:?}", path))?;

        // SQLite files start with "SQLite format 3\0" (16 bytes)
        use std::io::Read;
        let mut header = [0u8; 16];
        let mut reader = std::io::BufReader::new(file);
        reader.read_exact(&mut header)
            .context("Failed to read file header")?;

        let sqlite_header = b"SQLite format 3\0";
        if &header != sqlite_header {
            anyhow::bail!("File is not a valid SQLite database");
        }

        Ok(())
    }

    /// Get human-readable size string from bytes
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn create_test_database(path: &Path) -> Result<()> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", [])?;
        Ok(())
    }

    #[test]
    fn test_create_backup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backup_dir = temp_dir.path().join("backups");

        // Create test database
        create_test_database(&db_path).unwrap();

        // Create backup
        let backup_path = BackupService::create_backup(&db_path, Some(&backup_dir)).unwrap();

        // Verify backup exists
        assert!(backup_path.exists());
        assert!(backup_path.starts_with(&backup_dir));
        assert_eq!(backup_path.extension().unwrap(), "db");
    }

    #[test]
    fn test_create_backup_nonexistent_db() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("nonexistent.db");
        let backup_dir = temp_dir.path().join("backups");

        // Should fail with nonexistent database
        let result = BackupService::create_backup(&db_path, Some(&backup_dir));
        assert!(result.is_err());
    }

    #[test]
    fn test_restore_backup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let original_db = temp_dir.path().join("original.db");
        let backup_dir = temp_dir.path().join("backups");
        let restored_db = temp_dir.path().join("restored.db");

        // Create and populate original database
        {
            let conn = rusqlite::Connection::open(&original_db).unwrap();
            conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", []).unwrap();
            conn.execute("INSERT INTO test (value) VALUES (?)", ["test_value"]).unwrap();
        }

        // Create backup
        let backup_path = BackupService::create_backup(&original_db, Some(&backup_dir)).unwrap();

        // Restore to new location
        BackupService::restore_backup(&backup_path, &restored_db).unwrap();

        // Verify restored database
        assert!(restored_db.exists());

        let conn = rusqlite::Connection::open(&restored_db).unwrap();
        let value: String = conn.query_row("SELECT value FROM test", [], |row| row.get(0)).unwrap();
        assert_eq!(value, "test_value");
    }

    #[test]
    fn test_list_backups() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_database(&db_path).unwrap();

        // Create multiple backups with longer delays to ensure different timestamps
        BackupService::create_backup(&db_path, Some(&backup_dir)).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        BackupService::create_backup(&db_path, Some(&backup_dir)).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        BackupService::create_backup(&db_path, Some(&backup_dir)).unwrap();

        // List backups
        let backups = BackupService::list_backups(Some(&backup_dir)).unwrap();
        assert_eq!(backups.len(), 3);

        // Verify sorted by date (newest first)
        for i in 0..backups.len() - 1 {
            assert!(backups[i].created_at >= backups[i + 1].created_at);
        }
    }

    #[test]
    fn test_cleanup_old_backups() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_database(&db_path).unwrap();

        // Create 5 backups with distinct timestamps
        for _ in 0..5 {
            BackupService::create_backup(&db_path, Some(&backup_dir)).unwrap();
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        // Clean up, keeping only 2
        let removed = BackupService::cleanup_old_backups(Some(&backup_dir), 2).unwrap();
        assert_eq!(removed, 3);

        // Verify only 2 remain
        let backups = BackupService::list_backups(Some(&backup_dir)).unwrap();
        assert_eq!(backups.len(), 2);
    }

    #[test]
    fn test_verify_sqlite_file() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Valid SQLite file
        let valid_db = temp_dir.path().join("valid.db");
        create_test_database(&valid_db).unwrap();
        assert!(BackupService::verify_sqlite_file(&valid_db).is_ok());

        // Invalid file (not SQLite)
        let invalid_file = temp_dir.path().join("invalid.txt");
        let mut file = File::create(&invalid_file).unwrap();
        file.write_all(b"This is not a SQLite file").unwrap();
        assert!(BackupService::verify_sqlite_file(&invalid_file).is_err());
    }

    #[test]
    fn test_format_size() {
        assert_eq!(BackupService::format_size(500), "500 bytes");
        assert_eq!(BackupService::format_size(1024), "1.00 KB");
        assert_eq!(BackupService::format_size(1024 * 1024), "1.00 MB");
        assert_eq!(BackupService::format_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(BackupService::format_size(1536), "1.50 KB");
    }

    #[test]
    fn test_delete_backup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_database(&db_path).unwrap();

        // Create backup
        let backup_path = BackupService::create_backup(&db_path, Some(&backup_dir)).unwrap();
        assert!(backup_path.exists());

        // Delete backup
        BackupService::delete_backup(&backup_path).unwrap();
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_auto_backup_on_startup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        create_test_database(&db_path).unwrap();

        // Set custom backup directory for testing
        let backup_dir = temp_dir.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        // Create multiple backups to test cleanup with distinct timestamps
        for _ in 0..7 {
            BackupService::create_backup(&db_path, Some(&backup_dir)).unwrap();
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        // Verify all 7 backups exist
        let backups = BackupService::list_backups(Some(&backup_dir)).unwrap();
        assert_eq!(backups.len(), 7);

        // Clean up to 3
        BackupService::cleanup_old_backups(Some(&backup_dir), 3).unwrap();
        let backups = BackupService::list_backups(Some(&backup_dir)).unwrap();
        assert_eq!(backups.len(), 3);
    }
}
