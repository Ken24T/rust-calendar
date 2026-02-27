#![allow(dead_code)]

pub mod mapping;
pub mod fetcher;
pub mod engine;
pub mod scheduler;

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use rusqlite::{params, Connection};

use crate::models::calendar_source::CalendarSource;

pub struct CalendarSourceService<'a> {
    conn: &'a Connection,
}

impl<'a> CalendarSourceService<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create(&self, mut source: CalendarSource) -> Result<CalendarSource> {
        source.validate().map_err(|err| anyhow!(err))?;

        let now = Local::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO calendar_sources (
                    name, source_type, ics_url, enabled, poll_interval_minutes,
                    last_sync_at, last_sync_status, last_error, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    source.name,
                    source.source_type,
                    source.ics_url,
                    source.enabled as i32,
                    source.poll_interval_minutes,
                    source.last_sync_at,
                    source.last_sync_status,
                    source.last_error,
                    now,
                    now,
                ],
            )
            .context("Failed to insert calendar source")?;

        source.id = Some(self.conn.last_insert_rowid());
        Ok(source)
    }

    pub fn list_all(&self) -> Result<Vec<CalendarSource>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, source_type, ics_url, enabled, poll_interval_minutes,
                        last_sync_at, last_sync_status, last_error
                 FROM calendar_sources
                 ORDER BY name COLLATE NOCASE ASC",
            )
            .context("Failed to prepare calendar source list query")?;

        let rows = stmt.query_map([], Self::row_to_source)?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("Failed to load calendar sources")
    }

    pub fn get_by_id(&self, id: i64) -> Result<Option<CalendarSource>> {
        let result = self.conn.query_row(
            "SELECT id, name, source_type, ics_url, enabled, poll_interval_minutes,
                    last_sync_at, last_sync_status, last_error
             FROM calendar_sources
             WHERE id = ?1",
            [id],
            Self::row_to_source,
        );

        match result {
            Ok(source) => Ok(Some(source)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err).context("Failed to fetch calendar source by id"),
        }
    }

    pub fn update(&self, source: &CalendarSource) -> Result<()> {
        source.validate().map_err(|err| anyhow!(err))?;

        let id = source
            .id
            .ok_or_else(|| anyhow!("Calendar source ID is required for update"))?;

        let rows_affected = self.conn.execute(
            "UPDATE calendar_sources
             SET name = ?1,
                 source_type = ?2,
                 ics_url = ?3,
                 enabled = ?4,
                 poll_interval_minutes = ?5,
                 updated_at = ?6
             WHERE id = ?7",
            params![
                source.name,
                source.source_type,
                source.ics_url,
                source.enabled as i32,
                source.poll_interval_minutes,
                Local::now().to_rfc3339(),
                id,
            ],
        )
        .context("Failed to update calendar source")?;

        if rows_affected == 0 {
            return Err(anyhow!("Calendar source with id {} not found", id));
        }

        Ok(())
    }

    pub fn set_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE calendar_sources SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
                params![enabled as i32, Local::now().to_rfc3339(), id],
            )
            .context("Failed to update calendar source enabled state")?;

        if rows_affected == 0 {
            return Err(anyhow!("Calendar source with id {} not found", id));
        }

        Ok(())
    }

    pub fn update_sync_status(
        &self,
        id: i64,
        status: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE calendar_sources
                 SET last_sync_at = ?1,
                     last_sync_status = ?2,
                     last_error = ?3,
                     updated_at = ?4
                 WHERE id = ?5",
                params![
                    Local::now().to_rfc3339(),
                    status,
                    error,
                    Local::now().to_rfc3339(),
                    id,
                ],
            )
            .context("Failed to update calendar source sync status")?;

        if rows_affected == 0 {
            return Err(anyhow!("Calendar source with id {} not found", id));
        }

        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        let rows_affected = self
            .conn
            .execute("DELETE FROM calendar_sources WHERE id = ?1", [id])
            .context("Failed to delete calendar source")?;

        if rows_affected == 0 {
            return Err(anyhow!("Calendar source with id {} not found", id));
        }

        Ok(())
    }

    fn row_to_source(row: &rusqlite::Row<'_>) -> rusqlite::Result<CalendarSource> {
        Ok(CalendarSource {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            source_type: row.get(2)?,
            ics_url: row.get(3)?,
            enabled: row.get::<_, i32>(4)? != 0,
            poll_interval_minutes: row.get(5)?,
            last_sync_at: row.get(6)?,
            last_sync_status: row.get(7)?,
            last_error: row.get(8)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::calendar_source::GOOGLE_ICS_SOURCE_TYPE;
    use crate::services::database::Database;

    fn build_source(name: &str) -> CalendarSource {
        CalendarSource {
            id: None,
            name: name.to_string(),
            source_type: GOOGLE_ICS_SOURCE_TYPE.to_string(),
            ics_url: format!(
                "https://calendar.google.com/calendar/ical/{}%40gmail.com/private-token/basic.ics",
                name.to_lowercase().replace(' ', "")
            ),
            enabled: true,
            poll_interval_minutes: 15,
            last_sync_at: None,
            last_sync_status: None,
            last_error: None,
        }
    }

    #[test]
    fn test_create_and_get_calendar_source() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let service = CalendarSourceService::new(db.connection());

        let created = service.create(build_source("Work")).unwrap();
        assert!(created.id.is_some());

        let fetched = service.get_by_id(created.id.unwrap()).unwrap().unwrap();
        assert_eq!(fetched.name, "Work");
        assert_eq!(fetched.source_type, GOOGLE_ICS_SOURCE_TYPE);
    }

    #[test]
    fn test_list_all_calendar_sources() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let service = CalendarSourceService::new(db.connection());

        service.create(build_source("Personal")).unwrap();
        service.create(build_source("Work")).unwrap();

        let sources = service.list_all().unwrap();
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn test_update_calendar_source() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let service = CalendarSourceService::new(db.connection());

        let created = service.create(build_source("Work")).unwrap();

        let updated = CalendarSource {
            id: created.id,
            name: "Work Updated".to_string(),
            source_type: GOOGLE_ICS_SOURCE_TYPE.to_string(),
            ics_url: "https://calendar.google.com/calendar/ical/work%40gmail.com/private-updated/basic.ics".to_string(),
            enabled: false,
            poll_interval_minutes: 30,
            last_sync_at: None,
            last_sync_status: None,
            last_error: None,
        };

        service.update(&updated).unwrap();
        let fetched = service.get_by_id(created.id.unwrap()).unwrap().unwrap();
        assert_eq!(fetched.name, "Work Updated");
        assert!(!fetched.enabled);
        assert_eq!(fetched.poll_interval_minutes, 30);
    }

    #[test]
    fn test_set_enabled() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let service = CalendarSourceService::new(db.connection());

        let created = service.create(build_source("Work")).unwrap();
        let source_id = created.id.unwrap();

        service.set_enabled(source_id, false).unwrap();
        let fetched = service.get_by_id(source_id).unwrap().unwrap();
        assert!(!fetched.enabled);
    }

    #[test]
    fn test_update_sync_status() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let service = CalendarSourceService::new(db.connection());

        let created = service.create(build_source("Work")).unwrap();
        let source_id = created.id.unwrap();

        service
            .update_sync_status(source_id, Some("success"), None)
            .unwrap();

        let fetched = service.get_by_id(source_id).unwrap().unwrap();
        assert_eq!(fetched.last_sync_status.as_deref(), Some("success"));
        assert!(fetched.last_sync_at.is_some());
    }

    #[test]
    fn test_delete_calendar_source() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let service = CalendarSourceService::new(db.connection());

        let created = service.create(build_source("Work")).unwrap();
        let source_id = created.id.unwrap();

        service.delete(source_id).unwrap();
        assert!(service.get_by_id(source_id).unwrap().is_none());
    }
}
