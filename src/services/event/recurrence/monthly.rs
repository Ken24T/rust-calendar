use crate::models::event::Event;
use chrono::Datelike;
use chrono::{DateTime, Duration, Local, NaiveDate};

use super::parser::{parse_bymonthday, parse_interval, parse_monthly_byday};
use super::utils::{
    advance_month, all_weekdays_in_month, is_valid_occurrence, push_if_in_range,
    select_month_boundary, select_positional_weekday,
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
    let monthly_byday = parse_monthly_byday(rrule);

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

        let occurrence_dates = if let Some(day) = bymonthday {
            select_month_boundary(current_date, day)
                .into_iter()
                .collect::<Vec<_>>()
        } else if !monthly_byday.is_empty() {
            let mut dates = Vec::new();
            for entry in &monthly_byday {
                if let Some(position) = entry.position {
                    if let Some(date) =
                        select_positional_weekday(current_date, position, entry.weekday)
                    {
                        dates.push(date);
                    }
                } else {
                    dates.extend(all_weekdays_in_month(current_date, entry.weekday));
                }
            }

            dates.sort_unstable();
            dates.dedup();
            dates
        } else {
            vec![current_date]
        };

        for occ_date in occurrence_dates {
            if let Some(until) = until_date {
                if occ_date > until {
                    continue;
                }
            }

            if let Some(max) = max_count {
                if occurrence_count >= max {
                    break;
                }
            }

            if occ_date.month() != current_date.month() || occ_date.year() != current_date.year() {
                continue;
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

#[cfg(test)]
mod tests {
    use super::generate;
    use crate::models::event::Event;
    use chrono::{Datelike, Duration, Local, TimeZone, Weekday};

    #[test]
    fn monthly_byday_supports_multiple_non_positional_weekdays() {
        let start = Local.with_ymd_and_hms(2026, 1, 1, 9, 0, 0).unwrap();
        let event = Event::new("Monthly", start, start + Duration::hours(1)).unwrap();

        let occurrences = generate(
            &event,
            "FREQ=MONTHLY;BYDAY=MO,WE",
            start,
            start + Duration::days(40),
            Duration::hours(1),
            Some(4),
            None,
        );

        assert_eq!(occurrences.len(), 4);
        assert_eq!(occurrences[0].start.weekday(), Weekday::Mon);
        assert_eq!(occurrences[1].start.weekday(), Weekday::Wed);
        assert_eq!(occurrences[2].start.weekday(), Weekday::Mon);
        assert_eq!(occurrences[3].start.weekday(), Weekday::Wed);
    }

    #[test]
    fn monthly_byday_supports_multiple_positional_tokens() {
        let start = Local.with_ymd_and_hms(2026, 1, 1, 9, 0, 0).unwrap();
        let event = Event::new("Monthly", start, start + Duration::hours(1)).unwrap();

        let occurrences = generate(
            &event,
            "FREQ=MONTHLY;BYDAY=1MO,-1FR",
            start,
            start + Duration::days(70),
            Duration::hours(1),
            Some(4),
            None,
        );

        assert_eq!(occurrences.len(), 4);
        assert_eq!(
            occurrences[0].start.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()
        );
        assert_eq!(
            occurrences[1].start.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2026, 1, 30).unwrap()
        );
        assert_eq!(
            occurrences[2].start.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2026, 2, 2).unwrap()
        );
        assert_eq!(
            occurrences[3].start.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2026, 2, 27).unwrap()
        );
    }
}
