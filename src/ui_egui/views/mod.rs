use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::collections::HashSet;

use crate::models::event::Event;
use crate::services::calendar_sync::mapping::EventSyncMapService;
use crate::services::database::Database;

pub mod day_view;
pub mod month_view;
mod palette;
pub mod quarter_view;
pub mod week_shared;
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
    #[allow(dead_code)]
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

/// Filter events by category if a filter is active.
/// Returns only events whose category matches the filter.
/// If filter is None, all events pass through.
pub fn filter_events_by_category(events: Vec<Event>, filter: Option<&str>) -> Vec<Event> {
    match filter {
        None => events,
        Some(category) => events
            .into_iter()
            .filter(|e| e.category.as_deref() == Some(category))
            .collect(),
    }
}

pub fn load_synced_event_ids(database: &'static Database) -> HashSet<i64> {
    let service = EventSyncMapService::new(database.connection());
    match service.list_synced_local_event_ids_for_enabled_sources() {
        Ok(ids) => ids,
        Err(err) => {
            log::warn!("Failed to load synced event IDs: {}", err);
            HashSet::new()
        }
    }
}

pub fn is_synced_event(event_id: Option<i64>, synced_event_ids: &HashSet<i64>) -> bool {
    event_id
        .map(|id| synced_event_ids.contains(&id))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_event(title: &str, category: Option<&str>) -> Event {
        let start = Local.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2025, 1, 15, 11, 0, 0).unwrap();
        Event {
            id: None,
            title: title.to_string(),
            description: None,
            location: None,
            start,
            end,
            all_day: false,
            category: category.map(|s| s.to_string()),
            color: None,
            recurrence_rule: None,
            recurrence_exceptions: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn test_filter_events_no_filter_passes_all() {
        let events = vec![
            make_event("Work Event", Some("Work")),
            make_event("Personal Event", Some("Personal")),
            make_event("No Category", None),
        ];
        
        let result = filter_events_by_category(events, None);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_filter_events_with_category_filter() {
        let events = vec![
            make_event("Work Event 1", Some("Work")),
            make_event("Personal Event", Some("Personal")),
            make_event("Work Event 2", Some("Work")),
            make_event("No Category", None),
        ];
        
        let result = filter_events_by_category(events, Some("Work"));
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.category.as_deref() == Some("Work")));
    }

    #[test]
    fn test_filter_events_no_matches() {
        let events = vec![
            make_event("Work Event", Some("Work")),
            make_event("Personal Event", Some("Personal")),
        ];
        
        let result = filter_events_by_category(events, Some("Birthday"));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_filter_events_empty_list() {
        let events: Vec<Event> = vec![];
        
        let result = filter_events_by_category(events, Some("Work"));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_is_synced_event_detects_membership() {
        let mut synced = HashSet::new();
        synced.insert(42);

        assert!(is_synced_event(Some(42), &synced));
        assert!(!is_synced_event(Some(7), &synced));
        assert!(!is_synced_event(None, &synced));
    }
}
