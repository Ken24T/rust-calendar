//! Calendar event service entry point.
//! Provides database-backed operations and recurrence expansion helpers
//! organized across focused submodules.

use rusqlite::Connection;

pub mod crud;
pub mod queries;
pub mod recurrence;
mod shared;

/// Service for managing calendar events stored in SQLite.
pub struct EventService<'a> {
    pub(crate) conn: &'a Connection,
}

impl<'a> EventService<'a> {
    /// Create a new EventService with a database connection
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::event::Event;
    use crate::services::database::Database;
    use chrono::{Duration, Local};

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

        let past_event = Event::new(
            "Past Event",
            now - Duration::days(2),
            now - Duration::days(2) + Duration::hours(1),
        )
        .unwrap();
        service.create(past_event).unwrap();

        let current_event = Event::new("Current Event", now, now + Duration::hours(1)).unwrap();
        service.create(current_event).unwrap();

        let future_event = Event::new(
            "Future Event",
            now + Duration::days(2),
            now + Duration::days(2) + Duration::hours(1),
        )
        .unwrap();
        service.create(future_event).unwrap();

        let events = service
            .find_by_date_range(now - Duration::hours(1), now + Duration::hours(2))
            .unwrap();

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
        assert_eq!(
            created.recurrence_rule,
            Some("FREQ=WEEKLY;BYDAY=MO".to_string())
        );

        let retrieved = service.get(created.id.unwrap()).unwrap().unwrap();
        assert_eq!(
            retrieved.recurrence_rule,
            Some("FREQ=WEEKLY;BYDAY=MO".to_string())
        );
    }
}
