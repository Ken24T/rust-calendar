#![allow(dead_code)]

use serde::{Deserialize, Serialize};

pub const SYNC_CONFLICT_STATUS_OPEN: &str = "open";
pub const SYNC_CONFLICT_STATUS_RESOLVED: &str = "resolved";

pub const SYNC_CONFLICT_REASON_LOCAL_CREATE_PENDING: &str = "local_create_pending";
pub const SYNC_CONFLICT_REASON_LOCAL_UPDATE_PENDING: &str = "local_update_pending";
pub const SYNC_CONFLICT_REASON_LOCAL_DELETE_PENDING: &str = "local_delete_pending";

pub const SYNC_CONFLICT_RESOLUTION_REMOTE_WINS: &str = "remote_wins";
pub const SYNC_CONFLICT_RESOLUTION_RETRY_LOCAL: &str = "retry_local";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncConflict {
    pub id: Option<i64>,
    pub source_id: i64,
    pub local_event_id: Option<i64>,
    pub external_uid: String,
    pub outbound_operation_id: Option<i64>,
    pub local_operation_type: Option<String>,
    pub remote_change_type: String,
    pub reason: String,
    pub resolution: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
    pub resolved_at: Option<String>,
    pub updated_at: Option<String>,
}

impl SyncConflict {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_id <= 0 {
            return Err("source_id must be greater than 0".to_string());
        }

        if self.external_uid.trim().is_empty() {
            return Err("external_uid cannot be empty".to_string());
        }

        if self.remote_change_type.trim().is_empty() {
            return Err("remote_change_type cannot be empty".to_string());
        }

        let reason = self.reason.trim();
        if reason != SYNC_CONFLICT_REASON_LOCAL_CREATE_PENDING
            && reason != SYNC_CONFLICT_REASON_LOCAL_UPDATE_PENDING
            && reason != SYNC_CONFLICT_REASON_LOCAL_DELETE_PENDING
        {
            return Err("reason must describe a supported pending local operation".to_string());
        }

        let status = self.status.trim();
        if status != SYNC_CONFLICT_STATUS_OPEN && status != SYNC_CONFLICT_STATUS_RESOLVED {
            return Err("status must be open or resolved".to_string());
        }

        if let Some(resolution) = &self.resolution {
            let resolution = resolution.trim();
            if resolution != SYNC_CONFLICT_RESOLUTION_REMOTE_WINS
                && resolution != SYNC_CONFLICT_RESOLUTION_RETRY_LOCAL
            {
                return Err("resolution must be remote_wins or retry_local".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_conflict() -> SyncConflict {
        SyncConflict {
            id: None,
            source_id: 1,
            local_event_id: Some(42),
            external_uid: "uid-42".to_string(),
            outbound_operation_id: Some(7),
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
    fn test_validate_valid_sync_conflict() {
        assert!(valid_conflict().validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_reason() {
        let conflict = SyncConflict {
            reason: "other".to_string(),
            ..valid_conflict()
        };
        assert!(conflict.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_resolution() {
        let conflict = SyncConflict {
            resolution: Some("manual".to_string()),
            ..valid_conflict()
        };
        assert!(conflict.validate().is_err());
    }
}
