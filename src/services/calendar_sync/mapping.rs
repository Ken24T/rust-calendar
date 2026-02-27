#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use rusqlite::{params, Connection};
use std::collections::HashSet;

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

    pub fn list_by_source_id(&self, source_id: i64) -> Result<Vec<EventSyncMap>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, source_id, external_uid, local_event_id,
                        external_last_modified, external_etag_hash, last_seen_at
                 FROM event_sync_map
                 WHERE source_id = ?1",
            )
            .context("Failed to prepare event_sync_map list query")?;

        let rows = stmt.query_map([source_id], Self::row_to_mapping)?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("Failed to list event sync map rows by source")
    }

    pub fn is_synced_local_event(&self, local_event_id: i64) -> Result<bool> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM event_sync_map WHERE local_event_id = ?1",
                [local_event_id],
                |row| row.get(0),
            )
            .context("Failed to check synced status for local event")?;

        Ok(count > 0)
    }

    pub fn get_source_name_for_local_event(&self, local_event_id: i64) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT cs.name
             FROM event_sync_map esm
             JOIN calendar_sources cs ON cs.id = esm.source_id
             WHERE esm.local_event_id = ?1
             ORDER BY esm.id DESC
             LIMIT 1",
            [local_event_id],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(name) => Ok(Some(name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err).context("Failed to fetch source name for local event"),
        }
    }

    pub fn list_synced_local_event_ids(&self) -> Result<HashSet<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT local_event_id FROM event_sync_map")
            .context("Failed to prepare synced local event ids query")?;

        let rows = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .context("Failed to execute synced local event ids query")?;

        let ids = rows
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect synced local event ids")?;

        Ok(ids.into_iter().collect())
    }

    pub fn list_synced_local_event_ids_for_enabled_sources(&self) -> Result<HashSet<i64>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT esm.local_event_id
                 FROM event_sync_map esm
                 JOIN calendar_sources cs ON cs.id = esm.source_id
                 WHERE cs.enabled = 1",
            )
            .context("Failed to prepare synced local event ids query for enabled sources")?;

        let rows = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .context("Failed to execute synced local event ids query for enabled sources")?;

        let ids = rows
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect synced local event ids for enabled sources")?;

        Ok(ids.into_iter().collect())
    }

    pub fn list_synced_local_event_ids_for_enabled_source(
        &self,
        source_id: i64,
    ) -> Result<HashSet<i64>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT esm.local_event_id
                 FROM event_sync_map esm
                 JOIN calendar_sources cs ON cs.id = esm.source_id
                 WHERE cs.enabled = 1 AND esm.source_id = ?1",
            )
            .context("Failed to prepare synced local event ids query for enabled source")?;

        let rows = stmt
            .query_map([source_id], |row| row.get::<_, i64>(0))
            .context("Failed to execute synced local event ids query for enabled source")?;

        let ids = rows
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect synced local event ids for enabled source")?;

        Ok(ids.into_iter().collect())
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
        create_source_named(conn, "Test Source")
    }

    fn create_source_named(conn: &Connection, name: &str) -> i64 {
        conn.execute(
            "INSERT INTO calendar_sources (name, source_type, ics_url, enabled, poll_interval_minutes)
             VALUES (?1, ?2, ?3, 1, 15)",
            params![
                name,
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
    fn test_get_source_name_for_local_event() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();

        let source_id = create_source_named(conn, "Google Work");
        let local_event_id = create_event(conn);
        let service = EventSyncMapService::new(conn);

        service
            .create(EventSyncMap {
                id: None,
                source_id,
                external_uid: "uid-source-name".to_string(),
                local_event_id,
                external_last_modified: None,
                external_etag_hash: None,
                last_seen_at: None,
            })
            .unwrap();

        let source_name = service
            .get_source_name_for_local_event(local_event_id)
            .unwrap();
        assert_eq!(source_name.as_deref(), Some("Google Work"));

        let missing = service.get_source_name_for_local_event(9999).unwrap();
        assert!(missing.is_none());
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

    #[test]
    fn test_list_by_source_id() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);
        let local_event_id = create_event(conn);
        let service = EventSyncMapService::new(conn);

        let mapping = EventSyncMap {
            id: None,
            source_id,
            external_uid: "uid-list".to_string(),
            local_event_id,
            external_last_modified: None,
            external_etag_hash: None,
            last_seen_at: None,
        };
        service.create(mapping).unwrap();

        let listed = service.list_by_source_id(source_id).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].external_uid, "uid-list");
    }

    #[test]
    fn test_is_synced_local_event_and_list_ids() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);
        let local_event_id = create_event(conn);
        let other_event_id = create_event(conn);
        let service = EventSyncMapService::new(conn);

        let mapping = EventSyncMap {
            id: None,
            source_id,
            external_uid: "uid-synced".to_string(),
            local_event_id,
            external_last_modified: None,
            external_etag_hash: None,
            last_seen_at: None,
        };
        service.create(mapping).unwrap();

        assert!(service.is_synced_local_event(local_event_id).unwrap());
        assert!(!service.is_synced_local_event(other_event_id).unwrap());

        let ids = service.list_synced_local_event_ids().unwrap();
        assert!(ids.contains(&local_event_id));
        assert!(!ids.contains(&other_event_id));
    }

    #[test]
    fn test_list_synced_ids_for_enabled_sources_only() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();

        let enabled_source_id = create_source_named(conn, "Enabled Source");
        let disabled_source_id = create_source_named(conn, "Disabled Source");

        conn.execute(
            "UPDATE calendar_sources SET enabled = 0 WHERE id = ?1",
            [disabled_source_id],
        )
        .unwrap();

        let enabled_event_id = create_event(conn);
        let disabled_event_id = create_event(conn);
        let service = EventSyncMapService::new(conn);

        service
            .create(EventSyncMap {
                id: None,
                source_id: enabled_source_id,
                external_uid: "uid-enabled".to_string(),
                local_event_id: enabled_event_id,
                external_last_modified: None,
                external_etag_hash: None,
                last_seen_at: None,
            })
            .unwrap();

        service
            .create(EventSyncMap {
                id: None,
                source_id: disabled_source_id,
                external_uid: "uid-disabled".to_string(),
                local_event_id: disabled_event_id,
                external_last_modified: None,
                external_etag_hash: None,
                last_seen_at: None,
            })
            .unwrap();

        let enabled_ids = service
            .list_synced_local_event_ids_for_enabled_sources()
            .unwrap();

        assert!(enabled_ids.contains(&enabled_event_id));
        assert!(!enabled_ids.contains(&disabled_event_id));
    }

    #[test]
    fn test_list_synced_ids_for_single_enabled_source_only() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();

        let source_a = create_source_named(conn, "Source A");
        let source_b = create_source_named(conn, "Source B");

        let event_a = create_event(conn);
        let event_b = create_event(conn);

        let service = EventSyncMapService::new(conn);

        service
            .create(EventSyncMap {
                id: None,
                source_id: source_a,
                external_uid: "uid-source-a".to_string(),
                local_event_id: event_a,
                external_last_modified: None,
                external_etag_hash: None,
                last_seen_at: None,
            })
            .unwrap();

        service
            .create(EventSyncMap {
                id: None,
                source_id: source_b,
                external_uid: "uid-source-b".to_string(),
                local_event_id: event_b,
                external_last_modified: None,
                external_etag_hash: None,
                last_seen_at: None,
            })
            .unwrap();

        let source_a_ids = service
            .list_synced_local_event_ids_for_enabled_source(source_a)
            .unwrap();

        assert!(source_a_ids.contains(&event_a));
        assert!(!source_a_ids.contains(&event_b));
    }
}
