use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use crate::models::event::Event;

pub mod day_view;
pub mod month_view;
mod palette;
pub mod quarter_view;
pub mod week_view;
pub mod workweek_view;

#[derive(Clone, Debug)]
pub struct CountdownRequest {
    pub event_id: Option<i64>,
    pub title: String,
    pub start_at: DateTime<Local>,
    pub end_at: DateTime<Local>,
    pub color: Option<String>,
    pub body: Option<String>,
    pub display_label: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct AutoFocusRequest {
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
}

impl AutoFocusRequest {
    pub fn from_event(event: &Event) -> Self {
        Self {
            date: event.start.date_naive(),
            time: (!event.all_day).then(|| event.start.time()),
        }
    }

    pub fn matches_slot(
        &self,
        date: NaiveDate,
        slot_start: NaiveTime,
        slot_end: NaiveTime,
    ) -> bool {
        if self.date != date {
            return false;
        }

        let target_time = self
            .time
            .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());

        let target_secs = target_time.num_seconds_from_midnight();
        let slot_start_secs = slot_start.num_seconds_from_midnight();
        let slot_end_secs = slot_end.num_seconds_from_midnight();

        // slot_end for final slot can be 23:59:59, so treat it as inclusive.
        if slot_start_secs <= target_secs && target_secs < slot_end_secs {
            return true;
        }

        slot_end_secs == target_secs
    }
}

/// Returns the start/end timestamps for the portion of `event` that should appear on `date`.
/// This clamps long multi-day events to the original time-of-day window so they only fill
/// the slots that correspond to their configured duration.
pub fn event_time_segment_for_date(
    event: &Event,
    date: NaiveDate,
) -> Option<(NaiveDateTime, NaiveDateTime)> {
    let event_start = event.start.naive_local();
    let event_end = event.end.naive_local();

    if date < event_start.date() || date > event_end.date() {
        return None;
    }

    if event_start.date() == event_end.date() {
        return if date == event_start.date() {
            Some((event_start, event_end))
        } else {
            None
        };
    }

    let day_start = date.and_hms_opt(0, 0, 0).unwrap();
    let day_end = day_start + Duration::days(1);

    let segment_start = event_start.max(day_start);
    let segment_end = event_end.min(day_end);

    let mut adjusted_start = segment_start;
    let mut adjusted_end = segment_end;

    if event_end.time() >= event_start.time() {
        let daily_start = date.and_time(event_start.time());
        let daily_end = date.and_time(event_end.time());
        adjusted_start = adjusted_start.max(daily_start);
        adjusted_end = adjusted_end.min(daily_end);
    }

    if adjusted_start < adjusted_end {
        Some((adjusted_start, adjusted_end))
    } else if segment_start < segment_end {
        Some((segment_start, segment_end))
    } else {
        None
    }
}

impl CountdownRequest {
    pub fn from_event(event: &Event) -> Self {
        let location_label = event
            .location
            .as_deref()
            .map(str::trim)
            .filter(|loc| !loc.is_empty())
            .map(|loc| loc.to_string());

        Self {
            event_id: event.id,
            title: event.title.clone(),
            start_at: event.start,
            end_at: event.end,
            color: event.color.clone(),
            body: event.description.clone(),
            display_label: location_label,
        }
    }
}
