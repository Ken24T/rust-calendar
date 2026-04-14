//! Common utility functions for calendar views.

use chrono::{Datelike, NaiveDate};

/// Get the number of days in a given month.
///
/// # Arguments
/// * `year` - The year
/// * `month` - The month (1-12)
///
/// # Returns
/// The number of days in that month (28, 29, 30, or 31)
pub fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(30)
}

/// Get full weekday names rotated to the configured first day of week.
pub fn get_full_day_names(first_day_of_week: u8) -> Vec<&'static str> {
    let all_days = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    rotate_day_names(&all_days, first_day_of_week)
}

/// Get short weekday names rotated to the configured first day of week.
pub fn get_short_day_names(first_day_of_week: u8) -> Vec<&'static str> {
    let all_days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    rotate_day_names(&all_days, first_day_of_week)
}

fn rotate_day_names<const N: usize>(
    all_days: &[&'static str; N],
    first_day_of_week: u8,
) -> Vec<&'static str> {
    let start = first_day_of_week as usize;
    let mut result = Vec::with_capacity(N);
    for index in 0..N {
        result.push(all_days[(start + index) % N]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_in_month_regular() {
        assert_eq!(days_in_month(2024, 1), 31);
        assert_eq!(days_in_month(2024, 4), 30);
        assert_eq!(days_in_month(2024, 6), 30);
        assert_eq!(days_in_month(2024, 12), 31);
    }

    #[test]
    fn test_days_in_month_february() {
        // Leap year
        assert_eq!(days_in_month(2024, 2), 29);
        // Non-leap year
        assert_eq!(days_in_month(2023, 2), 28);
    }

    #[test]
    fn test_get_full_day_names_sunday_start() {
        let day_names = get_full_day_names(0);
        assert_eq!(
            day_names,
            vec![
                "Sunday",
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday"
            ]
        );
    }

    #[test]
    fn test_get_short_day_names_monday_start() {
        let day_names = get_short_day_names(1);
        assert_eq!(
            day_names,
            vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        );
    }
}
