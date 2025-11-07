// Recurrence module
// RFC 5545 iCalendar recurrence rule implementation

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Weekday};
use std::collections::HashSet;

/// Frequency of recurrence per RFC 5545
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frequency {
    Daily,
    Weekly,
    Fortnightly,  // Convenience alias for WEEKLY with INTERVAL=2
    Monthly,
    Quarterly,    // Convenience alias for MONTHLY with INTERVAL=3
    Yearly,
}

impl Frequency {
    /// Parse frequency from RRULE string (e.g., "DAILY", "WEEKLY")
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "DAILY" => Ok(Frequency::Daily),
            "WEEKLY" => Ok(Frequency::Weekly),
            "FORTNIGHTLY" => Ok(Frequency::Fortnightly),
            "MONTHLY" => Ok(Frequency::Monthly),
            "QUARTERLY" => Ok(Frequency::Quarterly),
            "YEARLY" => Ok(Frequency::Yearly),
            _ => Err(format!("Unknown frequency: {}", s)),
        }
    }

    /// Convert to RFC 5545 FREQ value
    pub fn to_rrule_string(&self) -> &'static str {
        match self {
            Frequency::Daily => "DAILY",
            Frequency::Weekly => "WEEKLY",
            Frequency::Fortnightly => "WEEKLY",
            Frequency::Monthly => "MONTHLY",
            Frequency::Quarterly => "MONTHLY",
            Frequency::Yearly => "YEARLY",
        }
    }

    /// Get the interval for convenience frequencies
    pub fn default_interval(&self) -> u32 {
        match self {
            Frequency::Fortnightly => 2,
            Frequency::Quarterly => 3,
            _ => 1,
        }
    }
}

/// Recurrence rule parser and calculator
#[derive(Debug, Clone)]
pub struct RecurrenceRule {
    pub frequency: Frequency,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<DateTime<Local>>,
    pub by_weekday: Vec<Weekday>,
    pub by_month_day: Vec<i32>,
    pub by_month: Vec<u32>,
}

impl RecurrenceRule {
    /// Create a simple recurrence rule
    pub fn new(frequency: Frequency) -> Self {
        Self {
            frequency,
            interval: frequency.default_interval(),
            count: None,
            until: None,
            by_weekday: Vec::new(),
            by_month_day: Vec::new(),
            by_month: Vec::new(),
        }
    }

    /// Parse an RRULE string (RFC 5545 format)
    /// Example: "FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE,FR;COUNT=10"
    pub fn from_rrule(rrule: &str) -> Result<Self, String> {
        let mut frequency = None;
        let mut interval = 1;
        let mut count = None;
        let mut until = None;
        let mut by_weekday = Vec::new();
        let mut by_month_day = Vec::new();
        let mut by_month = Vec::new();

        for part in rrule.split(';') {
            let parts: Vec<&str> = part.split('=').collect();
            if parts.len() != 2 {
                continue;
            }

            match parts[0] {
                "FREQ" => {
                    frequency = Some(Frequency::from_str(parts[1])?);
                }
                "INTERVAL" => {
                    interval = parts[1]
                        .parse()
                        .map_err(|_| format!("Invalid interval: {}", parts[1]))?;
                }
                "COUNT" => {
                    count = Some(
                        parts[1]
                            .parse()
                            .map_err(|_| format!("Invalid count: {}", parts[1]))?,
                    );
                }
                "UNTIL" => {
                    // Parse RFC 3339 datetime
                    until = Some(
                        DateTime::parse_from_rfc3339(parts[1])
                            .map_err(|_| format!("Invalid until date: {}", parts[1]))?
                            .with_timezone(&Local),
                    );
                }
                "BYDAY" => {
                    by_weekday = parts[1]
                        .split(',')
                        .filter_map(|s| Self::parse_weekday(s).ok())
                        .collect();
                }
                "BYMONTHDAY" => {
                    by_month_day = parts[1]
                        .split(',')
                        .filter_map(|s| s.parse().ok())
                        .collect();
                }
                "BYMONTH" => {
                    by_month = parts[1]
                        .split(',')
                        .filter_map(|s| s.parse().ok())
                        .collect();
                }
                _ => {}
            }
        }

        let frequency = frequency.ok_or_else(|| "FREQ is required".to_string())?;

        Ok(Self {
            frequency,
            interval,
            count,
            until,
            by_weekday,
            by_month_day,
            by_month,
        })
    }

