// Event Template service
// CRUD operations for event templates

use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeZone};
use rusqlite::{params, Connection};

use crate::models::template::EventTemplate;

pub struct TemplateService<'a> {
    conn: &'a Connection,
}

impl<'a> TemplateService<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Create a new template
    pub fn create(&self, template: EventTemplate) -> Result<EventTemplate> {
        template.validate().map_err(|e| anyhow::anyhow!(e))?;

        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        self.conn.execute(
            "INSERT INTO event_templates (name, title, description, location, duration_minutes, 
             all_day, category, color, recurrence_rule, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                template.name,
                template.title,
                template.description,
                template.location,
                template.duration_minutes,
                template.all_day as i32,
                template.category,
                template.color,
                template.recurrence_rule,
                now,
            ],
        ).context("Failed to insert template")?;

        let id = self.conn.last_insert_rowid();
        self.get_by_id(id)
    }

    /// Get a template by ID
    pub fn get_by_id(&self, id: i64) -> Result<EventTemplate> {
        let template = self.conn.query_row(
            "SELECT id, name, title, description, location, duration_minutes, all_day,
             category, color, recurrence_rule, created_at
             FROM event_templates WHERE id = ?1",
            params![id],
            |row| {
                Ok(EventTemplate {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    location: row.get(4)?,
                    duration_minutes: row.get(5)?,
                    all_day: row.get::<_, i32>(6)? != 0,
                    category: row.get(7)?,
                    color: row.get(8)?,
                    recurrence_rule: row.get(9)?,
                    created_at: parse_datetime(row.get::<_, Option<String>>(10)?),
                })
            },
        ).context("Template not found")?;

        Ok(template)
    }

    /// Get all templates ordered by name
    pub fn list_all(&self) -> Result<Vec<EventTemplate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, title, description, location, duration_minutes, all_day,
             category, color, recurrence_rule, created_at
             FROM event_templates ORDER BY name ASC",
        )?;

        let templates = stmt.query_map([], |row| {
            Ok(EventTemplate {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                location: row.get(4)?,
                duration_minutes: row.get(5)?,
                all_day: row.get::<_, i32>(6)? != 0,
                category: row.get(7)?,
                color: row.get(8)?,
                recurrence_rule: row.get(9)?,
                created_at: parse_datetime(row.get::<_, Option<String>>(10)?),
            })
        })?;

        templates.collect::<Result<Vec<_>, _>>().context("Failed to fetch templates")
    }

    /// Update an existing template
    pub fn update(&self, template: &EventTemplate) -> Result<()> {
        template.validate().map_err(|e| anyhow::anyhow!(e))?;

        let id = template.id.ok_or_else(|| anyhow::anyhow!("Template ID is required for update"))?;

        self.conn.execute(
            "UPDATE event_templates SET 
             name = ?1, title = ?2, description = ?3, location = ?4, 
             duration_minutes = ?5, all_day = ?6, category = ?7, color = ?8, recurrence_rule = ?9
             WHERE id = ?10",
            params![
                template.name,
                template.title,
                template.description,
                template.location,
                template.duration_minutes,
                template.all_day as i32,
                template.category,
                template.color,
                template.recurrence_rule,
                id,
            ],
        ).context("Failed to update template")?;

        Ok(())
    }

    /// Delete a template by ID
    pub fn delete(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM event_templates WHERE id = ?1",
            params![id],
        ).context("Failed to delete template")?;

        Ok(())
    }

    /// Check if a template name already exists (excluding a specific ID)
    pub fn name_exists(&self, name: &str, exclude_id: Option<i64>) -> Result<bool> {
        let count: i32 = if let Some(id) = exclude_id {
            self.conn.query_row(
                "SELECT COUNT(*) FROM event_templates WHERE name = ?1 AND id != ?2",
                params![name, id],
                |row| row.get(0),
            )?
        } else {
            self.conn.query_row(
                "SELECT COUNT(*) FROM event_templates WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )?
        };

        Ok(count > 0)
    }
}

fn parse_datetime(s: Option<String>) -> Option<DateTime<Local>> {
    s.and_then(|s| {
        chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
            .ok()
            .and_then(|naive| Local.from_local_datetime(&naive).single())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE event_templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                description TEXT,
                location TEXT,
                duration_minutes INTEGER NOT NULL,
                all_day INTEGER NOT NULL DEFAULT 0,
                category TEXT,
                color TEXT,
                recurrence_rule TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).unwrap();
        conn
    }

    #[test]
    fn test_create_and_get() {
        let conn = setup_test_db();
        let service = TemplateService::new(&conn);

        let template = EventTemplate::new("Meeting", "Team Standup", 30);
        let created = service.create(template).unwrap();

        assert!(created.id.is_some());
        assert_eq!(created.name, "Meeting");
        assert_eq!(created.title, "Team Standup");
        assert_eq!(created.duration_minutes, 30);

        let fetched = service.get_by_id(created.id.unwrap()).unwrap();
        assert_eq!(fetched.name, "Meeting");
    }

    #[test]
    fn test_list_all() {
        let conn = setup_test_db();
        let service = TemplateService::new(&conn);

        service.create(EventTemplate::new("Zebra", "Z Event", 60)).unwrap();
        service.create(EventTemplate::new("Apple", "A Event", 30)).unwrap();

        let templates = service.list_all().unwrap();
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].name, "Apple"); // Ordered by name
        assert_eq!(templates[1].name, "Zebra");
    }

    #[test]
    fn test_update() {
        let conn = setup_test_db();
        let service = TemplateService::new(&conn);

        let template = service.create(EventTemplate::new("Original", "Title", 60)).unwrap();
        let mut updated = template;
        updated.name = "Updated".to_string();
        updated.duration_minutes = 90;

        service.update(&updated).unwrap();

        let fetched = service.get_by_id(updated.id.unwrap()).unwrap();
        assert_eq!(fetched.name, "Updated");
        assert_eq!(fetched.duration_minutes, 90);
    }

    #[test]
    fn test_delete() {
        let conn = setup_test_db();
        let service = TemplateService::new(&conn);

        let template = service.create(EventTemplate::new("ToDelete", "Title", 60)).unwrap();
        let id = template.id.unwrap();

        service.delete(id).unwrap();

        assert!(service.get_by_id(id).is_err());
    }

    #[test]
    fn test_name_exists() {
        let conn = setup_test_db();
        let service = TemplateService::new(&conn);

        let template = service.create(EventTemplate::new("Unique", "Title", 60)).unwrap();

        assert!(service.name_exists("Unique", None).unwrap());
        assert!(!service.name_exists("Other", None).unwrap());
        assert!(!service.name_exists("Unique", template.id).unwrap()); // Exclude self
    }
}
