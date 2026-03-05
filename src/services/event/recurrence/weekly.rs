use crate::models::event::Event;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate};

use super::parser::{parse_interval, parse_weekly_byday};
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
    let byday_days = parse_weekly_byday(rrule, event.start.weekday());

    if byday_days.is_empty() {
        return occurrences;
    }

    let mut current_week_start = event.start.date_naive()
        - Duration::days(event.start.weekday().num_days_from_monday() as i64);
    let week_start_time = event.start.time();
    let mut occurrence_count = 0usize;

    loop {
        if let Some(max) = max_count {
            if occurrence_count >= max {
                break;
            }
        }

        if let Some(until) = until_date {
            if current_week_start > until {
                break;
            }
        }

        for &target_weekday in &byday_days {
            let days_offset = target_weekday.num_days_from_monday() as i64;
            let occurrence_date = current_week_start + Duration::days(days_offset);

            if let Some(until) = until_date {
                if occurrence_date > until {
                    continue;
                }
            }

            if let Some(occurrence_datetime) = occurrence_date
                .and_time(week_start_time)
                .and_local_timezone(Local)
                .single()
            {
                if occurrence_datetime >= event.start
                    && is_valid_occurrence(event, occurrence_datetime)
                {
                    if let Some(max) = max_count {
                        if occurrence_count >= max {
                            break;
                        }
                    }

                    push_if_in_range(
                        &mut occurrences,
                        event,
                        occurrence_datetime,
                        duration,
                        range_start,
                        range_end,
                    );
                    occurrence_count += 1;
                }
            }
        }

        current_week_start += Duration::weeks(interval);

        if current_week_start > range_end.date_naive() + Duration::days(365) {
            break;
        }
    }

    occurrences
}

#[cfg(test)]
mod tests {
    use super::generate;
    use crate::models::event::Event;
    use chrono::{Datelike, Duration, Local, TimeZone, Weekday};

    #[test]
    fn weekly_count_limits_total_occurrences_not_weeks() {
        let start = Local.with_ymd_and_hms(2026, 3, 2, 9, 0, 0).unwrap(); // Monday
        let end = start + Duration::hours(1);
        let event = Event::new("Weekly", start, end).unwrap();

        let occurrences = generate(
            &event,
            "FREQ=WEEKLY;BYDAY=MO,WE",
            start,
            start + Duration::days(21),
            Duration::hours(1),
            Some(2),
            None,
        );

        assert_eq!(occurrences.len(), 2);
        assert_eq!(occurrences[0].start.weekday(), Weekday::Mon);
        assert_eq!(occurrences[1].start.weekday(), Weekday::Wed);
    }
}
