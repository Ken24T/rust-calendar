use anyhow::Result;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use std::str::FromStr;

pub(super) fn format_datetime(dt: &DateTime<Local>) -> String {
    dt.format("%Y%m%dT%H%M%S").to_string()
}

pub(super) fn format_date(dt: &DateTime<Local>) -> String {
    dt.format("%Y%m%d").to_string()
}

pub(super) fn escape_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(',', "\\,")
        .replace(';', "\\;")
}

pub(super) fn unescape_text(text: &str) -> String {
    text.replace("\\n", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

pub(super) fn parse_datetime(s: &str) -> Result<DateTime<Local>> {
    parse_datetime_with_tzid(s, None)
}

pub(super) fn parse_datetime_with_tzid(s: &str, tzid: Option<&str>) -> Result<DateTime<Local>> {
    let has_utc_suffix = s.ends_with('Z');
    let normalized = s.trim_end_matches('Z');

    if normalized.len() < 15 {
        return Err(anyhow::anyhow!("Invalid datetime format: {}", normalized));
    }

    let year: i32 = normalized[0..4].parse()?;
    let month: u32 = normalized[4..6].parse()?;
    let day: u32 = normalized[6..8].parse()?;
    let hour: u32 = normalized[9..11].parse()?;
    let minute: u32 = normalized[11..13].parse()?;
    let second: u32 = normalized[13..15].parse()?;

    let naive = NaiveDateTime::new(
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| anyhow::anyhow!("Invalid date: {}", normalized))?,
        chrono::NaiveTime::from_hms_opt(hour, minute, second)
            .ok_or_else(|| anyhow::anyhow!("Invalid time: {}", normalized))?,
    );

    if has_utc_suffix {
        let utc = Utc.from_utc_datetime(&naive);
        return Ok(utc.with_timezone(&Local));
    }

    if let Some(tz_name) = tzid {
        if let Ok(timezone) = Tz::from_str(tz_name) {
            if let Some(dt) = timezone.from_local_datetime(&naive).single() {
                return Ok(dt.with_timezone(&Local));
            }
        }
    }

    Local
        .with_ymd_and_hms(year, month, day, hour, minute, second)
        .single()
        .ok_or_else(|| anyhow::anyhow!("Invalid datetime: {}", normalized))
}

pub(super) fn parse_date(s: &str) -> Result<DateTime<Local>> {
    if s.len() < 8 {
        return Err(anyhow::anyhow!("Invalid date format: {}", s));
    }

    let year: i32 = s[0..4].parse()?;
    let month: u32 = s[4..6].parse()?;
    let day: u32 = s[6..8].parse()?;

    Local
        .with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .ok_or_else(|| anyhow::anyhow!("Invalid date: {}", s))
}

#[cfg(test)]
mod tests {
    use super::{parse_datetime, parse_datetime_with_tzid};
    use chrono::Timelike;

    #[test]
    fn test_parse_datetime_utc_suffix() {
        let parsed = parse_datetime("20260227T010203Z").unwrap();
        assert_eq!(parsed.second(), 3);
    }

    #[test]
    fn test_parse_datetime_with_tzid() {
        let parsed = parse_datetime_with_tzid("20260227T090000", Some("Australia/Sydney"));
        assert!(parsed.is_ok());
    }
}
