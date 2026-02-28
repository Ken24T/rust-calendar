use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::collections::HashSet;

use crate::models::event::Event;
use crate::services::calendar_sync::mapping::EventSyncMapService;
use crate::services::database::Database;

mod day_context_menu;
pub mod day_view;
mod day_event_rendering;
mod event_rendering;
mod month_context_menu;
mod month_day_cell;
pub mod month_view;
mod palette;
pub mod quarter_view;
mod time_grid;
mod time_grid_context_menu;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CountdownMenuState {
    Hidden,
    Active,
    Available,
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

pub fn countdown_menu_state(
    event: &Event,
    active_countdown_events: &HashSet<i64>,
    now: DateTime<Local>,
) -> CountdownMenuState {
    if event.start <= now {
        return CountdownMenuState::Hidden;
    }

    let timer_exists = event
        .id
        .map(|id| active_countdown_events.contains(&id))
        .unwrap_or(false);

    if timer_exists {
        CountdownMenuState::Active
    } else {
        CountdownMenuState::Available
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

pub fn is_ribbon_event(event: &Event) -> bool {
    event.all_day
}

pub fn event_display_end_date(event: &Event) -> NaiveDate {
    let start_date = event.start.date_naive();
    let end_date = event.end.date_naive();

    let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    if event.all_day && event.end.time() == midnight && end_date > start_date {
        end_date.pred_opt().unwrap_or(end_date)
    } else {
        end_date
    }
}

pub fn event_covers_date(event: &Event, date: NaiveDate) -> bool {
    let start_date = event.start.date_naive();
    let end_date = event_display_end_date(event);
    date >= start_date && date <= end_date
}

pub fn build_ribbon_lanes(events: &[Event]) -> Vec<Vec<&Event>> {
    let mut sorted: Vec<&Event> = events.iter().filter(|event| is_ribbon_event(event)).collect();

    sorted.sort_by(|a, b| {
        let a_start = a.start.date_naive();
        let b_start = b.start.date_naive();
        let a_span_days = (event_display_end_date(a) - a_start).num_days();
        let b_span_days = (event_display_end_date(b) - b_start).num_days();

        a_start
            .cmp(&b_start)
            .then_with(|| b_span_days.cmp(&a_span_days))
            .then_with(|| a.title.cmp(&b.title))
    });

    let mut lanes: Vec<Vec<&Event>> = Vec::new();

    'place_event: for event in sorted {
        let start = event.start.date_naive();
        let end = event_display_end_date(event);

        for lane in &mut lanes {
            let overlaps = lane.iter().any(|existing| {
                let existing_start = existing.start.date_naive();
                let existing_end = event_display_end_date(existing);
                !(end < existing_start || start > existing_end)
            });

            if !overlaps {
                lane.push(event);
                continue 'place_event;
            }
        }

        lanes.push(vec![event]);
    }

    lanes
}

pub fn load_synced_event_ids(
    database: &'static Database,
    source_id: Option<i64>,
) -> HashSet<i64> {
    let service = EventSyncMapService::new(database.connection());
    let ids_result = match source_id {
        Some(source_id) => service.list_synced_local_event_ids_for_enabled_source(source_id),
        None => service.list_synced_local_event_ids_for_enabled_sources(),
    };

    match ids_result {
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

pub fn filter_events_by_sync_scope(
    events: Vec<Event>,
    database: &'static Database,
    synced_only: bool,
    synced_source_id: Option<i64>,
) -> Vec<Event> {
    if !synced_only && synced_source_id.is_none() {
        return events;
    }

    let selected_synced_ids = load_synced_event_ids(database, synced_source_id);

    if synced_only {
        return events
            .into_iter()
            .filter(|event| is_synced_event(event.id, &selected_synced_ids))
            .collect();
    }

    let all_synced_ids = load_synced_event_ids(database, None);
    events
        .into_iter()
        .filter(|event| {
            is_synced_event(event.id, &selected_synced_ids)
                || !is_synced_event(event.id, &all_synced_ids)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
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

    #[test]
    fn test_filter_events_by_sync_scope_selected_source_plus_local() {
        let db = Box::leak(Box::new(Database::new(":memory:").unwrap()));
        db.initialize_schema().unwrap();

        let conn = db.connection();
        conn.execute(
            "INSERT INTO calendar_sources (name, source_type, ics_url, enabled, poll_interval_minutes)
             VALUES (?1, ?2, ?3, 1, 15)",
            params![
                "Den",
                "google_ics",
                "https://calendar.google.com/calendar/ical/den/basic.ics"
            ],
        )
        .unwrap();
        let den_source_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO calendar_sources (name, source_type, ics_url, enabled, poll_interval_minutes)
             VALUES (?1, ?2, ?3, 1, 15)",
            params![
                "Birthdays",
                "google_ics",
                "https://calendar.google.com/calendar/ical/birthdays/basic.ics"
            ],
        )
        .unwrap();
        let birthday_source_id = conn.last_insert_rowid();

        let mut den_event = make_event("Den synced", None);
        den_event.id = Some(101);
        let mut birthday_event = make_event("Birthday synced", None);
        birthday_event.id = Some(102);
        let mut local_event = make_event("Local", None);
        local_event.id = Some(103);

        for (id, title) in [(101_i64, "Den synced"), (102_i64, "Birthday synced"), (103_i64, "Local")]
        {
            conn.execute(
                "INSERT INTO events (id, title, start_datetime, end_datetime, is_all_day)
                 VALUES (?1, ?2, ?3, ?4, 0)",
                params![
                    id,
                    title,
                    "2026-02-27T09:00:00+10:00",
                    "2026-02-27T10:00:00+10:00"
                ],
            )
            .unwrap();
        }

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id)
             VALUES (?1, ?2, ?3)",
            params![den_source_id, "uid-den", 101_i64],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id)
             VALUES (?1, ?2, ?3)",
            params![birthday_source_id, "uid-bday", 102_i64],
        )
        .unwrap();

        let filtered = filter_events_by_sync_scope(
            vec![den_event, birthday_event, local_event],
            db,
            false,
            Some(den_source_id),
        );

        let ids: HashSet<i64> = filtered.into_iter().filter_map(|event| event.id).collect();
        assert!(ids.contains(&101));
        assert!(ids.contains(&103));
        assert!(!ids.contains(&102));
    }

    #[test]
    fn test_countdown_menu_state_hidden_for_non_future_event() {
        let event = make_event("Past Event", Some("Work"));
        let active = HashSet::new();

        let now = event.start + Duration::minutes(1);
        assert_eq!(
            countdown_menu_state(&event, &active, now),
            CountdownMenuState::Hidden
        );
    }

    #[test]
    fn test_countdown_menu_state_active_when_existing_card_found() {
        let mut event = make_event("Future Event", Some("Work"));
        event.id = Some(9);

        let mut active = HashSet::new();
        active.insert(9);

        let now = event.start - Duration::minutes(1);
        assert_eq!(
            countdown_menu_state(&event, &active, now),
            CountdownMenuState::Active
        );
    }

    #[test]
    fn test_countdown_menu_state_available_for_future_without_card() {
        let mut event = make_event("Future Event", Some("Work"));
        event.id = Some(9);

        let active = HashSet::new();
        let now = event.start - Duration::minutes(1);
        assert_eq!(
            countdown_menu_state(&event, &active, now),
            CountdownMenuState::Available
        );
    }

    #[test]
    fn test_is_ribbon_event_for_all_day_event() {
        let mut event = make_event("All Day", Some("Work"));
        event.all_day = true;

        assert!(is_ribbon_event(&event));
    }

    #[test]
    fn test_is_ribbon_event_false_for_multi_day_timed_event() {
        let start = Local.with_ymd_and_hms(2025, 1, 15, 22, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2025, 1, 16, 23, 0, 0).unwrap();
        let event = Event {
            id: None,
            title: "Overnight".to_string(),
            description: None,
            location: None,
            start,
            end,
            all_day: false,
            category: Some("Work".to_string()),
            color: None,
            recurrence_rule: None,
            recurrence_exceptions: None,
            created_at: None,
            updated_at: None,
        };

        assert!(!is_ribbon_event(&event));
    }

    #[test]
    fn test_is_ribbon_event_false_for_short_overnight_event() {
        let start = Local.with_ymd_and_hms(2025, 1, 15, 22, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2025, 1, 16, 2, 0, 0).unwrap();
        let event = Event {
            id: None,
            title: "Short Overnight".to_string(),
            description: None,
            location: None,
            start,
            end,
            all_day: false,
            category: Some("Work".to_string()),
            color: None,
            recurrence_rule: None,
            recurrence_exceptions: None,
            created_at: None,
            updated_at: None,
        };

        assert!(!is_ribbon_event(&event));
    }

    #[test]
    fn test_is_ribbon_event_false_for_single_day_timed_event() {
        let event = make_event("Single Day", Some("Work"));

        assert!(!is_ribbon_event(&event));
    }

    #[test]
    fn test_event_display_end_date_treats_all_day_midnight_end_as_exclusive() {
        let start = Local.with_ymd_and_hms(2026, 2, 23, 0, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2026, 2, 24, 0, 0, 0).unwrap();
        let event = Event {
            id: None,
            title: "All-day one day".to_string(),
            description: None,
            location: None,
            start,
            end,
            all_day: true,
            category: None,
            color: None,
            recurrence_rule: None,
            recurrence_exceptions: None,
            created_at: None,
            updated_at: None,
        };

        assert_eq!(event_display_end_date(&event), start.date_naive());
    }

    #[test]
    fn test_event_display_end_date_keeps_timed_event_end_date() {
        let start = Local.with_ymd_and_hms(2026, 2, 23, 22, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2026, 2, 24, 2, 0, 0).unwrap();
        let event = Event {
            id: None,
            title: "Timed overnight".to_string(),
            description: None,
            location: None,
            start,
            end,
            all_day: false,
            category: None,
            color: None,
            recurrence_rule: None,
            recurrence_exceptions: None,
            created_at: None,
            updated_at: None,
        };

        assert_eq!(event_display_end_date(&event), end.date_naive());
    }

    #[test]
    fn test_build_ribbon_lanes_splits_overlapping_events() {
        let day1 = Local.with_ymd_and_hms(2026, 3, 16, 0, 0, 0).unwrap();
        let day2 = Local.with_ymd_and_hms(2026, 3, 17, 0, 0, 0).unwrap();
        let day3 = Local.with_ymd_and_hms(2026, 3, 18, 0, 0, 0).unwrap();

        let event_a = Event {
            id: None,
            title: "Event A".to_string(),
            description: None,
            location: None,
            start: day1,
            end: day3,
            all_day: true,
            category: None,
            color: None,
            recurrence_rule: None,
            recurrence_exceptions: None,
            created_at: None,
            updated_at: None,
        };

        let event_b = Event {
            id: None,
            title: "Event B".to_string(),
            description: None,
            location: None,
            start: day2,
            end: day3,
            all_day: true,
            category: None,
            color: None,
            recurrence_rule: None,
            recurrence_exceptions: None,
            created_at: None,
            updated_at: None,
        };

        let events = vec![event_a, event_b];
        let lanes = build_ribbon_lanes(&events);
        assert_eq!(lanes.len(), 2);
    }
}
