#![allow(dead_code)]

// Date utility functions
// Implementation pending - Phase 1

use chrono::{DateTime, Local};

pub fn is_same_day(date1: DateTime<Local>, date2: DateTime<Local>) -> bool {
    date1.date_naive() == date2.date_naive()
}

pub fn start_of_day(date: DateTime<Local>) -> DateTime<Local> {
    date.date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(date.timezone())
        .unwrap()
}

pub fn end_of_day(date: DateTime<Local>) -> DateTime<Local> {
    date.date_naive()
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_local_timezone(date.timezone())
        .unwrap()
}

/// Get short day names (3-letter abbreviations) starting from the given first day of week.
///
/// # Arguments
/// * `first_day_of_week` - 0 = Sunday, 1 = Monday, etc.
///
/// # Returns
/// A vector of 7 short day names starting from the specified first day of week.
pub fn get_short_day_names(first_day_of_week: u8) -> Vec<&'static str> {
    const SHORT_DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let start = first_day_of_week as usize;
    (0..7).map(|i| SHORT_DAYS[(start + i) % 7]).collect()
}

/// Get full day names starting from the given first day of week.
///
/// # Arguments
/// * `first_day_of_week` - 0 = Sunday, 1 = Monday, etc.
///
/// # Returns
/// A vector of 7 full day names starting from the specified first day of week.
pub fn get_full_day_names(first_day_of_week: u8) -> Vec<&'static str> {
    const FULL_DAYS: [&str; 7] = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    let start = first_day_of_week as usize;
    (0..7).map(|i| FULL_DAYS[(start + i) % 7]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_short_day_names_sunday_start() {
        let names = get_short_day_names(0);
        assert_eq!(names, vec!["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]);
    }

    #[test]
    fn test_get_short_day_names_monday_start() {
        let names = get_short_day_names(1);
        assert_eq!(names, vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]);
    }

    #[test]
    fn test_get_full_day_names_sunday_start() {
        let names = get_full_day_names(0);
        assert_eq!(names, vec!["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"]);
    }

    #[test]
    fn test_get_full_day_names_monday_start() {
        let names = get_full_day_names(1);
        assert_eq!(names, vec!["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]);
    }
}
