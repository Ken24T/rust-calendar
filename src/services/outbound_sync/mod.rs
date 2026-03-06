#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use rusqlite::{params, Connection};

use crate::models::calendar_source::SYNC_CAPABILITY_READ_WRITE;
use crate::models::outbound_sync_operation::{
    OutboundSyncOperation, OUTBOUND_STATUS_COMPLETED, OUTBOUND_STATUS_FAILED,
    OUTBOUND_STATUS_PENDING, OUTBOUND_STATUS_PROCESSING,
};

const DEFAULT_BACKOFF_BASE_MINUTES: i64 = 1;
const DEFAULT_MAX_BACKOFF_MINUTES: i64 = 60;
const BROKEN_REMOTE_METADATA_ERROR_FRAGMENT: &str = "missing remote_event_id";

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

    pub fn enqueue_upsert_for_identity(
        &self,
        source_id: i64,
        local_event_id: i64,
        external_uid: &str,
        operation_type: &str,
        payload_json: Option<&str>,
    ) -> Result<()> {
        self.enqueue_for_identity(
            source_id,
            Some(local_event_id),
            external_uid,
            operation_type,
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
                   AND status = ?4
                   AND COALESCE(last_error, '') NOT LIKE ?5",
                params![
                    OUTBOUND_STATUS_PENDING,
                    Local::now().to_rfc3339(),
                    source_id,
                    OUTBOUND_STATUS_FAILED,
                    format!("%{}%", BROKEN_REMOTE_METADATA_ERROR_FRAGMENT),
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

    pub fn list_pending_for_source(
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
                 ORDER BY updated_at ASC
                 LIMIT ?3",
            )
            .context("Failed to prepare pending outbound operations query")?;

        let rows = stmt.query_map(
            params![source_id, OUTBOUND_STATUS_PENDING, safe_limit],
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
            .context("Failed to load pending outbound operations")
    }

    pub fn list_runnable_for_source(
        &self,
        source_id: i64,
        limit: i64,
    ) -> Result<Vec<OutboundSyncOperation>> {
        let safe_limit = limit.clamp(1, 1000);
        let now = Local::now().to_rfc3339();
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, source_id, local_event_id, external_uid, operation_type,
                        payload_json, status, attempt_count, next_retry_at, last_error,
                        created_at, updated_at
                 FROM outbound_sync_operations
                 WHERE source_id = ?1
                   AND (
                        status = ?2
                        OR (status = ?3 AND next_retry_at IS NOT NULL AND next_retry_at <= ?4)
                   )
                 ORDER BY CASE WHEN status = ?2 THEN 0 ELSE 1 END, updated_at ASC
                 LIMIT ?5",
            )
            .context("Failed to prepare runnable outbound operations query")?;

        let rows = stmt.query_map(
            params![
                source_id,
                OUTBOUND_STATUS_PENDING,
                OUTBOUND_STATUS_FAILED,
                now,
                safe_limit,
            ],
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
            .context("Failed to load runnable outbound operations")
    }

    pub fn active_operation_for_identity(
        &self,
        source_id: i64,
        external_uid: &str,
    ) -> Result<Option<OutboundSyncOperation>> {
        let result = self.conn.query_row(
            "SELECT id, source_id, local_event_id, external_uid, operation_type,
                    payload_json, status, attempt_count, next_retry_at, last_error,
                    created_at, updated_at
             FROM outbound_sync_operations
             WHERE source_id = ?1
               AND external_uid = ?2
               AND status IN (?3, ?4, ?5)
             ORDER BY updated_at DESC
             LIMIT 1",
            params![
                source_id,
                external_uid,
                OUTBOUND_STATUS_PENDING,
                OUTBOUND_STATUS_PROCESSING,
                OUTBOUND_STATUS_FAILED,
            ],
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
        );

        match result {
            Ok(operation) => Ok(Some(operation)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err).context("Failed to load active outbound operation"),
        }
    }

    pub fn mark_operation_failed(&self, operation_id: i64, error: &str) -> Result<()> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE outbound_sync_operations
                 SET status = ?1,
                     next_retry_at = NULL,
                     last_error = ?2,
                     updated_at = ?3
                 WHERE id = ?4",
                params![
                    OUTBOUND_STATUS_FAILED,
                    error,
                    Local::now().to_rfc3339(),
                    operation_id,
                ],
            )
            .context("Failed to mark outbound sync operation as terminal failed")?;

        if rows_affected == 0 {
            return Err(anyhow!(
                "Outbound sync operation with id {} not found",
                operation_id
            ));
        }

        Ok(())
    }

    pub fn mark_operation_failed_with_retry(
        &self,
        operation_id: i64,
        attempt_count: i64,
        base_backoff_minutes: i64,
        error: &str,
    ) -> Result<()> {
        let retry_at = Local::now()
            + chrono::Duration::minutes(Self::calculate_backoff_minutes(
                base_backoff_minutes,
                attempt_count,
            ));

        let rows_affected = self
            .conn
            .execute(
                "UPDATE outbound_sync_operations
                 SET status = ?1,
                     next_retry_at = ?2,
                     last_error = ?3,
                     updated_at = ?4
                 WHERE id = ?5",
                params![
                    OUTBOUND_STATUS_FAILED,
                    retry_at.to_rfc3339(),
                    error,
                    Local::now().to_rfc3339(),
                    operation_id,
                ],
            )
            .context("Failed to mark outbound sync operation as failed with retry")?;

        if rows_affected == 0 {
            return Err(anyhow!(
                "Outbound sync operation with id {} not found",
                operation_id
            ));
        }

        Ok(())
    }

    pub fn mark_operation_completed(&self, operation_id: i64) -> Result<()> {
        self.update_operation_status(operation_id, OUTBOUND_STATUS_COMPLETED, None)
    }

    pub fn mark_operation_processing(&self, operation_id: i64) -> Result<()> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE outbound_sync_operations
                 SET status = ?1,
                     attempt_count = attempt_count + 1,
                     next_retry_at = NULL,
                     last_error = NULL,
                     updated_at = ?2
                 WHERE id = ?3",
                params![
                    OUTBOUND_STATUS_PROCESSING,
                    Local::now().to_rfc3339(),
                    operation_id,
                ],
            )
            .context("Failed to mark outbound sync operation as processing")?;

        if rows_affected == 0 {
            return Err(anyhow!(
                "Outbound sync operation with id {} not found",
                operation_id
            ));
        }

        Ok(())
    }

    pub fn reset_operation_to_pending(&self, operation_id: i64) -> Result<()> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE outbound_sync_operations
                 SET status = ?1,
                     attempt_count = 0,
                     next_retry_at = NULL,
                     last_error = NULL,
                     updated_at = ?2
                 WHERE id = ?3",
                params![
                    OUTBOUND_STATUS_PENDING,
                    Local::now().to_rfc3339(),
                    operation_id
                ],
            )
            .context("Failed to reset outbound sync operation to pending")?;

        if rows_affected == 0 {
            return Err(anyhow!(
                "Outbound sync operation with id {} not found",
                operation_id
            ));
        }

        Ok(())
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

    fn update_operation_status(
        &self,
        operation_id: i64,
        status: &str,
        last_error: Option<&str>,
    ) -> Result<()> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE outbound_sync_operations
                 SET status = ?1,
                     last_error = ?2,
                     updated_at = ?3
                 WHERE id = ?4",
                params![status, last_error, Local::now().to_rfc3339(), operation_id],
            )
            .context("Failed to update outbound sync operation status")?;

        if rows_affected == 0 {
            return Err(anyhow!(
                "Outbound sync operation with id {} not found",
                operation_id
            ));
        }

        Ok(())
    }

    fn calculate_backoff_minutes(base_backoff_minutes: i64, attempt_count: i64) -> i64 {
        let base = base_backoff_minutes.max(DEFAULT_BACKOFF_BASE_MINUTES);
        let exponent = attempt_count.clamp(1, 10) as u32;
        let backoff = base.saturating_mul(2_i64.saturating_pow(exponent));
        backoff.min(DEFAULT_MAX_BACKOFF_MINUTES.max(base))
    }
}

