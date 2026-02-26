#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use rusqlite::{params, Connection};

use crate::models::event_sync_map::EventSyncMap;

pub struct EventSyncMapService<'a> {
    conn: &'a Connection,
}

impl<'a> EventSyncMapService<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create(&self, mut mapping: EventSyncMap) -> Result<EventSyncMap> {
        mapping.validate().map_err(|err| anyhow!(err))?;

        let now = Local::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO event_sync_map (
                    source_id, external_uid, local_event_id,
                    external_last_modified, external_etag_hash,
                    last_seen_at, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    mapping.source_id,
                    mapping.external_uid,
                    mapping.local_event_id,
                    mapping.external_last_modified,
                    mapping.external_etag_hash,
                    mapping.last_seen_at,
                    now,
                    now,
                ],
            )
            .context("Failed to insert event sync map row")?;

        mapping.id = Some(self.conn.last_insert_rowid());
        Ok(mapping)
    }

    pub fn get_by_source_and_uid(&self, source_id: i64, external_uid: &str) -> Result<Option<EventSyncMap>> {
        let result = self.conn.query_row(
            "SELECT id, source_id, external_uid, local_event_id,
                    external_last_modified, external_etag_hash, last_seen_at
             FROM event_sync_map
             WHERE source_id = ?1 AND external_uid = ?2",
            params![source_id, external_uid],
            Self::row_to_mapping,
        );

        match result {
            Ok(mapping) => Ok(Some(mapping)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err).context("Failed to fetch event sync map row"),
        }
    }

    pub fn touch_last_seen(&self, source_id: i64, external_uid: &str) -> Result<()> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE event_sync_map
                 SET last_seen_at = ?1,
                     updated_at = ?2
                 WHERE source_id = ?3 AND external_uid = ?4",
                params![
                    Local::now().to_rfc3339(),
                    Local::now().to_rfc3339(),
                    source_id,
                    external_uid,
                ],
            )
            .context("Failed to update event sync map last_seen_at")?;

        if rows_affected == 0 {
            return Err(anyhow!(
                "Event sync map row not found for source_id={} external_uid={}",
                source_id,
                external_uid
            ));
        }

        Ok(())
    }

    pub fn delete_by_source_and_uid(&self, source_id: i64, external_uid: &str) -> Result<()> {
        let rows_affected = self
            .conn
            .execute(
                "DELETE FROM event_sync_map WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, external_uid],
            )
            .context("Failed to delete event sync map row")?;

        if rows_affected == 0 {
            return Err(anyhow!(
                "Event sync map row not found for source_id={} external_uid={}",
                source_id,
                external_uid
            ));
        }

        Ok(())
    }

    fn row_to_mapping(row: &rusqlite::Row<'_>) -> rusqlite::Result<EventSyncMap> {
        Ok(EventSyncMap {
            id: Some(row.get(0)?),
            source_id: row.get(1)?,
            external_uid: row.get(2)?,
            local_event_id: row.get(3)?,
            external_last_modified: row.get(4)?,
            external_etag_hash: row.get(5)?,
            last_seen_at: row.get(6)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::EventSyncMapService;
    use crate::models::event_sync_map::EventSyncMap;
    use crate::services::database::Database;
    use rusqlite::{params, Connection};

    fn create_source(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO calendar_sources (name, source_type, ics_url, enabled, poll_interval_minutes)
             VALUES (?1, ?2, ?3, 1, 15)",
            params![
                "Test Source",
                "google_ics",
                "https://calendar.google.com/calendar/ical/test%40gmail.com/private-token/basic.ics"
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn create_event(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO events (title, start_datetime, end_datetime, is_all_day)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                "Mapped event",
                "2026-02-27T09:00:00+10:00",
                "2026-02-27T10:00:00+10:00",
                0
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn test_create_and_get_mapping() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);
        let local_event_id = create_event(conn);
        let service = EventSyncMapService::new(conn);

        let mapping = EventSyncMap {
            id: None,
            source_id,
            external_uid: "uid-1".to_string(),
            local_event_id,
            external_last_modified: Some("20260227T010000Z".to_string()),
            external_etag_hash: None,
            last_seen_at: Some("2026-02-27T01:00:00Z".to_string()),
        };

        let created = service.create(mapping).unwrap();
        assert!(created.id.is_some());

        let fetched = service
            .get_by_source_and_uid(source_id, "uid-1")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.local_event_id, local_event_id);
    }

    #[test]
    fn test_unique_source_uid_constraint() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);
        let local_event_id = create_event(conn);
        let service = EventSyncMapService::new(conn);

        let first = EventSyncMap {
            id: None,
            source_id,
            external_uid: "uid-dupe".to_string(),
            local_event_id,
            external_last_modified: None,
            external_etag_hash: None,
            last_seen_at: None,
        };
        service.create(first).unwrap();

        let second_event_id = create_event(conn);
        let duplicate = EventSyncMap {
            id: None,
            source_id,
            external_uid: "uid-dupe".to_string(),
            local_event_id: second_event_id,
            external_last_modified: None,
            external_etag_hash: None,
            last_seen_at: None,
        };

        assert!(service.create(duplicate).is_err());
    }

    #[test]
    fn test_touch_last_seen_updates_value() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);
        let local_event_id = create_event(conn);
        let service = EventSyncMapService::new(conn);

        let mapping = EventSyncMap {
            id: None,
            source_id,
            external_uid: "uid-touch".to_string(),
            local_event_id,
            external_last_modified: None,
            external_etag_hash: None,
            last_seen_at: None,
        };
        service.create(mapping).unwrap();

        service.touch_last_seen(source_id, "uid-touch").unwrap();

        let fetched = service
            .get_by_source_and_uid(source_id, "uid-touch")
            .unwrap()
            .unwrap();
        assert!(fetched.last_seen_at.is_some());
    }

    #[test]
    fn test_delete_by_source_and_uid() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);
        let local_event_id = create_event(conn);
        let service = EventSyncMapService::new(conn);

        let mapping = EventSyncMap {
            id: None,
            source_id,
            external_uid: "uid-del".to_string(),
            local_event_id,
            external_last_modified: None,
            external_etag_hash: None,
            last_seen_at: None,
        };
        service.create(mapping).unwrap();

        service.delete_by_source_and_uid(source_id, "uid-del").unwrap();
        assert!(service
            .get_by_source_and_uid(source_id, "uid-del")
            .unwrap()
            .is_none());
    }
}
