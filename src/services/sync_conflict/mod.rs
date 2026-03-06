#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use rusqlite::{params, Connection};

use crate::models::sync_conflict::{
    SyncConflict, SYNC_CONFLICT_RESOLUTION_REMOTE_WINS, SYNC_CONFLICT_RESOLUTION_RETRY_LOCAL,
    SYNC_CONFLICT_STATUS_OPEN, SYNC_CONFLICT_STATUS_RESOLVED,
};

pub struct SyncConflictService<'a> {
    conn: &'a Connection,
}

impl<'a> SyncConflictService<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn upsert_open(&self, conflict: &SyncConflict) -> Result<SyncConflict> {
        conflict.validate().map_err(|err| anyhow!(err))?;

        let now = Local::now().to_rfc3339();
        if let Some(existing) =
            self.get_open_by_source_and_uid(conflict.source_id, &conflict.external_uid)?
        {
            let id = existing
                .id
                .ok_or_else(|| anyhow!("Open sync conflict is missing an ID"))?;

            self.conn
                .execute(
                    "UPDATE sync_conflicts
                     SET local_event_id = ?1,
                         outbound_operation_id = ?2,
                         local_operation_type = ?3,
                         remote_change_type = ?4,
                         reason = ?5,
                         resolution = ?6,
                         updated_at = ?7
                     WHERE id = ?8",
                    params![
                        conflict.local_event_id,
                        conflict.outbound_operation_id,
                        conflict.local_operation_type,
                        conflict.remote_change_type,
                        conflict.reason,
                        conflict.resolution,
                        now,
                        id,
                    ],
                )
                .context("Failed to update open sync conflict")?;

            return self
                .get_by_id(id)?
                .ok_or_else(|| anyhow!("Updated sync conflict could not be reloaded"));
        }

        let mut stored = conflict.clone();
        self.conn
            .execute(
                "INSERT INTO sync_conflicts (
                    source_id, local_event_id, external_uid, outbound_operation_id,
                    local_operation_type, remote_change_type, reason, resolution,
                    status, created_at, resolved_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11)",
                params![
                    stored.source_id,
                    stored.local_event_id,
                    stored.external_uid,
                    stored.outbound_operation_id,
                    stored.local_operation_type,
                    stored.remote_change_type,
                    stored.reason,
                    stored.resolution,
                    SYNC_CONFLICT_STATUS_OPEN,
                    now,
                    now,
                ],
            )
            .context("Failed to insert sync conflict")?;

        stored.id = Some(self.conn.last_insert_rowid());
        stored.created_at = Some(now.clone());
        stored.updated_at = Some(now);
        stored.resolved_at = None;
        stored.status = SYNC_CONFLICT_STATUS_OPEN.to_string();
        Ok(stored)
    }

    pub fn count_open_for_source(&self, source_id: i64) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM sync_conflicts WHERE source_id = ?1 AND status = ?2",
                params![source_id, SYNC_CONFLICT_STATUS_OPEN],
                |row| row.get(0),
            )
            .context("Failed to count open sync conflicts")
    }

    pub fn list_open_for_source(&self, source_id: i64, limit: i64) -> Result<Vec<SyncConflict>> {
        let safe_limit = limit.clamp(1, 1000);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, source_id, local_event_id, external_uid, outbound_operation_id,
                        local_operation_type, remote_change_type, reason, resolution,
                        status, created_at, resolved_at, updated_at
                 FROM sync_conflicts
                 WHERE source_id = ?1 AND status = ?2
                 ORDER BY updated_at DESC
                 LIMIT ?3",
            )
            .context("Failed to prepare open sync conflicts query")?;

        let rows = stmt.query_map(
            params![source_id, SYNC_CONFLICT_STATUS_OPEN, safe_limit],
            Self::row_to_conflict,
        )?;

        rows.collect::<Result<Vec<_>, _>>()
            .context("Failed to load open sync conflicts")
    }

    pub fn mark_resolved(&self, conflict_id: i64, resolution: &str) -> Result<()> {
        Self::validate_resolution(resolution)?;
        let now = Local::now().to_rfc3339();
        let rows_affected = self
            .conn
            .execute(
                "UPDATE sync_conflicts
                 SET status = ?1,
                     resolution = ?2,
                     resolved_at = ?3,
                     updated_at = ?4
                 WHERE id = ?5",
                params![
                    SYNC_CONFLICT_STATUS_RESOLVED,
                    resolution,
                    now,
                    Local::now().to_rfc3339(),
                    conflict_id,
                ],
            )
            .context("Failed to resolve sync conflict")?;

        if rows_affected == 0 {
            return Err(anyhow!("Sync conflict with id {} not found", conflict_id));
        }

        Ok(())
    }

    pub fn resolve_open_for_identity(
        &self,
        source_id: i64,
        external_uid: &str,
        resolution: &str,
    ) -> Result<()> {
        Self::validate_resolution(resolution)?;
        let now = Local::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE sync_conflicts
                 SET status = ?1,
                     resolution = ?2,
                     resolved_at = ?3,
                     updated_at = ?4
                 WHERE source_id = ?5
                   AND external_uid = ?6
                   AND status = ?7",
                params![
                    SYNC_CONFLICT_STATUS_RESOLVED,
                    resolution,
                    now,
                    Local::now().to_rfc3339(),
                    source_id,
                    external_uid,
                    SYNC_CONFLICT_STATUS_OPEN,
                ],
            )
            .context("Failed to resolve open sync conflict for identity")?;

        Ok(())
    }

    fn get_open_by_source_and_uid(
        &self,
        source_id: i64,
        external_uid: &str,
    ) -> Result<Option<SyncConflict>> {
        let result = self.conn.query_row(
            "SELECT id, source_id, local_event_id, external_uid, outbound_operation_id,
                    local_operation_type, remote_change_type, reason, resolution,
                    status, created_at, resolved_at, updated_at
             FROM sync_conflicts
             WHERE source_id = ?1 AND external_uid = ?2 AND status = ?3
             ORDER BY updated_at DESC
             LIMIT 1",
            params![source_id, external_uid, SYNC_CONFLICT_STATUS_OPEN],
            Self::row_to_conflict,
        );

        match result {
            Ok(conflict) => Ok(Some(conflict)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err).context("Failed to load open sync conflict by identity"),
        }
    }

    fn get_by_id(&self, conflict_id: i64) -> Result<Option<SyncConflict>> {
        let result = self.conn.query_row(
            "SELECT id, source_id, local_event_id, external_uid, outbound_operation_id,
                    local_operation_type, remote_change_type, reason, resolution,
                    status, created_at, resolved_at, updated_at
             FROM sync_conflicts
             WHERE id = ?1",
            [conflict_id],
            Self::row_to_conflict,
        );

        match result {
            Ok(conflict) => Ok(Some(conflict)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err).context("Failed to load sync conflict by ID"),
        }
    }

    fn validate_resolution(resolution: &str) -> Result<()> {
        if resolution != SYNC_CONFLICT_RESOLUTION_REMOTE_WINS
            && resolution != SYNC_CONFLICT_RESOLUTION_RETRY_LOCAL
        {
            return Err(anyhow!(
                "Invalid sync conflict resolution '{}': expected remote_wins or retry_local",
                resolution
            ));
        }

        Ok(())
    }

    fn row_to_conflict(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncConflict> {
        Ok(SyncConflict {
            id: row.get(0)?,
            source_id: row.get(1)?,
            local_event_id: row.get(2)?,
            external_uid: row.get(3)?,
            outbound_operation_id: row.get(4)?,
            local_operation_type: row.get(5)?,
            remote_change_type: row.get(6)?,
            reason: row.get(7)?,
            resolution: row.get(8)?,
            status: row.get(9)?,
            created_at: row.get(10)?,
            resolved_at: row.get(11)?,
            updated_at: row.get(12)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SyncConflictService;
    use crate::models::sync_conflict::{
        SyncConflict, SYNC_CONFLICT_REASON_LOCAL_UPDATE_PENDING,
        SYNC_CONFLICT_RESOLUTION_REMOTE_WINS, SYNC_CONFLICT_STATUS_OPEN,
        SYNC_CONFLICT_STATUS_RESOLVED,
    };
    use crate::services::database::Database;
    use rusqlite::params;

    fn create_source(conn: &rusqlite::Connection) -> i64 {
        conn.execute(
            "INSERT INTO calendar_sources (name, source_type, ics_url, enabled, poll_interval_minutes)
             VALUES (?1, 'google_ics', ?2, 1, 15)",
            params![
                "Conflict Source",
                "https://calendar.google.com/calendar/ical/test%40gmail.com/private-token/basic.ics",
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn build_conflict(source_id: i64) -> SyncConflict {
        SyncConflict {
            id: None,
            source_id,
            local_event_id: None,
            external_uid: "uid-1".to_string(),
            outbound_operation_id: None,
            local_operation_type: Some("update".to_string()),
            remote_change_type: "update".to_string(),
            reason: SYNC_CONFLICT_REASON_LOCAL_UPDATE_PENDING.to_string(),
            resolution: Some(SYNC_CONFLICT_RESOLUTION_REMOTE_WINS.to_string()),
            status: SYNC_CONFLICT_STATUS_OPEN.to_string(),
            created_at: None,
            resolved_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn test_upsert_open_reuses_existing_conflict() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);
        let service = SyncConflictService::new(conn);

        let first = service.upsert_open(&build_conflict(source_id)).unwrap();
        let mut second = build_conflict(source_id);
        second.remote_change_type = "delete".to_string();

        let updated = service.upsert_open(&second).unwrap();

        assert_eq!(first.id, updated.id);
        assert_eq!(updated.remote_change_type, "delete");
        assert_eq!(service.count_open_for_source(source_id).unwrap(), 1);
    }

    #[test]
    fn test_mark_resolved_updates_status() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn);
        let service = SyncConflictService::new(conn);
        let created = service.upsert_open(&build_conflict(source_id)).unwrap();

        service
            .mark_resolved(created.id.unwrap(), SYNC_CONFLICT_RESOLUTION_REMOTE_WINS)
            .unwrap();

        let status: String = conn
            .query_row(
                "SELECT status FROM sync_conflicts WHERE id = ?1",
                [created.id.unwrap()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, SYNC_CONFLICT_STATUS_RESOLVED);
    }
}
