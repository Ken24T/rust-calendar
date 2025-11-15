use crate::models::event::Event;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Weekday};

pub(super) fn is_valid_occurrence(event: &Event, occurrence_start: DateTime<Local>) -> bool {
    if occurrence_start < event.start {
        return false;
    }

    if let Some(ref exceptions) = event.recurrence_exceptions {
        if exceptions
            .iter()
            .any(|ex| ex.date_naive() == occurrence_start.date_naive())
        {
            return false;
        }
    }

    true
}

pub(super) fn push_if_in_range(
    occurrences: &mut Vec<Event>,
    event: &Event,
    start: DateTime<Local>,
    duration: Duration,
    range_start: DateTime<Local>,
    range_end: DateTime<Local>,
) {
    if start >= range_start && start <= range_end {
        let mut occurrence = event.clone();
        occurrence.start = start;
        occurrence.end = start + duration;
        occurrences.push(occurrence);
    }
}

pub(super) fn advance_month(current_date: NaiveDate, interval: i64) -> NaiveDate {
    let new_month = current_date.month() as i64 + interval;
    let years_to_add = (new_month - 1) / 12;
    let final_month = ((new_month - 1) % 12 + 1) as u32;
    let final_year = current_date.year() as i64 + years_to_add;

    NaiveDate::from_ymd_opt(final_year as i32, final_month, current_date.day().min(28))
        .unwrap_or(current_date + Duration::days(30))
}

pub(super) fn select_month_boundary(current_date: NaiveDate, flag: i32) -> Option<NaiveDate> {
    if flag == 1 {
        NaiveDate::from_ymd_opt(current_date.year(), current_date.month(), 1)
    } else if flag == -1 {
        let next = advance_month(current_date, 1);
        next.pred_opt()
    } else {
        Some(current_date)
    }
}

pub(super) fn select_positional_weekday(
    current_date: NaiveDate,
    position: i32,
    weekday: Weekday,
) -> Option<NaiveDate> {
    if position == 1 {
        let first = NaiveDate::from_ymd_opt(current_date.year(), current_date.month(), 1)?;
        let first_weekday = first.weekday();
        let days_until_target =
            ((weekday.num_days_from_monday() as i32 - first_weekday.num_days_from_monday() as i32 + 7)
                % 7) as i64;
        Some(first + Duration::days(days_until_target))
    } else if position == -1 {
        let next = advance_month(current_date, 1);
        let last = next.pred_opt()?;
        let last_weekday = last.weekday();
        let days_back_to_target =
            ((last_weekday.num_days_from_monday() as i32 - weekday.num_days_from_monday() as i32 + 7)
                % 7) as i64;
        Some(last - Duration::days(days_back_to_target))
    } else {
        None
    }
}