    /// Convert to RRULE string
    pub fn to_rrule(&self) -> String {
        let mut parts = vec![format!("FREQ={}", self.frequency.to_rrule_string())];

        if self.interval != self.frequency.default_interval() {
            parts.push(format!("INTERVAL={}", self.interval));
        }

        if let Some(count) = self.count {
            parts.push(format!("COUNT={}", count));
        }

        if let Some(until) = &self.until {
            parts.push(format!("UNTIL={}", until.to_rfc3339()));
        }

        if !self.by_weekday.is_empty() {
            let days: Vec<String> = self
                .by_weekday
                .iter()
                .map(|d| Self::weekday_to_rrule(*d))
                .collect();
            parts.push(format!("BYDAY={}", days.join(",")));
        }

        if !self.by_month_day.is_empty() {
            parts.push(format!(
                "BYMONTHDAY={}",
                self.by_month_day
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }

        if !self.by_month.is_empty() {
            parts.push(format!(
                "BYMONTH={}",
                self.by_month
                    .iter()
                    .map(|m| m.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }

        parts.join(";")
    }

    /// Generate occurrences for a date range
    /// Returns a list of dates when the event occurs, excluding exception dates
    pub fn generate_occurrences(
        &self,
        start: DateTime<Local>,
        range_start: DateTime<Local>,
        range_end: DateTime<Local>,
        exceptions: &[DateTime<Local>],
    ) -> Vec<DateTime<Local>> {
        let mut occurrences = Vec::new();
        let exception_set: HashSet<NaiveDate> = exceptions
            .iter()
            .map(|dt| dt.date_naive())
            .collect();

        let mut current = start;
        let mut iteration = 0;
        let max_iterations = 10000; // Safety limit

        while iteration < max_iterations {
            // Check if we've reached the count limit
            if let Some(count) = self.count {
                if occurrences.len() >= count as usize {
                    break;
                }
            }

            // Check if we've passed the until date
            if let Some(until) = self.until {
                if current > until {
                    break;
                }
            }

            // Check if we've passed the range end
            if current > range_end {
                break;
            }

            // Check if this occurrence is in range and not an exception
            if current >= range_start && !exception_set.contains(&current.date_naive()) {
                // Apply filters
                if self.matches_filters(&current) {
                    occurrences.push(current);
                }
            }

            // Move to next occurrence
            current = self.next_occurrence(current);
            iteration += 1;
        }

        occurrences
    }

    /// Check if a date matches all filter criteria
    fn matches_filters(&self, date: &DateTime<Local>) -> bool {
        // Check BYDAY filter
        if !self.by_weekday.is_empty() {
            if !self.by_weekday.contains(&date.weekday()) {
                return false;
            }
        }

        // Check BYMONTHDAY filter
        if !self.by_month_day.is_empty() {
            if !self.by_month_day.contains(&(date.day() as i32)) {
                return false;
            }
        }

        // Check BYMONTH filter
        if !self.by_month.is_empty() {
            if !self.by_month.contains(&date.month()) {
                return false;
            }
        }

        true
    }

    /// Calculate the next occurrence after the given date
    fn next_occurrence(&self, current: DateTime<Local>) -> DateTime<Local> {
        match self.frequency {
            Frequency::Daily => current + Duration::days(self.interval as i64),
            Frequency::Weekly | Frequency::Fortnightly => {
                current + Duration::weeks(self.interval as i64)
            }
            Frequency::Monthly | Frequency::Quarterly => {
                Self::add_months(current, self.interval as i32)
            }
            Frequency::Yearly => Self::add_months(current, 12 * self.interval as i32),
        }
    }

    /// Add months to a date (handles month/year overflow correctly)
    fn add_months(date: DateTime<Local>, months: i32) -> DateTime<Local> {
        let mut year = date.year();
        let mut month = date.month() as i32 + months;

        while month > 12 {
            month -= 12;
            year += 1;
        }
        while month < 1 {
            month += 12;
            year -= 1;
        }

        // Handle day overflow (e.g., Jan 31 + 1 month = Feb 28/29)
        let max_day = Self::days_in_month(year, month as u32);
        let day = date.day().min(max_day);

        // Create new naive date with clamped day
        let new_naive_date = NaiveDate::from_ymd_opt(year, month as u32, day)
            .unwrap_or_else(|| date.date_naive());
        
        // Combine with original time
        let new_naive_datetime = new_naive_date.and_time(date.time());
        
        // Convert to Local timezone
        Local.from_local_datetime(&new_naive_datetime)
            .earliest()
            .unwrap_or(date)
    }

    /// Get number of days in a month
    fn days_in_month(year: i32, month: u32) -> u32 {
        NaiveDate::from_ymd_opt(year, month, 1)
            .and_then(|d| {
                if month == 12 {
                    NaiveDate::from_ymd_opt(year + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(year, month + 1, 1)
                }
                .map(|next| (next - d).num_days() as u32)
            })
            .unwrap_or(30)
    }

    /// Parse weekday from RRULE format (MO, TU, WE, TH, FR, SA, SU)
    fn parse_weekday(s: &str) -> Result<Weekday, String> {
        match s.to_uppercase().as_str() {
            "MO" => Ok(Weekday::Mon),
            "TU" => Ok(Weekday::Tue),
            "WE" => Ok(Weekday::Wed),
            "TH" => Ok(Weekday::Thu),
            "FR" => Ok(Weekday::Fri),
            "SA" => Ok(Weekday::Sat),
            "SU" => Ok(Weekday::Sun),
            _ => Err(format!("Invalid weekday: {}", s)),
        }
    }

    /// Convert weekday to RRULE format
    fn weekday_to_rrule(day: Weekday) -> String {
        match day {
            Weekday::Mon => "MO",
            Weekday::Tue => "TU",
            Weekday::Wed => "WE",
            Weekday::Thu => "TH",
            Weekday::Fri => "FR",
            Weekday::Sat => "SA",
            Weekday::Sun => "SU",
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_frequency_from_str() {
        assert_eq!(Frequency::from_str("DAILY").unwrap(), Frequency::Daily);
        assert_eq!(Frequency::from_str("weekly").unwrap(), Frequency::Weekly);
        assert_eq!(Frequency::from_str("MONTHLY").unwrap(), Frequency::Monthly);
        assert!(Frequency::from_str("INVALID").is_err());
    }

    #[test]
    fn test_frequency_to_rrule_string() {
        assert_eq!(Frequency::Daily.to_rrule_string(), "DAILY");
        assert_eq!(Frequency::Weekly.to_rrule_string(), "WEEKLY");
        assert_eq!(Frequency::Fortnightly.to_rrule_string(), "WEEKLY");
        assert_eq!(Frequency::Monthly.to_rrule_string(), "MONTHLY");
        assert_eq!(Frequency::Quarterly.to_rrule_string(), "MONTHLY");
        assert_eq!(Frequency::Yearly.to_rrule_string(), "YEARLY");
    }

    #[test]
    fn test_parse_simple_rrule() {
        let rule = RecurrenceRule::from_rrule("FREQ=DAILY").unwrap();
        assert_eq!(rule.frequency, Frequency::Daily);
        assert_eq!(rule.interval, 1);
        assert!(rule.count.is_none());
    }

    #[test]
    fn test_parse_complex_rrule() {
        let rule = RecurrenceRule::from_rrule("FREQ=WEEKLY;INTERVAL=2;COUNT=10;BYDAY=MO,WE,FR")
            .unwrap();
        assert_eq!(rule.frequency, Frequency::Weekly);
        assert_eq!(rule.interval, 2);
        assert_eq!(rule.count, Some(10));
        assert_eq!(rule.by_weekday.len(), 3);
    }

    #[test]
    fn test_to_rrule_basic() {
        let rule = RecurrenceRule::new(Frequency::Daily);
        assert_eq!(rule.to_rrule(), "FREQ=DAILY");
    }

    #[test]
    fn test_to_rrule_with_interval() {
        let mut rule = RecurrenceRule::new(Frequency::Weekly);
        rule.interval = 2;
        assert_eq!(rule.to_rrule(), "FREQ=WEEKLY;INTERVAL=2");
    }

    #[test]
    fn test_to_rrule_with_count() {
        let mut rule = RecurrenceRule::new(Frequency::Daily);
        rule.count = Some(5);
        assert_eq!(rule.to_rrule(), "FREQ=DAILY;COUNT=5");
    }

    #[test]
    fn test_generate_daily_occurrences() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let range_start = start;
        let range_end = start + Duration::days(5);

        let rule = RecurrenceRule::new(Frequency::Daily);
        let occurrences = rule.generate_occurrences(start, range_start, range_end, &[]);

        assert_eq!(occurrences.len(), 6); // Days 1-6
    }

    #[test]
    fn test_generate_weekly_occurrences() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let range_start = start;
        let range_end = start + Duration::weeks(4);

        let rule = RecurrenceRule::new(Frequency::Weekly);
        let occurrences = rule.generate_occurrences(start, range_start, range_end, &[]);

        assert_eq!(occurrences.len(), 5); // Weeks 0-4
    }

    #[test]
    fn test_generate_fortnightly_occurrences() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let range_start = start;
        let range_end = start + Duration::weeks(8);

        let rule = RecurrenceRule::new(Frequency::Fortnightly);
        let occurrences = rule.generate_occurrences(start, range_start, range_end, &[]);

        assert_eq!(occurrences.len(), 5); // Every 2 weeks for 8 weeks
    }

    #[test]
    fn test_generate_monthly_occurrences() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let range_start = start;
        let range_end = Local.with_ymd_and_hms(2025, 6, 1, 10, 0, 0).unwrap();

        let rule = RecurrenceRule::new(Frequency::Monthly);
        let occurrences = rule.generate_occurrences(start, range_start, range_end, &[]);

        assert_eq!(occurrences.len(), 6); // Jan-Jun
    }

    #[test]
    fn test_generate_with_count_limit() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let range_start = start;
        let range_end = start + Duration::days(365);

        let mut rule = RecurrenceRule::new(Frequency::Daily);
        rule.count = Some(5);
        let occurrences = rule.generate_occurrences(start, range_start, range_end, &[]);

        assert_eq!(occurrences.len(), 5);
    }

    #[test]
    fn test_generate_with_until() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let range_start = start;
        let range_end = start + Duration::days(365);

        let mut rule = RecurrenceRule::new(Frequency::Daily);
        rule.until = Some(start + Duration::days(4));
        let occurrences = rule.generate_occurrences(start, range_start, range_end, &[]);

        assert_eq!(occurrences.len(), 5); // Days 0-4
    }

    #[test]
    fn test_generate_with_exceptions() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let range_start = start;
        let range_end = start + Duration::days(5);

        let exceptions = vec![
            start + Duration::days(2),
            start + Duration::days(4),
        ];

        let rule = RecurrenceRule::new(Frequency::Daily);
        let occurrences = rule.generate_occurrences(start, range_start, range_end, &exceptions);

        assert_eq!(occurrences.len(), 4); // 6 days - 2 exceptions
    }

    #[test]
    fn test_generate_with_weekday_filter() {
        let start = Local.with_ymd_and_hms(2025, 1, 6, 10, 0, 0).unwrap(); // Monday
        let range_start = start;
        let range_end = start + Duration::weeks(2) - Duration::days(1); // Up to Sunday Jan 19

        let mut rule = RecurrenceRule::new(Frequency::Daily);
        rule.by_weekday = vec![Weekday::Mon, Weekday::Wed, Weekday::Fri];
        let occurrences = rule.generate_occurrences(start, range_start, range_end, &[]);

        // Jan 6 (Mon), Jan 8 (Wed), Jan 10 (Fri), Jan 13 (Mon), Jan 15 (Wed), Jan 17 (Fri)
        assert_eq!(occurrences.len(), 6);
    }

    #[test]
    fn test_add_months() {
        let date = Local.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();

        let result = RecurrenceRule::add_months(date, 1);
        assert_eq!(result.month(), 2);
        assert_eq!(result.day(), 15);

        let result = RecurrenceRule::add_months(date, 12);
        assert_eq!(result.year(), 2026);
        assert_eq!(result.month(), 1);
    }

    #[test]
    fn test_add_months_overflow_day() {
        let date = Local.with_ymd_and_hms(2025, 1, 31, 10, 0, 0).unwrap();

        let result = RecurrenceRule::add_months(date, 1);
        assert_eq!(result.year(), 2025);
        assert_eq!(result.month(), 2);
        // Feb 2025 has 28 days, so Jan 31 + 1 month = Feb 28
        assert!(result.day() <= 28);
    }

    #[test]
    fn test_quarterly_frequency() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let range_start = start;
        let range_end = Local.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();

        let rule = RecurrenceRule::new(Frequency::Quarterly);
        let occurrences = rule.generate_occurrences(start, range_start, range_end, &[]);

        assert_eq!(occurrences.len(), 5); // Jan, Apr, Jul, Oct 2025, Jan 2026
    }

    #[test]
    fn test_yearly_frequency() {
        let start = Local.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        let range_start = start;
        let range_end = Local.with_ymd_and_hms(2028, 1, 1, 10, 0, 0).unwrap();

        let rule = RecurrenceRule::new(Frequency::Yearly);
        let occurrences = rule.generate_occurrences(start, range_start, range_end, &[]);

        assert_eq!(occurrences.len(), 4); // 2025, 2026, 2027, 2028
    }
}
