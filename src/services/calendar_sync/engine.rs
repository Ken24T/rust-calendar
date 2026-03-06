#![allow(dead_code)]

use std::collections::HashSet;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Local};
use rusqlite::Connection;

use crate::models::event_sync_map::EventSyncMap;
use crate::models::event::Event;
use crate::services::event::EventService;
use crate::services::icalendar::import::{self, ImportedIcsEvent};

use super::fetcher::IcsFetcher;
use super::mapping::EventSyncMapService;
use super::{CalendarSourceService, SyncRunDiagnostics};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncRunResult {
    pub source_id: i64,
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub unchanged: usize,
    pub skipped_missing_uid: usize,
    pub skipped_duplicate_uid: usize,
    pub skipped_filtered: usize,
    pub error_count: usize,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncBatchResult {
    pub completed: Vec<SyncRunResult>,
    pub failed_sources: Vec<(i64, String)>,
}

pub struct CalendarSyncEngine<'a> {
    conn: &'a Connection,
    fetcher: IcsFetcher,
}

impl<'a> CalendarSyncEngine<'a> {
    pub fn new(conn: &'a Connection) -> Result<Self> {
        Ok(Self {
            conn,
            fetcher: IcsFetcher::new()?,
        })
    }

    pub fn sync_source(&self, source_id: i64) -> Result<SyncRunResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let started_at = chrono::Local::now();
        let timer = Instant::now();

        let result = self
            .fetcher
            .fetch_ics(&source.ics_url)
            .and_then(|ics| self.sync_source_from_ics(source_id, &ics));

