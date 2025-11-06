// Test fixtures - reusable test data
// Provides consistent test data across all test files

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

/// Sample dates for testing
pub mod dates {
    use super::*;
    
    /// Returns Jan 1, 2025 at midnight
    pub fn jan_1_2025() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }
    
    /// Returns Feb 14, 2025 at 14:00 (Valentine's Day)
    pub fn valentine_2025() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2025, 2, 14)
            .unwrap()
            .and_hms_opt(14, 0, 0)
            .unwrap()
    }
    
    /// Returns Dec 31, 2025 at 23:59 (New Year's Eve)
    pub fn new_years_eve_2025() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2025, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 0)
            .unwrap()
    }
    
    /// Returns Feb 29, 2024 (leap year)
    pub fn leap_day_2024() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 2, 29)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }
}

/// Sample events for testing
pub mod events {
    use super::*;
    
    /// Creates a simple single event
    pub fn simple_event() -> MockEvent {
        MockEvent {
            title: "Simple Event".to_string(),
            start: dates::jan_1_2025(),
            end: dates::jan_1_2025(),
            recurrence: None,
        }
    }
    
    /// Creates a fortnightly recurring event
    pub fn fortnightly_meeting() -> MockEvent {
        MockEvent {
            title: "Bi-weekly Team Standup".to_string(),
            start: dates::jan_1_2025(),
            end: dates::jan_1_2025(),
            recurrence: Some(MockRecurrence::Fortnightly),
        }
    }
    
    /// Creates a quarterly review event
    pub fn quarterly_review() -> MockEvent {
        MockEvent {
            title: "Quarterly Business Review".to_string(),
            start: dates::jan_1_2025(),
            end: dates::jan_1_2025(),
            recurrence: Some(MockRecurrence::Quarterly),
        }
    }
    
    /// Creates an all-day event
    pub fn all_day_event() -> MockEvent {
        MockEvent {
            title: "All Day Conference".to_string(),
            start: NaiveDate::from_ymd_opt(2025, 1, 15)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            end: NaiveDate::from_ymd_opt(2025, 1, 15)
                .unwrap()
                .and_hms_opt(23, 59, 59)
                .unwrap(),
            recurrence: None,
        }
    }
}

// Mock types for testing (will be replaced with actual types)
#[derive(Debug, Clone)]
pub struct MockEvent {
    pub title: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub recurrence: Option<MockRecurrence>,
}

#[derive(Debug, Clone, Copy)]
pub enum MockRecurrence {
    Fortnightly,
    Quarterly,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fixture_dates_are_valid() {
        // Ensure fixture dates are valid
        assert!(dates::jan_1_2025().year() == 2025);
        assert!(dates::valentine_2025().month() == 2);
        assert!(dates::new_years_eve_2025().day() == 31);
        assert!(dates::leap_day_2024().day() == 29);
    }
    
    #[test]
    fn test_fixture_events_are_valid() {
        // Ensure fixture events are valid
        let event = events::simple_event();
        assert_eq!(event.title, "Simple Event");
        
        let fortnightly = events::fortnightly_meeting();
        assert!(fortnightly.recurrence.is_some());
        
        let quarterly = events::quarterly_review();
        assert!(quarterly.recurrence.is_some());
    }
}
