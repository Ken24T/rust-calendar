use crate::models::event::Event;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate};

use super::parser::parse_interval;
use super::utils::{is_valid_occurrence, push_if_in_range};

pub(super) fn generate(
    event: &Event,
    rrule: &str,
    range_start: DateTime<Local>,
    range_end: DateTime<Local>,
    duration: Duration,
    max_count: Option<usize>,
    until_date: Option<NaiveDate>,
) -> Vec<Event> {
    let mut occurrences = Vec::new();
    let interval = parse_interval(rrule, 1) as i32;
    let mut current_start = event.start;
    let mut occurrence_count = 0usize;

    loop {
        if let Some(max) = max_count {
            if occurrence_count >= max {
                break;
            }
        }

        if let Some(until) = until_date {
            if current_start.date_naive() > until {
                break;
            }
        }

        if is_valid_occurrence(event, current_start) {
            occurrence_count += 1;
            push_if_in_range(
                &mut occurrences,
                event,
                current_start,
                duration,
                range_start,
                range_end,
            );
        }

        current_start = advance_year_with_day_clamp(current_start, interval);

        if current_start > range_end + Duration::days(365) {
            break;
        }
    }

    occurrences
}

fn advance_year_with_day_clamp(
    current_start: DateTime<Local>,
    year_interval: i32,
) -> DateTime<Local> {
    let target_year = current_start.year() + year_interval;
    let month = current_start.month();
    let day = current_start.day();
    let time = current_start.time();

    // Keep month/time stable and clamp day to the target month's valid range.
    let mut clamped_day = day;
    while clamped_day > 28 {
        if let Some(candidate) = NaiveDate::from_ymd_opt(target_year, month, clamped_day)
            .and_then(|date| date.and_time(time).and_local_timezone(Local).single())
        {
            return candidate;
        }
        clamped_day -= 1;
    }

    current_start + Duration::days(365 * year_interval as i64)
}
