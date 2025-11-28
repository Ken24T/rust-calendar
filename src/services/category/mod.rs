//! Category service for CRUD operations on event categories.
//!
//! This service provides methods to create, read, update, and delete categories,
//! as well as initialize default categories on first run.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::models::category::{default_categories, Category};

/// Service for managing event categories.
pub struct CategoryService<'a> {
    conn: &'a Connection,
}

impl<'a> CategoryService<'a> {
    /// Create a new CategoryService with the given database connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Initialize the categories table and populate with defaults if empty.
    pub fn initialize_defaults(&self) -> Result<()> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM categories",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        if count == 0 {
            log::info!("Initializing default categories");
            for category in default_categories() {
                if let Err(e) = self.create(category) {
                    log::warn!("Failed to create default category: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Create a new category.
    pub fn create(&self, category: Category) -> Result<Category> {
        category.validate().map_err(|e| anyhow::anyhow!("{}", e))?;

        self.conn.execute(
            "INSERT INTO categories (name, color, icon, is_system)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                category.name.trim(),
                category.color,
                category.icon,
                category.is_system as i32,
            ],
        ).context("Failed to insert category")?;

        let id = self.conn.last_insert_rowid();
        self.get_by_id(id)
    }

    /// Get a category by ID.
    pub fn get_by_id(&self, id: i64) -> Result<Category> {
        let category = self.conn.query_row(
            "SELECT id, name, color, icon, is_system FROM categories WHERE id = ?1",
            params![id],
            |row| {
                Ok(Category {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    color: row.get(2)?,
                    icon: row.get(3)?,
                    is_system: row.get::<_, i32>(4)? != 0,
                })
            },
        ).context("Category not found")?;

        Ok(category)
    }

    /// Get a category by name.
    pub fn get_by_name(&self, name: &str) -> Result<Option<Category>> {
        let result = self.conn.query_row(
            "SELECT id, name, color, icon, is_system FROM categories WHERE name = ?1",
            params![name],
            |row| {
                Ok(Category {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    color: row.get(2)?,
                    icon: row.get(3)?,
                    is_system: row.get::<_, i32>(4)? != 0,
                })
            },
        );

        match result {
            Ok(cat) => Ok(Some(cat)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all categories ordered by name.
    pub fn list_all(&self) -> Result<Vec<Category>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, color, icon, is_system 
             FROM categories 
             ORDER BY is_system DESC, name ASC",
        )?;

        let categories = stmt.query_map([], |row| {
            Ok(Category {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                color: row.get(2)?,
                icon: row.get(3)?,
                is_system: row.get::<_, i32>(4)? != 0,
            })
        })?;

        categories.collect::<Result<Vec<_>, _>>().context("Failed to fetch categories")
    }

    /// Update an existing category.
    /// System categories can only have their color and icon updated, not their name.
    pub fn update(&self, category: &Category) -> Result<()> {
        category.validate().map_err(|e| anyhow::anyhow!("{}", e))?;

        let id = category.id.ok_or_else(|| anyhow::anyhow!("Category ID is required for update"))?;

        // Check if this is a system category
        let existing = self.get_by_id(id)?;
        
        if existing.is_system {
            // System categories can only update color and icon
            self.conn.execute(
                "UPDATE categories SET color = ?1, icon = ?2 WHERE id = ?3",
                params![category.color, category.icon, id],
            ).context("Failed to update system category")?;
        } else {
            // User categories can update everything
            self.conn.execute(
                "UPDATE categories SET name = ?1, color = ?2, icon = ?3 WHERE id = ?4",
                params![category.name.trim(), category.color, category.icon, id],
            ).context("Failed to update category")?;
        }

        Ok(())
    }

    /// Delete a category by ID.
    /// System categories cannot be deleted.
    pub fn delete(&self, id: i64) -> Result<()> {
        // Check if it's a system category
        let category = self.get_by_id(id)?;
        if category.is_system {
            return Err(anyhow::anyhow!("Cannot delete system category '{}'", category.name));
        }

        // Update events that use this category to have no category
        self.conn.execute(
            "UPDATE events SET category = NULL WHERE category = ?1",
            params![category.name],
        ).context("Failed to clear category from events")?;

        // Delete the category
        self.conn.execute(
            "DELETE FROM categories WHERE id = ?1",
            params![id],
        ).context("Failed to delete category")?;

        Ok(())
    }

    /// Check if a category name already exists (excluding a specific ID).
    pub fn name_exists(&self, name: &str, exclude_id: Option<i64>) -> Result<bool> {
        let count: i32 = if let Some(id) = exclude_id {
            self.conn.query_row(
                "SELECT COUNT(*) FROM categories WHERE LOWER(name) = LOWER(?1) AND id != ?2",
                params![name.trim(), id],
                |row| row.get(0),
            )?
        } else {
            self.conn.query_row(
                "SELECT COUNT(*) FROM categories WHERE LOWER(name) = LOWER(?1)",
                params![name.trim()],
                |row| row.get(0),
            )?
        };

        Ok(count > 0)
    }

    /// Get the number of events using a specific category.
    pub fn get_usage_count(&self, category_name: &str) -> Result<i32> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM events WHERE category = ?1",
            params![category_name],
            |row| row.get(0),
        )?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        
        // Create categories table
        conn.execute(
            "CREATE TABLE categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                color TEXT NOT NULL,
                icon TEXT,
                is_system INTEGER NOT NULL DEFAULT 0
            )",
            [],
        ).unwrap();

        // Create events table for usage count tests
        conn.execute(
            "CREATE TABLE events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                category TEXT
            )",
            [],
        ).unwrap();