        match result {
            Ok(mut success) => {
                let finished_at = chrono::Local::now();
                success.started_at = Some(started_at.to_rfc3339());
                success.finished_at = Some(finished_at.to_rfc3339());
                success.duration_ms = timer.elapsed().as_millis();
                success.error_count = 0;

                let diagnostics = SyncRunDiagnostics {
                    source_id,
                    started_at: success.started_at.clone().unwrap_or_default(),
                    finished_at: success.finished_at.clone().unwrap_or_default(),
                    status: "success".to_string(),
                    duration_ms: i64::try_from(success.duration_ms).unwrap_or(i64::MAX),
                    created_count: success.created as i64,
                    updated_count: success.updated as i64,
                    deleted_count: success.deleted as i64,
                    unchanged_count: success.unchanged as i64,
                    skipped_count: (success.skipped_missing_uid
                        + success.skipped_duplicate_uid
                        + success.skipped_filtered) as i64,
                    error_count: 0,
                    error_message: None,
                };

                source_service.update_sync_status_with_diagnostics(
                    source_id,
                    Some("success"),
                    None,
                    Some(&diagnostics),
                )?;

                Ok(success)
            }
            Err(err) => {
                let finished_at = chrono::Local::now();
                let redacted_error = Self::sanitize_error_message(&err.to_string(), &source.ics_url);

                let diagnostics = SyncRunDiagnostics {
                    source_id,
                    started_at: started_at.to_rfc3339(),
                    finished_at: finished_at.to_rfc3339(),
                    status: "failed".to_string(),
                    duration_ms: i64::try_from(timer.elapsed().as_millis()).unwrap_or(i64::MAX),
                    created_count: 0,
                    updated_count: 0,
                    deleted_count: 0,
                    unchanged_count: 0,
                    skipped_count: 0,
                    error_count: 1,
                    error_message: Some(redacted_error.clone()),
                };

                let _ = source_service.update_sync_status_with_diagnostics(
                    source_id,
                    Some("failed"),
                    Some(&redacted_error),
                    Some(&diagnostics),
                );
                Err(anyhow!(redacted_error))
            }
        }
    }

    pub fn sync_source_from_ics(&self, source_id: i64, ics_content: &str) -> Result<SyncRunResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let imported = import::from_str_with_metadata(ics_content)?;

        if imported.is_empty() && ics_content.contains("BEGIN:VEVENT") {
            return Err(anyhow!(
                "ICS payload contained VEVENT markers but no events were parsed; aborting sync to avoid accidental deletions"
            ));
        }

        let (filtered, skipped_filtered) = Self::filter_imported_by_window(&source, imported);
        let mut result = self.apply_imported(source_id, filtered)?;
        result.skipped_filtered = skipped_filtered;
        Ok(result)
    }

    pub fn preview_source(&self, source_id: i64) -> Result<SyncRunResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        self.fetcher
            .fetch_ics(&source.ics_url)
            .and_then(|ics| self.preview_source_from_ics(source_id, &ics))
    }

    pub fn preview_source_from_ics(&self, source_id: i64, ics_content: &str) -> Result<SyncRunResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let imported = import::from_str_with_metadata(ics_content)?;

        if imported.is_empty() && ics_content.contains("BEGIN:VEVENT") {
            return Err(anyhow!(
                "ICS payload contained VEVENT markers but no events were parsed; aborting preview"
            ));
        }

        let (filtered, skipped_filtered) = Self::filter_imported_by_window(&source, imported);
        let mut result = self.preview_imported(source_id, filtered)?;
        result.skipped_filtered = skipped_filtered;
        Ok(result)
    }

    pub fn sync_all_enabled_sources(&self) -> Result<SyncBatchResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let sources = source_service.list_all()?;

        let mut batch = SyncBatchResult::default();
        for source in sources.into_iter().filter(|source| source.enabled) {
            let Some(source_id) = source.id else {
                continue;
            };

            match self.sync_source(source_id) {
                Ok(result) => batch.completed.push(result),
                Err(err) => batch.failed_sources.push((source_id, err.to_string())),
            }
        }

        Ok(batch)
    }

    fn apply_imported(&self, source_id: i64, imported_events: Vec<ImportedIcsEvent>) -> Result<SyncRunResult> {
        let mut result = SyncRunResult {
            source_id,
            ..SyncRunResult::default()
        };

        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let map_service = EventSyncMapService::new(self.conn);
        let event_service = EventService::new(self.conn);

        let mut seen_uids: HashSet<String> = HashSet::new();

        for imported in imported_events {
            let uid = imported
                .uid
                .as_deref()
                .map(str::trim)
                .filter(|uid| !uid.is_empty());

            let Some(uid) = uid else {
                result.skipped_missing_uid += 1;
                continue;
            };

            if !seen_uids.insert(uid.to_string()) {
                result.skipped_duplicate_uid += 1;
                continue;
            }

            match map_service.get_by_source_and_uid(source_id, uid)? {
                Some(existing_map) => {
                    if let Some(existing_event) = event_service.get(existing_map.local_event_id)? {
                        if Self::is_effectively_unchanged(&existing_event, &imported.event) {
                            result.unchanged += 1;
                        } else {
                            let mut updated_event = imported.event.clone();
                            updated_event.id = existing_event.id;
                            updated_event.created_at = existing_event.created_at;
                            event_service.update(&updated_event)?;
                            result.updated += 1;
                        }
                    } else {
                        let created_event = event_service
                            .create(imported.event.clone())
                            .context("Failed to create event for stale mapping")?;

                        map_service
                            .delete_by_source_and_uid(source_id, uid)
                            .context("Failed to remove stale mapping")?;

                        map_service
                            .create(EventSyncMap {
                                id: None,
                                source_id,
                                external_uid: uid.to_string(),
                                local_event_id: created_event.id.ok_or_else(|| {
                                    anyhow!("Created event missing ID for mapping")
                                })?,
                                external_last_modified: imported.raw_last_modified.clone(),
                                external_etag_hash: None,
                                last_seen_at: Some(chrono::Local::now().to_rfc3339()),
                                first_missing_at: None,
                                purge_after_at: None,
                            })
                            .context("Failed to create replacement mapping")?;

                        result.created += 1;
                        continue;
                    }

                    map_service.touch_last_seen(source_id, uid)?;
                }
                None => {
                    let created_event = event_service
                        .create(imported.event.clone())
                        .context("Failed to create imported event")?;

                    map_service
                        .create(EventSyncMap {
                            id: None,
                            source_id,
                            external_uid: uid.to_string(),
                            local_event_id: created_event.id.ok_or_else(|| {
                                anyhow!("Created event missing ID for mapping")
                            })?,
                            external_last_modified: imported.raw_last_modified.clone(),
                            external_etag_hash: None,
                            last_seen_at: Some(chrono::Local::now().to_rfc3339()),
                            first_missing_at: None,
                            purge_after_at: None,
                        })
                        .context("Failed to create event mapping")?;

                    result.created += 1;
                }
            }
        }

        let existing_maps = map_service.list_by_source_id(source_id)?;
        let now = Local::now();
        let grace_minutes = (source.poll_interval_minutes.max(1) * 3).max(30);
        for mapping in existing_maps {
            if !seen_uids.contains(&mapping.external_uid) {
                let should_purge = mapping
                    .purge_after_at
                    .as_deref()
                    .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
                    .map(|dt| now >= dt.with_timezone(&Local))
                    .unwrap_or(false);

                if should_purge {
                    map_service
                        .delete_by_source_and_uid(source_id, &mapping.external_uid)
                        .context("Failed to delete reconciled mapping")?;

                    if event_service.get(mapping.local_event_id)?.is_some() {
                        event_service
                            .delete(mapping.local_event_id)
                            .context("Failed to delete reconciled local event")?;
                    }
                    result.deleted += 1;
                } else {
                    let first_missing_at = now.to_rfc3339();
                    let purge_after_at = (now + Duration::minutes(grace_minutes)).to_rfc3339();
                    map_service
                        .mark_missing(
                            source_id,
                            &mapping.external_uid,
                            &first_missing_at,
                            &purge_after_at,
                        )
                        .context("Failed to stage reconciled deletion")?;
                }
            }
        }

        Ok(result)
    }

    fn preview_imported(&self, source_id: i64, imported_events: Vec<ImportedIcsEvent>) -> Result<SyncRunResult> {
        let mut result = SyncRunResult {
            source_id,
            ..SyncRunResult::default()
        };

        let source_service = CalendarSourceService::new(self.conn);
        source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let map_service = EventSyncMapService::new(self.conn);
        let event_service = EventService::new(self.conn);

        let mut seen_uids: HashSet<String> = HashSet::new();

        for imported in imported_events {
            let uid = imported
                .uid
                .as_deref()
                .map(str::trim)
                .filter(|uid| !uid.is_empty());

            let Some(uid) = uid else {
                result.skipped_missing_uid += 1;
                continue;
            };

            if !seen_uids.insert(uid.to_string()) {
                result.skipped_duplicate_uid += 1;
                continue;
            }

            match map_service.get_by_source_and_uid(source_id, uid)? {
                Some(existing_map) => {
                    if let Some(existing_event) = event_service.get(existing_map.local_event_id)? {
                        if Self::is_effectively_unchanged(&existing_event, &imported.event) {
                            result.unchanged += 1;
                        } else {
                            result.updated += 1;
                        }
                    } else {
                        // Mapping exists but points to missing event; apply path would recreate it.
                        result.created += 1;
                    }
                }
                None => {
                    result.created += 1;
                }
            }
        }

        let existing_maps = map_service.list_by_source_id(source_id)?;
        for mapping in existing_maps {
            if !seen_uids.contains(&mapping.external_uid) {
                result.deleted += 1;
            }
        }

        Ok(result)
    }

    fn is_effectively_unchanged(existing: &Event, incoming: &Event) -> bool {
        existing.title == incoming.title
            && existing.description == incoming.description
            && existing.location == incoming.location
            && existing.start == incoming.start
            && existing.end == incoming.end
            && existing.all_day == incoming.all_day
            && existing.category == incoming.category
            && existing.color == incoming.color
            && existing.recurrence_rule == incoming.recurrence_rule
            && existing.recurrence_exceptions == incoming.recurrence_exceptions
    }

    fn sanitize_error_message(message: &str, source_url: &str) -> String {
        if source_url.is_empty() {
            return message.to_string();
        }

        message.replace(source_url, "***redacted-url***")
    }

    fn filter_imported_by_window(
        source: &crate::models::calendar_source::CalendarSource,
        imported_events: Vec<ImportedIcsEvent>,
    ) -> (Vec<ImportedIcsEvent>, usize) {
        let now = Local::now();
        let past_cutoff = now - Duration::days(source.sync_past_days.max(0));
        let future_cutoff = now + Duration::days(source.sync_future_days.max(1));

        let mut kept = Vec::with_capacity(imported_events.len());
        let mut skipped = 0usize;

        for imported in imported_events {
            let in_window = imported.event.end >= past_cutoff && imported.event.start <= future_cutoff;
            if in_window {
                kept.push(imported);
            } else {
                skipped += 1;
            }
        }

        (kept, skipped)
    }
}

