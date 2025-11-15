use anyhow::Result;
use chrono::{DateTime, Local, TimeZone};

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
    let s = s.trim_end_matches('Z');

    if s.len() < 15 {
        return Err(anyhow::anyhow!("Invalid datetime format: {}", s));
    }

    let year: i32 = s[0..4].parse()?;
    let month: u32 = s[4..6].parse()?;
    let day: u32 = s[6..8].parse()?;
    let hour: u32 = s[9..11].parse()?;
    let minute: u32 = s[11..13].parse()?;
    let second: u32 = s[13..15].parse()?;

    Local
        .with_ymd_and_hms(year, month, day, hour, minute, second)
        .single()
        .ok_or_else(|| anyhow::anyhow!("Invalid datetime: {}", s))
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
