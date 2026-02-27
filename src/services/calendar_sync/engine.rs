#![allow(dead_code)]

use std::collections::HashSet;

use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;

use crate::models::event_sync_map::EventSyncMap;
use crate::services::event::EventService;
use crate::services::icalendar::import::{self, ImportedIcsEvent};

use super::fetcher::IcsFetcher;
use super::mapping::EventSyncMapService;
use super::CalendarSourceService;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncRunResult {
    pub source_id: i64,
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub skipped_missing_uid: usize,
    pub skipped_duplicate_uid: usize,
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

        let result = self
            .fetcher
            .fetch_ics(&source.ics_url)
            .and_then(|ics| self.sync_source_from_ics(source_id, &ics));

        match result {
            Ok(success) => Ok(success),
            Err(err) => {
                let redacted_error = Self::sanitize_error_message(&err.to_string(), &source.ics_url);
                let _ = source_service.update_sync_status(source_id, Some("failed"), Some(&redacted_error));
                Err(anyhow!(redacted_error))
            }
        }
    }

    pub fn sync_source_from_ics(&self, source_id: i64, ics_content: &str) -> Result<SyncRunResult> {
        let imported = import::from_str_with_metadata(ics_content)?;
        self.apply_imported(source_id, imported)
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
                        let mut updated_event = imported.event.clone();
                        updated_event.id = existing_event.id;
                        updated_event.created_at = existing_event.created_at;
                        event_service.update(&updated_event)?;
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
                            })
                            .context("Failed to create replacement mapping")?;

                        result.created += 1;
                        continue;
                    }

                    map_service.touch_last_seen(source_id, uid)?;
                    result.updated += 1;
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
                        })
                        .context("Failed to create event mapping")?;

                    result.created += 1;
                }
            }
        }

        let existing_maps = map_service.list_by_source_id(source_id)?;
        for mapping in existing_maps {
            if !seen_uids.contains(&mapping.external_uid) {
                map_service
                    .delete_by_source_and_uid(source_id, &mapping.external_uid)
                    .context("Failed to delete reconciled mapping")?;

                if event_service.get(mapping.local_event_id)?.is_some() {
                    event_service
                        .delete(mapping.local_event_id)
                        .context("Failed to delete reconciled local event")?;
                }
                result.deleted += 1;
            }
        }

        source_service.update_sync_status(source_id, Some("success"), None)?;

        Ok(result)
    }

    fn sanitize_error_message(message: &str, source_url: &str) -> String {
        if source_url.is_empty() {
            return message.to_string();
        }

        message.replace(source_url, "***redacted-url***")
    }
}

#[cfg(test)]
mod tests {
    use super::CalendarSyncEngine;
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
        assert_eq!(next.deleted, 1);

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
}