#[cfg(test)]
mod tests {
    use super::CalendarSyncEngine;
    use chrono::{Duration, Local};
    use crate::services::calendar_sync::mapping::EventSyncMapService;
    use crate::services::database::Database;
    use crate::services::event::EventService;
    use rusqlite::{params, Connection};

    fn create_source(conn: &Connection, name: &str, enabled: bool) -> i64 {
        conn.execute(
            "INSERT INTO calendar_sources (name, source_type, ics_url, enabled, poll_interval_minutes)
             VALUES (?1, ?2, ?3, ?4, 15)",
            params![
                name,
                "google_ics",
                "https://calendar.google.com/calendar/ical/test%40gmail.com/private-token/basic.ics",
                enabled as i32,
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn set_source_windows(conn: &Connection, source_id: i64, past_days: i64, future_days: i64) {
        conn.execute(
            "UPDATE calendar_sources SET sync_past_days = ?1, sync_future_days = ?2 WHERE id = ?3",
            params![past_days, future_days, source_id],
        )
        .unwrap();
    }

    #[test]
    fn test_sync_source_from_ics_creates_and_updates_events() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let ics_first = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-100
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Original Title
    LAST-MODIFIED:20260227T000000Z
    END:VEVENT
    END:VCALENDAR"#;

        let result_first = engine.sync_source_from_ics(source_id, ics_first).unwrap();
        assert_eq!(result_first.created, 1);
        assert_eq!(result_first.updated, 0);

        let ics_second = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-100
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Updated Title
    LAST-MODIFIED:20260227T010000Z
    END:VEVENT
    END:VCALENDAR"#;

        let result_second = engine.sync_source_from_ics(source_id, ics_second).unwrap();
        assert_eq!(result_second.created, 0);
        assert_eq!(result_second.updated, 1);

        let event_service = EventService::new(conn);
        let all_events = event_service.list_all().unwrap();
        assert_eq!(all_events.len(), 1);
        assert_eq!(all_events[0].title, "Updated Title");
    }

    #[test]
    fn test_sync_source_from_ics_reconciles_deleted_events() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let ics_initial = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-a
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Event A
    END:VEVENT
    BEGIN:VEVENT
    UID:uid-b
    DTSTART:20260227T110000
    DTEND:20260227T120000
    SUMMARY:Event B
    END:VEVENT
    END:VCALENDAR"#;

        let initial = engine.sync_source_from_ics(source_id, ics_initial).unwrap();
        assert_eq!(initial.created, 2);

        let ics_next = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-a
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Event A
    END:VEVENT
    END:VCALENDAR"#;

        let next = engine.sync_source_from_ics(source_id, ics_next).unwrap();
        assert_eq!(next.deleted, 0);

        // First missing run should stage deletion, not purge immediately.
        let staged_maps = EventSyncMapService::new(conn)
            .list_by_source_id(source_id)
            .unwrap();
        let staged = staged_maps
            .iter()
            .find(|m| m.external_uid == "uid-b")
            .expect("uid-b mapping should be staged for deletion");
        assert!(staged.first_missing_at.is_some());
        assert!(staged.purge_after_at.is_some());

        // Force grace window expiry and run again to purge.
        conn.execute(
            "UPDATE event_sync_map SET purge_after_at = ?1 WHERE source_id = ?2 AND external_uid = ?3",
            params!["2000-01-01T00:00:00+00:00", source_id, "uid-b"],
        )
        .unwrap();

        let purge = engine.sync_source_from_ics(source_id, ics_next).unwrap();
        assert_eq!(purge.deleted, 1);

        let event_service = EventService::new(conn);
        let all_events = event_service.list_all().unwrap();
        assert_eq!(all_events.len(), 1);
        assert_eq!(all_events[0].title, "Event A");

        let map_service = EventSyncMapService::new(conn);
        let maps = map_service.list_by_source_id(source_id).unwrap();
        assert_eq!(maps.len(), 1);
        assert_eq!(maps[0].external_uid, "uid-a");
    }

    #[test]
    fn test_sync_source_from_ics_skips_missing_uid() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let ics = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:No UID Event
    END:VEVENT
    END:VCALENDAR"#;

        let result = engine.sync_source_from_ics(source_id, ics).unwrap();
        assert_eq!(result.skipped_missing_uid, 1);
        assert_eq!(result.created, 0);

        let event_service = EventService::new(conn);
        assert_eq!(event_service.list_all().unwrap().len(), 0);
    }

