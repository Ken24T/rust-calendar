#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use rusqlite::{params, Connection};

use crate::models::calendar_source::SYNC_CAPABILITY_READ_WRITE;
use crate::models::outbound_sync_operation::{
    OutboundSyncOperation, OUTBOUND_STATUS_FAILED, OUTBOUND_STATUS_PENDING,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OutboundQueueStats {
    pub pending: i64,
    pub processing: i64,
    pub failed: i64,
    pub completed: i64,
}

pub struct OutboundSyncService<'a> {
    conn: &'a Connection,
}

impl<'a> OutboundSyncService<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn enqueue_upsert_for_local_event(
        &self,
        local_event_id: i64,
        operation_type: &str,
        payload_json: Option<&str>,
    ) -> Result<bool> {
        let mapping = self.lookup_writable_mapping(local_event_id)?;
        let Some((source_id, external_uid)) = mapping else {
            return Ok(false);
        };

        self.enqueue_for_identity(
            source_id,
            Some(local_event_id),
            &external_uid,
            operation_type,
            payload_json,
        )?;

        Ok(true)
    }

    pub fn writable_identity_for_local_event(
        &self,
        local_event_id: i64,
    ) -> Result<Option<(i64, String)>> {
        self.lookup_writable_mapping(local_event_id)
    }

    pub fn enqueue_delete_for_identity(
        &self,
        source_id: i64,
        external_uid: &str,
        payload_json: Option<&str>,
    ) -> Result<()> {
        if source_id <= 0 {
            return Err(anyhow!("source_id must be greater than 0"));
        }

        if external_uid.trim().is_empty() {
            return Err(anyhow!("external_uid cannot be empty"));
        }

        self.enqueue_for_identity(
            source_id,
            None,
            external_uid,
            crate::models::outbound_sync_operation::OUTBOUND_OPERATION_DELETE,
            payload_json,
        )
    }

    pub fn queue_stats_for_source(&self, source_id: i64) -> Result<OutboundQueueStats> {
        let stats = self.conn.query_row(
            "SELECT
                 COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE WHEN status = 'processing' THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0),
                 COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0)
             FROM outbound_sync_operations
             WHERE source_id = ?1",
            [source_id],
            |row| {
                Ok(OutboundQueueStats {
                    pending: row.get(0)?,
                    processing: row.get(1)?,
                    failed: row.get(2)?,
                    completed: row.get(3)?,
                })
            },
        )?;

        Ok(stats)
    }

    pub fn reset_failed_for_source(&self, source_id: i64) -> Result<usize> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE outbound_sync_operations
                 SET status = ?1,
                     attempt_count = 0,
                     next_retry_at = NULL,
                     last_error = NULL,
                     updated_at = ?2
                 WHERE source_id = ?3
                   AND status = ?4",
                params![
                    OUTBOUND_STATUS_PENDING,
                    Local::now().to_rfc3339(),
                    source_id,
                    OUTBOUND_STATUS_FAILED,
                ],
            )
            .context("Failed to reset failed outbound sync operations")?;

        Ok(rows_affected)
    }

    pub fn list_failed_for_source(
        &self,
        source_id: i64,
        limit: i64,
    ) -> Result<Vec<OutboundSyncOperation>> {
        let safe_limit = limit.clamp(1, 1000);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, source_id, local_event_id, external_uid, operation_type,
                        payload_json, status, attempt_count, next_retry_at, last_error,
                        created_at, updated_at
                 FROM outbound_sync_operations
                 WHERE source_id = ?1 AND status = ?2
                 ORDER BY updated_at DESC
                 LIMIT ?3",
            )
            .context("Failed to prepare failed outbound operations query")?;

        let rows = stmt.query_map(
            params![source_id, OUTBOUND_STATUS_FAILED, safe_limit],
            |row| {
                Ok(OutboundSyncOperation {
                    id: Some(row.get(0)?),
                    source_id: row.get(1)?,
                    local_event_id: row.get(2)?,
                    external_uid: row.get(3)?,
                    operation_type: row.get(4)?,
                    payload_json: row.get(5)?,
                    status: row.get(6)?,
                    attempt_count: row.get(7)?,
                    next_retry_at: row.get(8)?,
                    last_error: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )?;

        rows.collect::<Result<Vec<_>, _>>()
            .context("Failed to load failed outbound operations")
    }

    fn lookup_writable_mapping(&self, local_event_id: i64) -> Result<Option<(i64, String)>> {
        let result = self.conn.query_row(
            "SELECT esm.source_id, esm.external_uid
             FROM event_sync_map esm
             JOIN calendar_sources cs ON cs.id = esm.source_id
             WHERE esm.local_event_id = ?1
               AND cs.enabled = 1
               AND cs.sync_capability = ?2
             ORDER BY esm.id DESC
             LIMIT 1",
            params![local_event_id, SYNC_CAPABILITY_READ_WRITE],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        );

        match result {
            Ok(mapping) => Ok(Some(mapping)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err).context("Failed to find writable mapping for local event"),
        }
    }

    fn enqueue_for_identity(
        &self,
        source_id: i64,
        local_event_id: Option<i64>,
        external_uid: &str,
        operation_type: &str,
        payload_json: Option<&str>,
    ) -> Result<()> {
        if source_id <= 0 {
            return Err(anyhow!("source_id must be greater than 0"));
        }

        if external_uid.trim().is_empty() {
            return Err(anyhow!("external_uid cannot be empty"));
        }

        let now = Local::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO outbound_sync_operations (
                    source_id, local_event_id, external_uid, operation_type,
                    payload_json, status, attempt_count, next_retry_at,
                    last_error, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, NULL, NULL, ?7, ?8)
                 ON CONFLICT(source_id, external_uid) DO UPDATE SET
                    local_event_id = excluded.local_event_id,
                    operation_type = excluded.operation_type,
                    payload_json = excluded.payload_json,
                    status = excluded.status,
                    attempt_count = 0,
                    next_retry_at = NULL,
                    last_error = NULL,
                    updated_at = excluded.updated_at",
                params![
                    source_id,
                    local_event_id,
                    external_uid,
                    operation_type,
                    payload_json,
                    OUTBOUND_STATUS_PENDING,
                    now,
                    now,
                ],
            )
            .context("Failed to enqueue outbound sync operation")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::OutboundSyncService;
    use crate::models::calendar_source::SYNC_CAPABILITY_READ_WRITE;
    use crate::services::database::Database;
    use rusqlite::params;

    fn create_source(conn: &rusqlite::Connection) -> i64 {
        conn.execute(
            "INSERT INTO calendar_sources (name, source_type, ics_url, enabled, poll_interval_minutes, sync_capability)
             VALUES (?1, 'google_ics', ?2, 1, 15, ?3)",
            params![
                "RW Source",
                "https://calendar.google.com/calendar/ical/test%40gmail.com/private-token/basic.ics",
                SYNC_CAPABILITY_READ_WRITE,
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn test_enqueue_upsert_for_mapped_event() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);

        conn.execute(
            "INSERT INTO events (title, start_datetime, end_datetime, is_all_day)
             VALUES ('Local', '2025-01-01T10:00:00+00:00', '2025-01-01T11:00:00+00:00', 0)",
            [],
        )
        .unwrap();
        let event_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id)
             VALUES (?1, ?2, ?3)",
            params![source_id, "uid-1", event_id],
        )
        .unwrap();

        let service = OutboundSyncService::new(conn);
        let queued = service
            .enqueue_upsert_for_local_event(
                event_id,
                crate::models::outbound_sync_operation::OUTBOUND_OPERATION_UPDATE,
                Some("{\"title\":\"Updated\"}"),
            )
            .unwrap();

        assert!(queued);

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM outbound_sync_operations WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_reset_failed_for_source() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);

        conn.execute(
            "INSERT INTO outbound_sync_operations (
                source_id, local_event_id, external_uid, operation_type, payload_json,
                status, attempt_count, last_error
             ) VALUES (?1, NULL, ?2, 'delete', '{}', 'failed', 2, 'network')",
            params![source_id, "uid-delete"],
        )
        .unwrap();

        let service = OutboundSyncService::new(conn);
        let reset = service.reset_failed_for_source(source_id).unwrap();
        assert_eq!(reset, 1);

        let status: String = conn
            .query_row(
                "SELECT status FROM outbound_sync_operations WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            status,
            crate::models::outbound_sync_operation::OUTBOUND_STATUS_PENDING
        );
    }

    #[test]
    fn test_queue_stats_for_source() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);

        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, external_uid, operation_type, status)
             VALUES (?1, 'uid-a', 'update', 'pending')",
            [source_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, external_uid, operation_type, status)
             VALUES (?1, 'uid-b', 'update', 'failed')",
            [source_id],
        )
        .unwrap();

        let service = OutboundSyncService::new(conn);
        let stats = service.queue_stats_for_source(source_id).unwrap();

        assert_eq!(stats.pending, 1);
        assert_eq!(stats.failed, 1);
    }
}
