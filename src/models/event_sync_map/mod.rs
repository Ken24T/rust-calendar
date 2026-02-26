#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSyncMap {
    pub id: Option<i64>,
    pub source_id: i64,
    pub external_uid: String,
    pub local_event_id: i64,
    pub external_last_modified: Option<String>,
    pub external_etag_hash: Option<String>,
    pub last_seen_at: Option<String>,
}

impl EventSyncMap {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_id <= 0 {
            return Err("source_id must be greater than 0".to_string());
        }

        if self.local_event_id <= 0 {
            return Err("local_event_id must be greater than 0".to_string());
        }

        if self.external_uid.trim().is_empty() {
            return Err("external_uid cannot be empty".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::EventSyncMap;

    fn valid_map() -> EventSyncMap {
        EventSyncMap {
            id: None,
            source_id: 1,
            external_uid: "uid-123".to_string(),
            local_event_id: 99,
            external_last_modified: None,
            external_etag_hash: None,
            last_seen_at: None,
        }
    }

    #[test]
    fn test_validate_valid_event_sync_map() {
        assert!(valid_map().validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_source_id() {
        let map = EventSyncMap {
            source_id: 0,
            ..valid_map()
        };
        assert!(map.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_local_event_id() {
        let map = EventSyncMap {
            local_event_id: -1,
            ..valid_map()
        };
        assert!(map.validate().is_err());
    }

    #[test]
    fn test_validate_empty_external_uid() {
        let map = EventSyncMap {
            external_uid: "   ".to_string(),
            ..valid_map()
        };
        assert!(map.validate().is_err());
    }
}
