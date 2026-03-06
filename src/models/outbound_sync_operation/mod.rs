#![allow(dead_code)]

use serde::{Deserialize, Serialize};

pub const OUTBOUND_OPERATION_CREATE: &str = "create";
pub const OUTBOUND_OPERATION_UPDATE: &str = "update";
pub const OUTBOUND_OPERATION_DELETE: &str = "delete";

pub const OUTBOUND_STATUS_PENDING: &str = "pending";
pub const OUTBOUND_STATUS_PROCESSING: &str = "processing";
pub const OUTBOUND_STATUS_FAILED: &str = "failed";
pub const OUTBOUND_STATUS_COMPLETED: &str = "completed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundSyncOperation {
    pub id: Option<i64>,
    pub source_id: i64,
    pub local_event_id: Option<i64>,
    pub external_uid: Option<String>,
    pub operation_type: String,
    pub payload_json: Option<String>,
    pub status: String,
    pub attempt_count: i64,
    pub next_retry_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl OutboundSyncOperation {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_id <= 0 {
            return Err("source_id must be greater than 0".to_string());
        }

        let operation_type = self.operation_type.trim();
        if operation_type != OUTBOUND_OPERATION_CREATE
            && operation_type != OUTBOUND_OPERATION_UPDATE
            && operation_type != OUTBOUND_OPERATION_DELETE
        {
            return Err("operation_type must be create, update, or delete".to_string());
        }

        let status = self.status.trim();
        if status != OUTBOUND_STATUS_PENDING
            && status != OUTBOUND_STATUS_PROCESSING
            && status != OUTBOUND_STATUS_FAILED
            && status != OUTBOUND_STATUS_COMPLETED
        {
            return Err("status must be pending, processing, failed, or completed".to_string());
        }

        if self.attempt_count < 0 {
            return Err("attempt_count must be non-negative".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_operation() -> OutboundSyncOperation {
        OutboundSyncOperation {
            id: None,
            source_id: 1,
            local_event_id: Some(42),
            external_uid: Some("uid-42".to_string()),
            operation_type: OUTBOUND_OPERATION_UPDATE.to_string(),
            payload_json: Some("{}".to_string()),
            status: OUTBOUND_STATUS_PENDING.to_string(),
            attempt_count: 0,
            next_retry_at: None,
            last_error: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn test_validate_valid_outbound_operation() {
        assert!(valid_operation().validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_source_id() {
        let op = OutboundSyncOperation {
            source_id: 0,
            ..valid_operation()
        };
        assert!(op.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_operation_type() {
        let op = OutboundSyncOperation {
            operation_type: "noop".to_string(),
            ..valid_operation()
        };
        assert!(op.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_status() {
        let op = OutboundSyncOperation {
            status: "queued".to_string(),
            ..valid_operation()
        };
        assert!(op.validate().is_err());
    }
}