#[cfg(test)]
mod tests {
    use super::{OutboundSyncService, BROKEN_REMOTE_METADATA_ERROR_FRAGMENT};
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
    fn test_reset_failed_for_source_skips_broken_remote_metadata_failures() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);

        conn.execute(
            "INSERT INTO outbound_sync_operations (
                source_id, local_event_id, external_uid, operation_type, payload_json,
                status, attempt_count, last_error
             ) VALUES (?1, NULL, ?2, 'update', '{}', 'failed', 2, ?3)",
            params![
                source_id,
                "uid-broken",
                "Remote metadata for 'uid-broken' is missing remote_event_id"
            ],
        )
        .unwrap();

        let service = OutboundSyncService::new(conn);
        let reset = service.reset_failed_for_source(source_id).unwrap();
        assert_eq!(reset, 0);

        let (status, error): (String, Option<String>) = conn
            .query_row(
                "SELECT status, last_error FROM outbound_sync_operations WHERE source_id = ?1",
                [source_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            status,
            crate::models::outbound_sync_operation::OUTBOUND_STATUS_FAILED
        );
        assert!(error
            .as_deref()
            .is_some_and(|value| value.contains(BROKEN_REMOTE_METADATA_ERROR_FRAGMENT)));
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

    #[test]
    fn test_mark_operation_failed_and_reset_pending() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);

        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, external_uid, operation_type, status)
             VALUES (?1, 'uid-c', 'update', 'pending')",
            [source_id],
        )
        .unwrap();
        let operation_id = conn.last_insert_rowid();

        let service = OutboundSyncService::new(conn);
        service
            .mark_operation_failed(operation_id, "conflict detected")
            .unwrap();

        let failed: String = conn
            .query_row(
                "SELECT status FROM outbound_sync_operations WHERE id = ?1",
                [operation_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            failed,
            crate::models::outbound_sync_operation::OUTBOUND_STATUS_FAILED
        );

        service.reset_operation_to_pending(operation_id).unwrap();
        let reset: String = conn
            .query_row(
                "SELECT status FROM outbound_sync_operations WHERE id = ?1",
                [operation_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            reset,
            crate::models::outbound_sync_operation::OUTBOUND_STATUS_PENDING
        );
    }

    #[test]
    fn test_list_runnable_for_source_includes_due_failed_operations_only() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);

        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, external_uid, operation_type, status, next_retry_at)
             VALUES (?1, 'uid-pending', 'update', 'pending', NULL)",
            [source_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, external_uid, operation_type, status, next_retry_at)
             VALUES (?1, 'uid-due', 'update', 'failed', '2000-01-01T00:00:00+00:00')",
            [source_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, external_uid, operation_type, status, next_retry_at)
             VALUES (?1, 'uid-later', 'update', 'failed', '2999-01-01T00:00:00+00:00')",
            [source_id],
        )
        .unwrap();

        let service = OutboundSyncService::new(conn);
        let runnable = service.list_runnable_for_source(source_id, 10).unwrap();
        let external_uids = runnable
            .into_iter()
            .filter_map(|operation| operation.external_uid)
            .collect::<Vec<_>>();

        assert_eq!(external_uids, vec!["uid-pending".to_string(), "uid-due".to_string()]);
    }

    #[test]
    fn test_list_runnable_for_source_excludes_terminal_failed_operations() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);

        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, external_uid, operation_type, status, next_retry_at)
             VALUES (?1, 'uid-terminal', 'update', 'failed', NULL)",
            [source_id],
        )
        .unwrap();

        let service = OutboundSyncService::new(conn);
        let runnable = service.list_runnable_for_source(source_id, 10).unwrap();

        assert!(runnable.is_empty());
    }

    #[test]
    fn test_mark_operation_failed_with_retry_sets_next_retry_at() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);

        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, external_uid, operation_type, status)
             VALUES (?1, 'uid-retry', 'update', 'processing')",
            [source_id],
        )
        .unwrap();
        let operation_id = conn.last_insert_rowid();

        let service = OutboundSyncService::new(conn);
        service
            .mark_operation_failed_with_retry(operation_id, 2, 5, "temporary outage")
            .unwrap();

        let (status, next_retry_at, error): (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT status, next_retry_at, last_error FROM outbound_sync_operations WHERE id = ?1",
                [operation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(status, crate::models::outbound_sync_operation::OUTBOUND_STATUS_FAILED);
        assert!(next_retry_at.is_some());
        assert_eq!(error.as_deref(), Some("temporary outage"));
    }

    #[test]
    fn test_mark_operation_failed_clears_next_retry_at() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);

        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, external_uid, operation_type, status, next_retry_at)
             VALUES (?1, 'uid-terminal-fail', 'update', 'processing', '2999-01-01T00:00:00+00:00')",
            [source_id],
        )
        .unwrap();
        let operation_id = conn.last_insert_rowid();

        let service = OutboundSyncService::new(conn);
        service
            .mark_operation_failed(operation_id, "broken mapping")
            .unwrap();

        let (status, next_retry_at, error): (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT status, next_retry_at, last_error FROM outbound_sync_operations WHERE id = ?1",
                [operation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(status, crate::models::outbound_sync_operation::OUTBOUND_STATUS_FAILED);
        assert!(next_retry_at.is_none());
        assert_eq!(error.as_deref(), Some("broken mapping"));
    }
}
