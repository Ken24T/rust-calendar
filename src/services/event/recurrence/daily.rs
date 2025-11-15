use crate::models::event::Event;
use chrono::{DateTime, Duration, Local, NaiveDate};

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
    let interval = parse_interval(rrule, 1);
    let mut current_start = event.start;
    let mut occurrence_count = 0usize;
    let limit = event.end.max(range_end);

    while current_start <= limit {
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

        current_start = current_start + Duration::days(interval);

        if current_start > range_end + Duration::days(365) {
            break;
        }
    }

    occurrences
}
