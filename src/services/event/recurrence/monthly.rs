use crate::models::event::Event;
use chrono::{DateTime, Duration, Local, NaiveDate};

use super::parser::{parse_bymonthday, parse_interval, parse_positional_byday};
use super::utils::{
    advance_month,
    is_valid_occurrence,
    push_if_in_range,
    select_month_boundary,
    select_positional_weekday,
};

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
    let bymonthday = parse_bymonthday(rrule);
    let positional_byday = parse_positional_byday(rrule);

    let mut current_date = event.start.date_naive();
    let event_time = event.start.time();
    let mut occurrence_count = 0usize;

    loop {
        if let Some(max) = max_count {
            if occurrence_count >= max {
                break;
            }
        }

        if let Some(until) = until_date {
            if current_date > until {
                break;
            }
        }

        let occurrence_date = if let Some(day) = bymonthday {
            select_month_boundary(current_date, day)
        } else if let Some((position, weekday)) = positional_byday {
            select_positional_weekday(current_date, position, weekday)
        } else {
            Some(current_date)
        };

        if let Some(occ_date) = occurrence_date {
            if let Some(until) = until_date {
                if occ_date > until {
                    break;
                }
            }

            if let Some(occurrence_datetime) = occ_date
                .and_time(event_time)
                .and_local_timezone(Local)
                .single()
            {
                if occurrence_datetime >= event.start
                    && is_valid_occurrence(event, occurrence_datetime)
                {
                    occurrence_count += 1;
                    push_if_in_range(
                        &mut occurrences,
                        event,
                        occurrence_datetime,
                        duration,
                        range_start,
                        range_end,
                    );
                }
            }
        }

        current_date = advance_month(current_date, interval);

        if current_date > range_end.date_naive() + Duration::days(365) {
            break;
        }
    }

    occurrences
}