    #[test]
    fn test_sync_source_from_ics_aborts_when_vevent_present_but_nothing_parsed() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let good_ics = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-safe
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Existing Event
    END:VEVENT
    END:VCALENDAR"#;

        let initial = engine.sync_source_from_ics(source_id, good_ics).unwrap();
        assert_eq!(initial.created, 1);

        let suspicious_ics = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-safe
    DTSTART:20260227T090000
    DTEND:20260227T100000
    END:VEVENT
    END:VCALENDAR"#;

        let result = engine.sync_source_from_ics(source_id, suspicious_ics);
        assert!(result.is_err());

        let event_service = EventService::new(conn);
        let all_events = event_service.list_all().unwrap();
        assert_eq!(all_events.len(), 1);
        assert_eq!(all_events[0].title, "Existing Event");
    }

    #[test]
    fn test_sync_source_from_ics_counts_unchanged() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let ics = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-200
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Stable Event
    END:VEVENT
    END:VCALENDAR"#;

        let first = engine.sync_source_from_ics(source_id, ics).unwrap();
        assert_eq!(first.created, 1);

        let second = engine.sync_source_from_ics(source_id, ics).unwrap();
        assert_eq!(second.unchanged, 1);
        assert_eq!(second.updated, 0);
    }

    #[test]
    fn test_preview_source_from_ics_reports_changes_without_writing() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let ics = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:preview-1
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Preview Event
    END:VEVENT
    END:VCALENDAR"#;

        let preview = engine.preview_source_from_ics(source_id, ics).unwrap();
        assert_eq!(preview.created, 1);
        assert_eq!(preview.updated, 0);
        assert_eq!(preview.deleted, 0);

        // Preview must not persist events or mappings.
        let event_service = EventService::new(conn);
        assert_eq!(event_service.list_all().unwrap().len(), 0);

        let map_service = EventSyncMapService::new(conn);
        assert_eq!(map_service.list_by_source_id(source_id).unwrap().len(), 0);
    }

    #[test]
    fn test_preview_source_from_ics_reports_delete_candidates() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let initial = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-a
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Event A
    END:VEVENT
    BEGIN:VEVENT
    UID:uid-b
    DTSTART:20260227T110000
    DTEND:20260227T120000
    SUMMARY:Event B
    END:VEVENT
    END:VCALENDAR"#;

        let _ = engine.sync_source_from_ics(source_id, initial).unwrap();

        let next = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-a
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Event A
    END:VEVENT
    END:VCALENDAR"#;

        let preview = engine.preview_source_from_ics(source_id, next).unwrap();
        assert_eq!(preview.deleted, 1);

        // Preview must not apply deletions.
        let event_service = EventService::new(conn);
        assert_eq!(event_service.list_all().unwrap().len(), 2);
    }

    #[test]
    fn test_sync_source_from_ics_respects_source_date_window() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);
        set_source_windows(conn, source_id, 0, 1);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let now = Local::now();
        let in_window_start = now + Duration::hours(6);
        let in_window_end = in_window_start + Duration::hours(1);
        let out_window_start = now + Duration::days(5);
        let out_window_end = out_window_start + Duration::hours(1);

        let ics = format!(
            "BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nUID:keep-uid\nDTSTART:{}\nDTEND:{}\nSUMMARY:Keep Event\nEND:VEVENT\nBEGIN:VEVENT\nUID:skip-uid\nDTSTART:{}\nDTEND:{}\nSUMMARY:Skip Event\nEND:VEVENT\nEND:VCALENDAR",
            in_window_start.format("%Y%m%dT%H%M%S"),
            in_window_end.format("%Y%m%dT%H%M%S"),
            out_window_start.format("%Y%m%dT%H%M%S"),
            out_window_end.format("%Y%m%dT%H%M%S"),
        );

        let result = engine.sync_source_from_ics(source_id, &ics).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.skipped_filtered, 1);

        let event_service = EventService::new(conn);
        let events = event_service.list_all().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Keep Event");
    }
}