        conn
    }

    #[test]
    fn test_create_and_get() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        let category = Category::new("Test", "#FF0000");
        let created = service.create(category).unwrap();

        assert!(created.id.is_some());
        assert_eq!(created.name, "Test");
        assert_eq!(created.color, "#FF0000");
        assert!(!created.is_system);

        let fetched = service.get_by_id(created.id.unwrap()).unwrap();
        assert_eq!(fetched.name, "Test");
    }

    #[test]
    fn test_create_with_icon() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        let category = Category::with_icon("Work", "#3B82F6", "💼");
        let created = service.create(category).unwrap();

        assert_eq!(created.icon, Some("💼".to_string()));
    }

    #[test]
    fn test_get_by_name() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        service.create(Category::new("FindMe", "#FF0000")).unwrap();

        let found = service.get_by_name("FindMe").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "FindMe");

        let not_found = service.get_by_name("NotHere").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_list_all() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        service.create(Category::new("Zebra", "#FF0000")).unwrap();
        service.create(Category::new("Apple", "#00FF00")).unwrap();
        service.create(Category::system("System", "#0000FF", "⭐")).unwrap();

        let categories = service.list_all().unwrap();
        assert_eq!(categories.len(), 3);
        // System categories come first, then sorted by name
        assert_eq!(categories[0].name, "System");
        assert_eq!(categories[1].name, "Apple");
        assert_eq!(categories[2].name, "Zebra");
    }

    #[test]
    fn test_update_user_category() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        let mut category = service.create(Category::new("Original", "#FF0000")).unwrap();
        category.name = "Updated".to_string();
        category.color = "#00FF00".to_string();

        service.update(&category).unwrap();

        let fetched = service.get_by_id(category.id.unwrap()).unwrap();
        assert_eq!(fetched.name, "Updated");
        assert_eq!(fetched.color, "#00FF00");
    }

    #[test]
    fn test_update_system_category_preserves_name() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        let mut category = service.create(Category::system("System", "#FF0000", "⭐")).unwrap();
        category.name = "Hacked".to_string(); // Try to change name
        category.color = "#00FF00".to_string();

        service.update(&category).unwrap();

        let fetched = service.get_by_id(category.id.unwrap()).unwrap();
        assert_eq!(fetched.name, "System"); // Name unchanged
        assert_eq!(fetched.color, "#00FF00"); // Color updated
    }

    #[test]
    fn test_delete_user_category() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        let category = service.create(Category::new("ToDelete", "#FF0000")).unwrap();
        let id = category.id.unwrap();

        service.delete(id).unwrap();

        assert!(service.get_by_id(id).is_err());
    }

    #[test]
    fn test_delete_system_category_fails() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        let category = service.create(Category::system("Protected", "#FF0000", "🔒")).unwrap();
        let id = category.id.unwrap();

        let result = service.delete(id);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot delete system category"));
    }

    #[test]
    fn test_name_exists() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        let category = service.create(Category::new("Unique", "#FF0000")).unwrap();

        assert!(service.name_exists("Unique", None).unwrap());
        assert!(service.name_exists("unique", None).unwrap()); // Case insensitive
        assert!(!service.name_exists("Other", None).unwrap());
        assert!(!service.name_exists("Unique", category.id).unwrap()); // Exclude self
    }

    #[test]
    fn test_initialize_defaults() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        service.initialize_defaults().unwrap();

        let categories = service.list_all().unwrap();
        assert_eq!(categories.len(), 6); // 6 default categories

        // Running again should not create duplicates
        service.initialize_defaults().unwrap();
        let categories = service.list_all().unwrap();
        assert_eq!(categories.len(), 6);
    }

    #[test]
    fn test_delete_clears_events() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        // Create a category and some events using it
        let category = service.create(Category::new("ToDelete", "#FF0000")).unwrap();
        conn.execute(
            "INSERT INTO events (title, category) VALUES ('Event1', 'ToDelete')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO events (title, category) VALUES ('Event2', 'ToDelete')",
            [],
        ).unwrap();

        // Delete the category
        service.delete(category.id.unwrap()).unwrap();

        // Check events have null category
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM events WHERE category IS NOT NULL",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_usage_count() {
        let conn = setup_test_db();
        let service = CategoryService::new(&conn);

        service.create(Category::new("Used", "#FF0000")).unwrap();
        
        conn.execute("INSERT INTO events (title, category) VALUES ('E1', 'Used')", []).unwrap();
        conn.execute("INSERT INTO events (title, category) VALUES ('E2', 'Used')", []).unwrap();
        conn.execute("INSERT INTO events (title, category) VALUES ('E3', 'Other')", []).unwrap();

        assert_eq!(service.get_usage_count("Used").unwrap(), 2);
        assert_eq!(service.get_usage_count("Other").unwrap(), 1);
        assert_eq!(service.get_usage_count("Unused").unwrap(), 0);
    }
}
