#![allow(dead_code)]

pub mod engine;
pub mod fetcher;
mod google_api;
pub mod mapping;
mod sanitizer;
pub mod scheduler;

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use rusqlite::{params, Connection};

use crate::models::calendar_source::CalendarSource;

pub struct CalendarSourceService<'a> {
    conn: &'a Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncRunDiagnostics {
    pub source_id: i64,
    pub started_at: String,
    pub finished_at: String,
    pub status: String,
    pub duration_ms: i64,
    pub created_count: i64,
    pub updated_count: i64,
    pub deleted_count: i64,
    pub unchanged_count: i64,
    pub skipped_count: i64,
    pub error_count: i64,
    pub error_message: Option<String>,
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
                    sync_past_days, sync_future_days,
                    sync_capability, api_sync_token, last_push_at,
                    last_sync_at, last_sync_status, last_error, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    source.name,
                    source.source_type,
                    source.ics_url,
                    source.enabled as i32,
                    source.poll_interval_minutes,
                    source.sync_past_days,
                    source.sync_future_days,
                    source.sync_capability,
                    source.api_sync_token,
                    source.last_push_at,
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
                    sync_past_days, sync_future_days,
                    sync_capability, api_sync_token, last_push_at,
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
                    sync_past_days, sync_future_days,
                    sync_capability, api_sync_token, last_push_at,
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

        let rows_affected = self
            .conn
            .execute(
                "UPDATE calendar_sources
             SET name = ?1,
                 source_type = ?2,
                 ics_url = ?3,
                 enabled = ?4,
                 poll_interval_minutes = ?5,
                 sync_past_days = ?6,
                 sync_future_days = ?7,
                 sync_capability = ?8,
                 api_sync_token = ?9,
                 last_push_at = ?10,
                 updated_at = ?11
             WHERE id = ?12",
                params![
                    source.name,
                    source.source_type,
                    source.ics_url,
                    source.enabled as i32,
                    source.poll_interval_minutes,
                    source.sync_past_days,
                    source.sync_future_days,
                    source.sync_capability,
                    source.api_sync_token,
                    source.last_push_at,
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

    pub fn set_sync_capability(&self, id: i64, capability: &str) -> Result<()> {
        if capability != crate::models::calendar_source::SYNC_CAPABILITY_READ_ONLY
            && capability != crate::models::calendar_source::SYNC_CAPABILITY_READ_WRITE
        {
            return Err(anyhow!(
                "Invalid sync capability '{}': expected read_only or read_write",
                capability
            ));
        }

        let rows_affected = self
            .conn
            .execute(
                "UPDATE calendar_sources
                 SET sync_capability = ?1,
                     updated_at = ?2
                 WHERE id = ?3",
                params![capability, Local::now().to_rfc3339(), id],
            )
            .context("Failed to update calendar source sync capability")?;

        if rows_affected == 0 {
            return Err(anyhow!("Calendar source with id {} not found", id));
        }

        Ok(())
    }

    pub fn set_api_sync_token(&self, id: i64, sync_token: Option<&str>) -> Result<()> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE calendar_sources
                 SET api_sync_token = ?1,
                     updated_at = ?2
                 WHERE id = ?3",
                params![sync_token, Local::now().to_rfc3339(), id],
            )
            .context("Failed to update calendar source API sync token")?;

        if rows_affected == 0 {
            return Err(anyhow!("Calendar source with id {} not found", id));
        }

        Ok(())
    }

    pub fn mark_last_push_now(&self, id: i64) -> Result<()> {
        let now = Local::now().to_rfc3339();
        let rows_affected = self
            .conn
            .execute(
                "UPDATE calendar_sources
                 SET last_push_at = ?1,
                     updated_at = ?2
                 WHERE id = ?3",
                params![now, Local::now().to_rfc3339(), id],
            )
            .context("Failed to update calendar source last_push_at")?;

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
        self.update_sync_status_with_diagnostics(id, status, error, None)
    }

    pub fn update_sync_status_with_diagnostics(
        &self,
        id: i64,
        status: Option<&str>,
        error: Option<&str>,
        diagnostics: Option<&SyncRunDiagnostics>,
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

        if let Some(diag) = diagnostics {
            self.record_sync_run(diag)?;
        }

        Ok(())
    }

    pub fn record_sync_run(&self, diagnostics: &SyncRunDiagnostics) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO calendar_sync_runs (
                    source_id, started_at, finished_at, status, duration_ms,
                    created_count, updated_count, deleted_count, unchanged_count,
                    skipped_count, error_count, error_message
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    diagnostics.source_id,
                    diagnostics.started_at,
                    diagnostics.finished_at,
                    diagnostics.status,
                    diagnostics.duration_ms,
                    diagnostics.created_count,
                    diagnostics.updated_count,
                    diagnostics.deleted_count,
                    diagnostics.unchanged_count,
                    diagnostics.skipped_count,
                    diagnostics.error_count,
                    diagnostics.error_message,
                ],
            )
            .context("Failed to insert calendar sync diagnostics run")?;

        Ok(())
    }

    pub fn latest_sync_run(&self, source_id: i64) -> Result<Option<SyncRunDiagnostics>> {
        let result = self.conn.query_row(
            "SELECT source_id, started_at, finished_at, status, duration_ms,
                    created_count, updated_count, deleted_count, unchanged_count,
                    skipped_count, error_count, error_message
             FROM calendar_sync_runs
             WHERE source_id = ?1
             ORDER BY id DESC
             LIMIT 1",
            [source_id],
            Self::row_to_sync_run,
        );

        match result {
            Ok(run) => Ok(Some(run)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err).context("Failed to fetch latest sync run"),
        }
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
            sync_past_days: row.get(6)?,
            sync_future_days: row.get(7)?,
            sync_capability: row.get(8)?,
            api_sync_token: row.get(9)?,
            last_push_at: row.get(10)?,
            last_sync_at: row.get(11)?,
            last_sync_status: row.get(12)?,
            last_error: row.get(13)?,
        })
    }

    fn row_to_sync_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncRunDiagnostics> {
        Ok(SyncRunDiagnostics {
            source_id: row.get(0)?,
            started_at: row.get(1)?,
            finished_at: row.get(2)?,
            status: row.get(3)?,
            duration_ms: row.get(4)?,
            created_count: row.get(5)?,
            updated_count: row.get(6)?,
            deleted_count: row.get(7)?,
            unchanged_count: row.get(8)?,
            skipped_count: row.get(9)?,
            error_count: row.get(10)?,
            error_message: row.get(11)?,
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
            sync_past_days: 90,
            sync_future_days: 365,
            sync_capability: crate::models::calendar_source::SYNC_CAPABILITY_READ_ONLY.to_string(),
            api_sync_token: None,
            last_push_at: None,
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
            sync_past_days: 30,
            sync_future_days: 180,
            sync_capability: crate::models::calendar_source::SYNC_CAPABILITY_READ_ONLY.to_string(),
            api_sync_token: None,
            last_push_at: None,
            last_sync_at: None,
            last_sync_status: None,
            last_error: None,
        };

        service.update(&updated).unwrap();
        let fetched = service.get_by_id(created.id.unwrap()).unwrap().unwrap();
        assert_eq!(fetched.name, "Work Updated");
        assert!(!fetched.enabled);
        assert_eq!(fetched.poll_interval_minutes, 30);
        assert_eq!(fetched.sync_past_days, 30);
        assert_eq!(fetched.sync_future_days, 180);
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
    fn test_source_metadata_helpers() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let service = CalendarSourceService::new(db.connection());

        let created = service.create(build_source("Meta")).unwrap();
        let source_id = created.id.unwrap();

        service
            .set_sync_capability(
                source_id,
                crate::models::calendar_source::SYNC_CAPABILITY_READ_WRITE,
            )
            .unwrap();
        service
            .set_api_sync_token(source_id, Some("next-page-token"))
            .unwrap();
        service.mark_last_push_now(source_id).unwrap();

        let fetched = service.get_by_id(source_id).unwrap().unwrap();
        assert_eq!(
            fetched.sync_capability,
            crate::models::calendar_source::SYNC_CAPABILITY_READ_WRITE
        );
        assert_eq!(fetched.api_sync_token.as_deref(), Some("next-page-token"));
        assert!(fetched.last_push_at.is_some());
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

    #[test]
    fn test_record_and_read_latest_sync_run() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let service = CalendarSourceService::new(db.connection());

        let created = service.create(build_source("Work")).unwrap();
        let source_id = created.id.unwrap();

        let diagnostics = SyncRunDiagnostics {
            source_id,
            started_at: "2026-03-06T08:00:00+10:00".to_string(),
            finished_at: "2026-03-06T08:00:02+10:00".to_string(),
            status: "success".to_string(),
            duration_ms: 1234,
            created_count: 2,
            updated_count: 1,
            deleted_count: 0,
            unchanged_count: 4,
            skipped_count: 1,
            error_count: 0,
            error_message: None,
        };

        service.record_sync_run(&diagnostics).unwrap();

        let fetched = service.latest_sync_run(source_id).unwrap().unwrap();
        assert_eq!(fetched.source_id, source_id);
        assert_eq!(fetched.duration_ms, 1234);
        assert_eq!(fetched.created_count, 2);
        assert_eq!(fetched.unchanged_count, 4);
        assert_eq!(fetched.error_count, 0);
    }

    #[test]
    fn test_update_sync_status_with_diagnostics_persists_backoff_status() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let service = CalendarSourceService::new(db.connection());

        let created = service.create(build_source("Work")).unwrap();
        let source_id = created.id.unwrap();

        let diagnostics = SyncRunDiagnostics {
            source_id,
            started_at: "2026-03-06T09:00:00+10:00".to_string(),
            finished_at: "2026-03-06T09:00:01+10:00".to_string(),
            status: "backoff".to_string(),
            duration_ms: 500,
            created_count: 0,
            updated_count: 0,
            deleted_count: 0,
            unchanged_count: 0,
            skipped_count: 0,
            error_count: 1,
            error_message: Some("retry after 15 minute(s)".to_string()),
        };

        service
            .update_sync_status_with_diagnostics(
                source_id,
                Some("backoff"),
                Some("retry after 15 minute(s)"),
                Some(&diagnostics),
            )
            .unwrap();

        let fetched_source = service.get_by_id(source_id).unwrap().unwrap();
        assert_eq!(fetched_source.last_sync_status.as_deref(), Some("backoff"));
        assert_eq!(
            fetched_source.last_error.as_deref(),
            Some("retry after 15 minute(s)")
        );

        let fetched_run = service.latest_sync_run(source_id).unwrap().unwrap();
        assert_eq!(fetched_run.status, "backoff");
        assert_eq!(fetched_run.error_count, 1);
        assert_eq!(
            fetched_run.error_message.as_deref(),
            Some("retry after 15 minute(s)")
        );
    }
}
