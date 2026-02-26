#![allow(dead_code)]

use serde::{Deserialize, Serialize};

pub const GOOGLE_ICS_SOURCE_TYPE: &str = "google_ics";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarSource {
    pub id: Option<i64>,
    pub name: String,
    pub source_type: String,
    pub ics_url: String,
    pub enabled: bool,
    pub poll_interval_minutes: i64,
    pub last_sync_at: Option<String>,
    pub last_sync_status: Option<String>,
    pub last_error: Option<String>,
}

impl CalendarSource {
    pub fn validate(&self) -> Result<(), String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Calendar source name cannot be empty".to_string());
        }

        if self.source_type != GOOGLE_ICS_SOURCE_TYPE {
            return Err("Calendar source type must be 'google_ics'".to_string());
        }

        if !Self::is_valid_google_ics_url(&self.ics_url) {
            return Err("Calendar source URL must be a valid Google Calendar ICS URL".to_string());
        }

        if self.poll_interval_minutes <= 0 {
            return Err("Poll interval must be greater than 0 minutes".to_string());
        }

        Ok(())
    }

    pub fn is_valid_google_ics_url(url: &str) -> bool {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return false;
        }

        trimmed.starts_with("https://calendar.google.com/calendar/ical/")
            && trimmed.ends_with(".ics")
    }
}

impl Default for CalendarSource {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            source_type: GOOGLE_ICS_SOURCE_TYPE.to_string(),
            ics_url: String::new(),
            enabled: true,
            poll_interval_minutes: 15,
            last_sync_at: None,
            last_sync_status: None,
            last_error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CalendarSource, GOOGLE_ICS_SOURCE_TYPE};

    fn valid_source() -> CalendarSource {
        CalendarSource {
            id: None,
            name: "Personal Google".to_string(),
            source_type: GOOGLE_ICS_SOURCE_TYPE.to_string(),
            ics_url: "https://calendar.google.com/calendar/ical/test%40gmail.com/private-abc123/basic.ics".to_string(),
            enabled: true,
            poll_interval_minutes: 15,
            last_sync_at: None,
            last_sync_status: None,
            last_error: None,
        }
    }

    #[test]
    fn test_validate_valid_source() {
        let source = valid_source();
        assert!(source.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let source = CalendarSource {
            name: "  ".to_string(),
            ..valid_source()
        };
        assert!(source.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_source_type() {
        let source = CalendarSource {
            source_type: "other".to_string(),
            ..valid_source()
        };
        assert!(source.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_url() {
        let source = CalendarSource {
            ics_url: "https://example.com/calendar.ics".to_string(),
            ..valid_source()
        };
        assert!(source.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_poll_interval() {
        let source = CalendarSource {
            poll_interval_minutes: 0,
            ..valid_source()
        };
        assert!(source.validate().is_err());
    }

    #[test]
    fn test_google_ics_url_validation() {
        assert!(CalendarSource::is_valid_google_ics_url(
            "https://calendar.google.com/calendar/ical/debp200517%40gmail.com/public/basic.ics"
        ));
        assert!(CalendarSource::is_valid_google_ics_url(
            "https://calendar.google.com/calendar/ical/debp200517%40gmail.com/private-abcdef/basic.ics"
        ));
        assert!(!CalendarSource::is_valid_google_ics_url(
            "http://calendar.google.com/calendar/ical/debp200517%40gmail.com/public/basic.ics"
        ));
        assert!(!CalendarSource::is_valid_google_ics_url(
            "https://example.com/calendar.ics"
        ));
    }
}
