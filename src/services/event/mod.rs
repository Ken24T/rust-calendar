// Event service module
// CRUD operations for calendar events with database integration

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use rusqlite::Connection;
use crate::models::event::Event;

/// Service for managing calendar events
pub struct EventService<'a> {
    conn: &'a Connection,
}

impl<'a> EventService<'a> {
    /// Create a new EventService with a database connection
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    
    /// Create a new event in the database
    /// 
    /// # Arguments
    /// * `event` - The event to create (id will be ignored and auto-generated)
    /// 
    /// # Returns
    /// Returns the created event with its database-assigned ID
    pub fn create(&self, mut event: Event) -> Result<Event> {
        // Validate before inserting
        event.validate().map_err(|e| anyhow::anyhow!(e))?;
        
        let now = Local::now().to_rfc3339();
        
        // Serialize recurrence exceptions if present
        let exceptions_json = event.recurrence_exceptions.as_ref()
            .map(|excs| {
                let dates: Vec<String> = excs.iter()
                    .map(|dt| dt.to_rfc3339())
                    .collect();
                serde_json::to_string(&dates).unwrap_or_default()
            });
        
        self.conn.execute(
            "INSERT INTO events (
                title, description, location, start_datetime, end_datetime,
                is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                event.title,
                event.description,
                event.location,
                event.start.to_rfc3339(),
                event.end.to_rfc3339(),
                event.all_day as i32,
                event.category,
                event.color,
                event.recurrence_rule,
                exceptions_json,
                &now,
                &now,
            ],
        ).context("Failed to insert event")?;
        
        let id = self.conn.last_insert_rowid();
        event.id = Some(id);
        event.created_at = Some(Local::now());
        event.updated_at = Some(Local::now());
        
