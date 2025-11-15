use chrono::{NaiveDate, Weekday};

#[derive(Clone, Copy)]
pub(super) enum RecurrenceFrequency {
    Weekly,
    Monthly,
    Yearly,
    Daily,
}

pub(super) fn detect_frequency(rrule: &str) -> RecurrenceFrequency {
    if rrule.contains("FREQ=WEEKLY") {
        RecurrenceFrequency::Weekly
    } else if rrule.contains("FREQ=MONTHLY") {
        RecurrenceFrequency::Monthly
    } else if rrule.contains("FREQ=YEARLY") {
        RecurrenceFrequency::Yearly
    } else {
        RecurrenceFrequency::Daily
    }
}

pub(super) fn parse_count(rrule: &str) -> Option<usize> {
    rrule.find("COUNT=").and_then(|idx| {
        let count_str = &rrule[idx + 6..];
        let end = count_str.find(';').unwrap_or(count_str.len());
        count_str[..end].parse::<usize>().ok()
    })
}

pub(super) fn parse_until(rrule: &str) -> Option<NaiveDate> {
    rrule.find("UNTIL=").and_then(|idx| {
        let slice = &rrule[idx + 6..];
        let end = slice.find(';').unwrap_or(slice.len());
        let date_str = &slice[..end];
        if date_str.len() == 8 {
            match (
                date_str[0..4].parse::<i32>(),
                date_str[4..6].parse::<u32>(),
                date_str[6..8].parse::<u32>(),
            ) {
                (Ok(year), Ok(month), Ok(day)) => NaiveDate::from_ymd_opt(year, month, day),
                _ => None,
            }
        } else {
            None
        }
    })
}

pub(super) fn parse_interval(rrule: &str, default: i64) -> i64 {
    rrule
        .find("INTERVAL=")
        .and_then(|idx| {
            let slice = &rrule[idx + 9..];
            let end = slice.find(';').unwrap_or(slice.len());
            slice[..end].parse::<i64>().ok()
        })
        .unwrap_or(default)
}

pub(super) fn parse_weekly_byday(rrule: &str, fallback: Weekday) -> Vec<Weekday> {
    rrule
        .find("BYDAY=")
        .map(|idx| {
            let slice = &rrule[idx + 6..];
            let end = slice.find(';').unwrap_or(slice.len());
            let values = &slice[..end];
            values
                .split(',')
                .filter_map(|code| weekday_from_code(code.trim()))
                .collect::<Vec<_>>()
        })
        .filter(|days| !days.is_empty())
        .unwrap_or_else(|| vec![fallback])
}

pub(super) fn parse_bymonthday(rrule: &str) -> Option<i32> {
    rrule.find("BYMONTHDAY=").and_then(|idx| {
        let slice = &rrule[idx + 11..];
        let end = slice.find(';').unwrap_or(slice.len());
        slice[..end].parse::<i32>().ok()
    })
}

pub(super) fn parse_positional_byday(rrule: &str) -> Option<(i32, Weekday)> {
    rrule.find("BYDAY=").and_then(|idx| {
        let slice = &rrule[idx + 6..];
        let end = slice.find(';').unwrap_or(slice.len());
        let day_str = &slice[..end];
        if day_str.len() > 2 {
            let (position, weekday_code) = day_str.split_at(day_str.len() - 2);
            let weekday = weekday_from_code(weekday_code)?;
            let pos = position.parse::<i32>().ok()?;
            Some((pos, weekday))
        } else {
            None
        }
    })
}
fn weekday_from_code(code: &str) -> Option<Weekday> {
    match code {
        "SU" => Some(Weekday::Sun),
        "MO" => Some(Weekday::Mon),
        "TU" => Some(Weekday::Tue),
        "WE" => Some(Weekday::Wed),
        "TH" => Some(Weekday::Thu),
        "FR" => Some(Weekday::Fri),
        "SA" => Some(Weekday::Sat),
        _ => None,
    }
}
