//! Common utility functions for calendar views.
//!
//! This module contains pure helper functions used across different view types.

use chrono::{Datelike, Duration, NaiveDate};
use egui::Color32;

use crate::models::event::Event;

/// Calculate the start of the week containing the given date.
///
/// # Arguments
/// * `date` - The date to find the week start for
/// * `first_day_of_week` - 0 = Sunday, 1 = Monday, etc.
pub fn get_week_start(date: NaiveDate, first_day_of_week: u8) -> NaiveDate {
    let weekday = date.weekday().num_days_from_sunday() as i64;
    let offset = (weekday - first_day_of_week as i64 + 7) % 7;
    date - Duration::days(offset)
}

/// Format a date in short form based on the date format setting.
///
/// # Arguments
/// * `date` - The date to format
/// * `date_format` - The format preference (e.g., "DD/MM/YYYY", "MM/DD/YYYY", "YYYY/MM/DD")
pub fn format_short_date(date: NaiveDate, date_format: &str) -> String {
    if date_format.starts_with("DD/MM") || date_format.starts_with("dd/mm") {
        date.format("%d/%m").to_string()
    } else if date_format.starts_with("YYYY") || date_format.starts_with("yyyy") {
        date.format("%Y/%m/%d").to_string()
    } else {
        date.format("%m/%d").to_string()
    }
}

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

/// Shift a date by the given number of months, preserving the day if possible.
///
/// # Arguments
/// * `date` - The starting date
/// * `delta` - The number of months to shift (positive = forward, negative = backward)
///
/// # Returns
/// The new date, with the day clamped to the last day of the target month if necessary.
/// For example, shifting Jan 31 by 1 month returns Feb 28 (or 29 in leap years).
pub fn shift_month(date: NaiveDate, delta: i32) -> NaiveDate {
    let total_months = (date.year() * 12) as i32 + (date.month() as i32 - 1) + delta;
    let new_year = total_months.div_euclid(12);
    let new_month = (total_months.rem_euclid(12) + 1) as u32;
    let max_day = days_in_month(new_year, new_month);
    let day = date.day().min(max_day);
    NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap_or(date)
}

/// Parse a hex color string to Color32.
///
/// # Arguments
/// * `hex` - A hex color string, optionally prefixed with '#' (e.g., "#FF5500" or "FF5500")
///
/// # Returns
/// * `Some(Color32)` if parsing succeeds
/// * `None` if the input is empty or invalid
pub fn parse_color(hex: &str) -> Option<Color32> {
    if hex.is_empty() {
        return None;
    }

    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color32::from_rgb(r, g, b))
}

/// Generate a rich tooltip string for an event.
/// Shows title, time range, location, and description preview.
pub fn format_event_tooltip(event: &Event) -> String {
    let mut lines = Vec::new();
    
    // Title (bold via unicode)
    lines.push(format!("📌 {}", event.title));
    
    // Time
    if event.all_day {
        let date_str = event.start.format("%A, %B %d, %Y").to_string();
        lines.push(format!("🕐 All day - {}", date_str));
    } else {
        let start_str = event.start.format("%H:%M").to_string();
        let end_str = event.end.format("%H:%M").to_string();
        let date_str = event.start.format("%A, %B %d").to_string();
        lines.push(format!("🕐 {} - {} ({})", start_str, end_str, date_str));
    }
    
    // Location
    if let Some(ref location) = event.location {
        if !location.is_empty() {
            lines.push(format!("📍 {}", location));
        }
    }
    
    // Category
    if let Some(ref category) = event.category {
        if !category.is_empty() {
            lines.push(format!("🏷️ {}", category));
        }
    }
    
    // Recurring indicator
    if event.recurrence_rule.is_some() {
        lines.push("🔄 Recurring event".to_string());
    }
    
    // Description preview (truncated)
    if let Some(ref description) = event.description {
        if !description.is_empty() {
            let preview = if description.len() > 100 {
                format!("{}...", &description[..100])
            } else {
                description.clone()
            };
            lines.push(format!("\n📝 {}", preview));
        }
    }
    
    // Add interaction hint
    lines.push("\n💡 Double-click to edit, right-click for more options".to_string());
    
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_get_week_start_sunday() {
        // Wednesday, Dec 4, 2024
        let date = NaiveDate::from_ymd_opt(2024, 12, 4).unwrap();
        // Week starts on Sunday (0)
        let start = get_week_start(date, 0);
        assert_eq!(start, NaiveDate::from_ymd_opt(2024, 12, 1).unwrap());
    }

    #[test]
    fn test_get_week_start_monday() {
        // Wednesday, Dec 4, 2024
        let date = NaiveDate::from_ymd_opt(2024, 12, 4).unwrap();
        // Week starts on Monday (1)
        let start = get_week_start(date, 1);
        assert_eq!(start, NaiveDate::from_ymd_opt(2024, 12, 2).unwrap());
    }

    #[test]
    fn test_parse_color_with_hash() {
        let color = parse_color("#FF5500").unwrap();
        assert_eq!(color, Color32::from_rgb(255, 85, 0));
    }

    #[test]
    fn test_parse_color_without_hash() {
        let color = parse_color("00FF00").unwrap();
        assert_eq!(color, Color32::from_rgb(0, 255, 0));
    }

    #[test]
    fn test_parse_color_invalid() {
        assert!(parse_color("").is_none());
        assert!(parse_color("FF5").is_none());
        assert!(parse_color("GGGGGG").is_none());
    }

    #[test]
    fn test_format_short_date_us() {
        let date = NaiveDate::from_ymd_opt(2024, 12, 4).unwrap();
        assert_eq!(format_short_date(date, "MM/DD/YYYY"), "12/04");
    }

    #[test]
    fn test_format_short_date_eu() {
        let date = NaiveDate::from_ymd_opt(2024, 12, 4).unwrap();
        assert_eq!(format_short_date(date, "DD/MM/YYYY"), "04/12");
    }

    #[test]
    fn test_format_short_date_iso() {
        let date = NaiveDate::from_ymd_opt(2024, 12, 4).unwrap();
        assert_eq!(format_short_date(date, "YYYY/MM/DD"), "2024/12/04");
    }

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
    fn test_shift_month_forward() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert_eq!(shift_month(date, 1), NaiveDate::from_ymd_opt(2024, 2, 15).unwrap());
        assert_eq!(shift_month(date, 12), NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());
    }

    #[test]
    fn test_shift_month_backward() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        assert_eq!(shift_month(date, -1), NaiveDate::from_ymd_opt(2024, 2, 15).unwrap());
        assert_eq!(shift_month(date, -3), NaiveDate::from_ymd_opt(2023, 12, 15).unwrap());
    }

    #[test]
    fn test_shift_month_clamp_day() {
        // Jan 31 -> Feb 29 (leap year)
        let date = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        assert_eq!(shift_month(date, 1), NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
        
        // Jan 31 -> Feb 28 (non-leap year)
        let date2 = NaiveDate::from_ymd_opt(2023, 1, 31).unwrap();
        assert_eq!(shift_month(date2, 1), NaiveDate::from_ymd_opt(2023, 2, 28).unwrap());
    }
}