        Ok(event)
    }
    
    /// Retrieve an event by ID
    pub fn get(&self, id: i64) -> Result<Option<Event>> {
        let result = self.conn.query_row(
            "SELECT id, title, description, location, start_datetime, end_datetime,
                    is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                    created_at, updated_at
             FROM events WHERE id = ?",
            [id],
            |row| {
                let exceptions_json: Option<String> = row.get(10)?;
                let recurrence_exceptions = exceptions_json.and_then(|json| {
                    serde_json::from_str::<Vec<String>>(&json).ok()
                        .map(|dates| dates.into_iter()
                            .filter_map(|s| DateTime::parse_from_rfc3339(&s).ok()
                                .map(|dt| dt.with_timezone(&Local)))
                            .collect())
                });
                
                Ok(Event {
                    id: Some(row.get(0)?),
                    title: row.get(1)?,
                    description: row.get(2)?,
                    location: row.get(3)?,
                    start: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Local),
                    end: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Local),
                    all_day: row.get::<_, i32>(6)? != 0,
                    category: row.get(7)?,
                    color: row.get(8)?,
                    recurrence_rule: row.get(9)?,
                    recurrence_exceptions,
                    created_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Local)),
                    updated_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Local)),
                })
            },
        );
        
        match result {
            Ok(event) => Ok(Some(event)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    /// Update an existing event
    pub fn update(&self, event: &Event) -> Result<()> {
        let id = event.id.ok_or_else(|| anyhow::anyhow!("Event ID is required for update"))?;
        
        // Validate before updating
        event.validate().map_err(|e| anyhow::anyhow!(e))?;
        
        // Serialize recurrence exceptions if present
        let exceptions_json = event.recurrence_exceptions.as_ref()
            .map(|excs| {
                let dates: Vec<String> = excs.iter()
                    .map(|dt| dt.to_rfc3339())
                    .collect();
                serde_json::to_string(&dates).unwrap_or_default()
            });
        
        let rows_affected = self.conn.execute(
            "UPDATE events SET
                title = ?, description = ?, location = ?, start_datetime = ?, end_datetime = ?,
                is_all_day = ?, category = ?, color = ?, recurrence_rule = ?,
                recurrence_exceptions = ?, updated_at = ?
             WHERE id = ?",
            rusqlite::params![
                event.title,
                event.description,
                event.location,
                event.start.to_rfc3339(),
                event.end.to_rfc3339(),
                event.all_day as i32,
                event.category,
                event.color,
                event.recurrence_rule,
                exceptions_json,
                Local::now().to_rfc3339(),
                id,
            ],
        ).context("Failed to update event")?;
        
        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Event with id {} not found", id));
        }
        
        Ok(())
    }
    
    /// Delete an event by ID
    pub fn delete(&self, id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM events WHERE id = ?",
            [id],
        ).context("Failed to delete event")?;
        
        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Event with id {} not found", id));
        }
        
        Ok(())
    }
    
    /// List all events
    pub fn list_all(&self) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, location, start_datetime, end_datetime,
                    is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                    created_at, updated_at
             FROM events
             ORDER BY start_datetime ASC"
        )?;
        
        let events = stmt.query_map([], |row| {
            let exceptions_json: Option<String> = row.get(10)?;
            let recurrence_exceptions = exceptions_json.and_then(|json| {
                serde_json::from_str::<Vec<String>>(&json).ok()
                    .map(|dates| dates.into_iter()
                        .filter_map(|s| DateTime::parse_from_rfc3339(&s).ok()
                            .map(|dt| dt.with_timezone(&Local)))
                        .collect())
            });
            
            Ok(Event {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                description: row.get(2)?,
                location: row.get(3)?,
                start: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local),
                end: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local),
                all_day: row.get::<_, i32>(6)? != 0,
                category: row.get(7)?,
                color: row.get(8)?,
                recurrence_rule: row.get(9)?,
                recurrence_exceptions,
                created_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local)),
                updated_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local)),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(events)
    }
    
    /// Find events within a date range
    pub fn find_by_date_range(&self, start: DateTime<Local>, end: DateTime<Local>) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, location, start_datetime, end_datetime,
                    is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                    created_at, updated_at
             FROM events
             WHERE start_datetime <= ? AND end_datetime >= ?
             ORDER BY start_datetime ASC"
        )?;
        
        let events = stmt.query_map([end.to_rfc3339(), start.to_rfc3339()], |row| {
            let exceptions_json: Option<String> = row.get(10)?;
            let recurrence_exceptions = exceptions_json.and_then(|json| {
                serde_json::from_str::<Vec<String>>(&json).ok()
                    .map(|dates| dates.into_iter()
                        .filter_map(|s| DateTime::parse_from_rfc3339(&s).ok()
                            .map(|dt| dt.with_timezone(&Local)))
                        .collect())
            });
            
            Ok(Event {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                description: row.get(2)?,
                location: row.get(3)?,
                start: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local),
                end: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local),
                all_day: row.get::<_, i32>(6)? != 0,
                category: row.get(7)?,
                color: row.get(8)?,
                recurrence_rule: row.get(9)?,
                recurrence_exceptions,
                created_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local)),
                updated_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local)),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::Database;
    use chrono::Duration;
    
    fn setup_test_db() -> Database {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        db
    }
    
    fn sample_event() -> Event {
        let start = Local::now();
        let end = start + Duration::hours(1);
        Event::new("Test Event", start, end).unwrap()
    }
    
    #[test]
    fn test_create_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let event = sample_event();
        let result = service.create(event.clone());
        
        assert!(result.is_ok());
        let created = result.unwrap();
        assert!(created.id.is_some());
        assert_eq!(created.title, event.title);
        assert!(created.created_at.is_some());
        assert!(created.updated_at.is_some());
    }
    
    #[test]
    fn test_create_event_with_optional_fields() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let event = Event::builder()
            .title("Conference")
            .description("Annual tech conference")
            .location("Convention Center")
            .start(Local::now())
            .end(Local::now() + Duration::hours(8))
            .category("Work")
            .color("#FF5733")
            .build()
            .unwrap();
        
        let created = service.create(event.clone()).unwrap();
        assert_eq!(created.description, event.description);
        assert_eq!(created.location, event.location);
        assert_eq!(created.category, event.category);
        assert_eq!(created.color, event.color);
    }
    
    #[test]
    fn test_get_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let created = service.create(sample_event()).unwrap();
        let id = created.id.unwrap();
        
        let result = service.get(id);
        assert!(result.is_ok());
        
        let found = result.unwrap();
        assert!(found.is_some());
        
        let event = found.unwrap();
        assert_eq!(event.id, Some(id));
        assert_eq!(event.title, created.title);
    }
    
    #[test]
    fn test_get_nonexistent_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let result = service.get(999);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
    
    #[test]
    fn test_update_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let mut event = service.create(sample_event()).unwrap();
        event.title = "Updated Title".to_string();
        event.description = Some("New description".to_string());
        
        let result = service.update(&event);
        assert!(result.is_ok());
        
        let updated = service.get(event.id.unwrap()).unwrap().unwrap();
        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.description, Some("New description".to_string()));
    }
    
    #[test]
    fn test_update_nonexistent_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let mut event = sample_event();
        event.id = Some(999);
        
        let result = service.update(&event);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_delete_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let created = service.create(sample_event()).unwrap();
        let id = created.id.unwrap();
        
        let result = service.delete(id);
        assert!(result.is_ok());
        
        let found = service.get(id).unwrap();
        assert!(found.is_none());
    }
    
    #[test]
    fn test_delete_nonexistent_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let result = service.delete(999);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_list_all_events() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        service.create(sample_event()).unwrap();
        service.create(sample_event()).unwrap();
        service.create(sample_event()).unwrap();
        
        let events = service.list_all().unwrap();
        assert_eq!(events.len(), 3);
    }
    
    #[test]
    fn test_find_by_date_range() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let now = Local::now();
        
        // Event in the past
        let past_event = Event::new(
            "Past Event",
            now - Duration::days(2),
            now - Duration::days(2) + Duration::hours(1),
        ).unwrap();
        service.create(past_event).unwrap();
        
        // Event in range
        let current_event = Event::new(
            "Current Event",
            now,
            now + Duration::hours(1),
        ).unwrap();
        service.create(current_event).unwrap();
        
        // Event in future
        let future_event = Event::new(
            "Future Event",
            now + Duration::days(2),
            now + Duration::days(2) + Duration::hours(1),
        ).unwrap();
        service.create(future_event).unwrap();
        
        let events = service.find_by_date_range(
            now - Duration::hours(1),
            now + Duration::hours(2),
        ).unwrap();
        
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Current Event");
    }
    
    #[test]
    fn test_create_event_with_recurrence() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let event = Event::builder()
            .title("Weekly Meeting")
            .start(Local::now())
            .end(Local::now() + Duration::hours(1))
            .recurrence_rule("FREQ=WEEKLY;BYDAY=MO")
            .build()
            .unwrap();
        
        let created = service.create(event).unwrap();
        assert_eq!(created.recurrence_rule, Some("FREQ=WEEKLY;BYDAY=MO".to_string()));
        
        let retrieved = service.get(created.id.unwrap()).unwrap().unwrap();
        assert_eq!(retrieved.recurrence_rule, Some("FREQ=WEEKLY;BYDAY=MO".to_string()));
    }
}
