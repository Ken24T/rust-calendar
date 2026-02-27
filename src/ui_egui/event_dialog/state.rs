use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::calendar_sync::mapping::EventSyncMapService;
use crate::services::countdown::{CountdownCardId, CountdownCardVisuals};
use crate::services::database::Database;
use crate::services::event::EventService;
use chrono::{self, Datelike, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime};

use super::recurrence::{ParsedRRule, RRuleBuilder, RecurrenceFrequency, RecurrencePattern};

/// Which date field is being edited by the inline calendar
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DatePickerTarget {
    StartDate,
    EndDate,
}

/// Optional countdown card state linked to the event
#[derive(Clone)]
pub struct LinkedCountdownCard {
    pub card_id: CountdownCardId,
    pub visuals: CountdownCardVisuals,
    pub always_on_top: bool,
}

/// State for the event editing dialog
pub struct EventDialogState {
    pub event_id: Option<i64>,
    pub title: String,
    pub description: String,
    pub location: String,
    pub date: NaiveDate,
    pub end_date: NaiveDate,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub all_day: bool,
    pub color: String,
    pub category: String,
    pub is_recurring: bool,
    pub frequency: RecurrenceFrequency,
    pub interval: u32,
    pub count: Option<u32>,
    pub until_date: Option<NaiveDate>,
    pub pattern: RecurrencePattern,
    pub byday_enabled: bool,
    pub byday_monday: bool,
    pub byday_tuesday: bool,
    pub byday_wednesday: bool,
    pub byday_thursday: bool,
    pub byday_friday: bool,
    pub byday_saturday: bool,
    pub byday_sunday: bool,
    pub error_message: Option<String>,
    /// Non-blocking warning messages (displayed in yellow)
    pub warning_messages: Vec<String>,
    #[allow(dead_code)]
    pub show_advanced: bool,
    pub create_countdown: bool,
    /// Linked countdown card (if any)
    pub linked_card: Option<LinkedCountdownCard>,
    /// Whether the card settings section is expanded
    pub show_card_settings: bool,
    /// Which date picker is currently shown (None = neither)
    pub active_date_picker: Option<DatePickerTarget>,
    /// The month/year currently being viewed in the date picker
    pub date_picker_viewing: NaiveDate,
    /// Whether this is a past event (end time < now) - date/time fields are read-only
    pub is_past_event: bool,
}

impl EventDialogState {
    pub fn new_event(date: NaiveDate, settings: &Settings) -> Self {
        Self::new_event_with_time(date, None, settings)
    }

    pub fn new_event_with_time(
        date: NaiveDate,
        start_time_opt: Option<NaiveTime>,
        settings: &Settings,
    ) -> Self {
        let start_time = start_time_opt.unwrap_or_else(|| {
            let (start_hour, start_minute) = settings
                .default_event_start_time
                .split_once(':')
                .and_then(|(h, m)| {
                    let hour = h.parse::<u32>().ok()?;
                    let minute = m.parse::<u32>().ok()?;
                    Some((hour, minute))
                })
                .unwrap_or((9, 0));

            NaiveTime::from_hms_opt(start_hour, start_minute, 0)
                .unwrap_or(NaiveTime::from_hms_opt(9, 0, 0).unwrap())
        });

        let duration_minutes = settings.default_event_duration as i64;
        let end_datetime =
            NaiveDateTime::new(date, start_time) + chrono::Duration::minutes(duration_minutes);
        let end_time = end_datetime.time();

        Self {
            event_id: None,
            title: String::new(),
            description: String::new(),
            location: String::new(),
            date,
            end_date: date,
            start_time,
            end_time,
            all_day: false,
            color: "#3B82F6".to_string(),
            category: String::new(),
            is_recurring: false,
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            count: None,
            until_date: None,
            pattern: RecurrencePattern::None,
            byday_enabled: false,
            byday_monday: false,
            byday_tuesday: false,
            byday_wednesday: false,
            byday_thursday: false,
            byday_friday: false,
            byday_saturday: false,
            byday_sunday: false,
            error_message: None,
            warning_messages: Vec::new(),
            show_advanced: false,
            create_countdown: false,
            linked_card: None,
            show_card_settings: false,
            active_date_picker: None,
            date_picker_viewing: date,
            is_past_event: false,
        }
    }

    pub fn from_event(event: &Event, _settings: &Settings) -> Self {
        let date = event.start.date_naive();
        let start_time = event.start.time();
        let end_time = event.end.time();
        let is_past = event.end < Local::now();

        let parsed = event
            .recurrence_rule
            .as_ref()
            .map(|rrule| ParsedRRule::parse(rrule))
            .unwrap_or_default();

        Self {
            event_id: event.id,
            title: event.title.clone(),
            description: event.description.clone().unwrap_or_default(),
            location: event.location.clone().unwrap_or_default(),
            date,
            end_date: event.end.date_naive(),
            start_time,
            end_time,
            all_day: event.all_day,
            color: event.color.clone().unwrap_or_else(|| "#3B82F6".to_string()),
            category: event.category.clone().unwrap_or_default(),
            is_recurring: parsed.is_recurring,
            frequency: parsed.frequency,
            interval: parsed.interval,
            count: parsed.count,
            until_date: parsed.until_date,
            pattern: parsed.pattern,
            byday_enabled: parsed.byday_flags.iter().any(|&b| b),
            byday_sunday: parsed.byday_flags[0],
            byday_monday: parsed.byday_flags[1],
            byday_tuesday: parsed.byday_flags[2],
            byday_wednesday: parsed.byday_flags[3],
            byday_thursday: parsed.byday_flags[4],
            byday_friday: parsed.byday_flags[5],
            byday_saturday: parsed.byday_flags[6],
            error_message: None,
            warning_messages: Vec::new(),
            show_advanced: false,
            create_countdown: false,
            linked_card: None,
            show_card_settings: false,
            active_date_picker: None,
            date_picker_viewing: date,
            is_past_event: is_past,
        }
    }

    /// Link a countdown card to this event dialog state
    pub fn link_countdown_card(&mut self, card_id: CountdownCardId, visuals: CountdownCardVisuals) {
        self.linked_card = Some(LinkedCountdownCard {
            card_id,
            always_on_top: visuals.always_on_top,
            visuals,
        });
        self.show_card_settings = true;
    }

    pub fn save(&self, database: &Database) -> Result<Event, String> {
        let mut event = self.to_event()?;
        let service = EventService::new(database.connection());
        let sync_map_service = EventSyncMapService::new(database.connection());

        if let Some(id) = self.event_id {
            if sync_map_service
                .is_synced_local_event(id)
                .map_err(|e| format!("Failed to check sync status: {}", e))?
            {
                return Err("Synced events are read-only and cannot be edited".to_string());
            }

            event.id = Some(id);
            service
                .update(&event)
                .map_err(|e| format!("Failed to update event: {}", e))?;
            Ok(event)
        } else {
            service
                .create(event)
                .map_err(|e| format!("Failed to create event: {}", e))
        }
    }

    fn start_end_datetimes(
        &self,
    ) -> Result<(chrono::DateTime<Local>, chrono::DateTime<Local>), String> {
        let start_naive = NaiveDateTime::new(self.date, self.start_time);
        let end_naive = NaiveDateTime::new(self.end_date, self.end_time);

        let start = match start_naive.and_local_timezone(Local) {
            LocalResult::Single(dt) => dt,
            LocalResult::Ambiguous(dt, _) => dt,
            LocalResult::None => {
                return Err("Start time is invalid for the selected day".to_string());
            }
        };

        let end = match end_naive.and_local_timezone(Local) {
            LocalResult::Single(dt) => dt,
            LocalResult::Ambiguous(dt, _) => dt,
            LocalResult::None => {
                return Err("End time is invalid for the selected day".to_string());
            }
        };

        Ok((start, end))
    }

    fn build_rrule(&self) -> Option<String> {
        RRuleBuilder {
            is_recurring: self.is_recurring,
            frequency: self.frequency,
            interval: self.interval,
            pattern: self.pattern,
            byday_enabled: self.byday_enabled,
            byday_flags: [
                self.byday_sunday,
                self.byday_monday,
                self.byday_tuesday,
                self.byday_wednesday,
                self.byday_thursday,
                self.byday_friday,
                self.byday_saturday,
            ],
            count: self.count,
            until_date: self.until_date,
        }
        .build()
    }

    fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("Event title is required".to_string());
        }

        if self.title.len() > 200 {
            return Err("Event title is too long (max 200 characters)".to_string());
        }

        let (start_dt, end_dt) = self.start_end_datetimes()?;
        if end_dt <= start_dt {
            return Err("Event must end after it starts".to_string());
        }

        if self.is_recurring {
            if self.interval < 1 {
                return Err("Interval must be at least 1".to_string());
            }

            if self.interval > 999 {
                return Err("Interval is too large (max 999)".to_string());
            }

            if self.byday_enabled
                && matches!(
                    self.frequency,
                    RecurrenceFrequency::Weekly | RecurrenceFrequency::Monthly
                )
            {
                let any_day_selected = self.byday_monday
                    || self.byday_tuesday
                    || self.byday_wednesday
                    || self.byday_thursday
                    || self.byday_friday
                    || self.byday_saturday
                    || self.byday_sunday;

                if !any_day_selected {
                    return Err("Select at least one day for weekly/monthly recurrence".to_string());
                }
            }

            if let Some(count) = self.count {
                if count < 1 {
                    return Err("Occurrence count must be at least 1".to_string());
                }
                if count > 999 {
                    return Err("Occurrence count is too large (max 999)".to_string());
                }
            }

            if let Some(until) = self.until_date {
                if until < self.date {
                    return Err("Recurrence end date cannot be before event start date".to_string());
                }
            }
        }

        if !self.color.is_empty() && !self.color.starts_with('#') {
            return Err("Color must start with # (e.g., #3B82F6)".to_string());
        }

        Ok(())
    }

    /// Check for non-blocking warnings (overlap detection, distant past, etc.)
    /// Call this when the dialog is opened or when dates change
    pub fn check_warnings(&mut self, database: &Database) {
        self.warning_messages.clear();
        
        let today = Local::now().date_naive();
        
        // Warning: Event date is more than 5 years in the past
        let five_years_ago = today.with_year(today.year() - 5).unwrap_or(today);
        if self.date < five_years_ago {
            self.warning_messages.push(
                "This event is more than 5 years in the past".to_string()
            );
        }
        
        // Warning: All-day event with non-midnight times
        if self.all_day {
            let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
            if self.start_time != midnight || self.end_time != midnight {
                // Don't warn - we'll just ignore the times for all-day events
                // This is handled at save time by using date boundaries
            }
        }
        
        // Warning: Overlap detection
        if let Ok((start_dt, end_dt)) = self.start_end_datetimes() {
            let service = EventService::new(database.connection());
            if let Ok(overlapping) = service.find_by_date_range(start_dt, end_dt) {
                // Filter out the current event being edited
                let other_overlapping: Vec<_> = overlapping
                    .iter()
                    .filter(|e| {
                        // Exclude the current event
                        if let Some(current_id) = self.event_id {
                            e.id != Some(current_id)
                        } else {
                            true
                        }
                    })
                    .filter(|e| {
                        // Check for actual time overlap (not just in same date range)
                        let event_start = e.start;
                        let event_end = e.end;
                        // Events overlap if one starts before the other ends
                        start_dt < event_end && end_dt > event_start
                    })
                    .collect();
                
                if !other_overlapping.is_empty() {
                    if other_overlapping.len() == 1 {
                        self.warning_messages.push(format!(
                            "Overlaps with \"{}\"",
                            other_overlapping[0].title
                        ));
                    } else {
                        self.warning_messages.push(format!(
                            "Overlaps with {} other events",
                            other_overlapping.len()
                        ));
                    }
                }
            }
        }
    }

    fn to_event(&self) -> Result<Event, String> {
        self.validate()?;
        let (start_datetime, end_datetime) = self.start_end_datetimes()?;

        let mut event = Event::builder()
            .title(&self.title)
            .start(start_datetime)
            .end(end_datetime)
            .all_day(self.all_day);

        if !self.description.is_empty() {
            event = event.description(&self.description);
        }

        if !self.location.is_empty() {
            event = event.location(&self.location);
        }

        if !self.color.is_empty() {
            event = event.color(&self.color);
        }

        if !self.category.is_empty() {
            event = event.category(&self.category);
        }

        if let Some(rrule) = self.build_rrule() {
            event = event.recurrence_rule(rrule);
        }

        event.build()
    }

    pub(super) fn weekday_flag(&self, index: u8) -> bool {
        match index % 7 {
            0 => self.byday_sunday,
            1 => self.byday_monday,
            2 => self.byday_tuesday,
            3 => self.byday_wednesday,
            4 => self.byday_thursday,
            5 => self.byday_friday,
            6 => self.byday_saturday,
            _ => false,
        }
    }

    pub(super) fn set_weekday_flag(&mut self, index: u8, value: bool) {
        match index % 7 {
            0 => self.byday_sunday = value,
            1 => self.byday_monday = value,
            2 => self.byday_tuesday = value,
            3 => self.byday_wednesday = value,
            4 => self.byday_thursday = value,
            5 => self.byday_friday = value,
            6 => self.byday_saturday = value,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::event::Event;
    use crate::models::settings::Settings;
    use crate::services::database::Database;
    use crate::ui_egui::event_dialog::recurrence::{ParsedRRule, Weekday};
    use chrono::{Local, NaiveDate, NaiveTime, TimeZone};

    fn sample_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()
    }

    fn base_state() -> EventDialogState {
        let settings = Settings::default();
        let mut state = EventDialogState::new_event(sample_date(), &settings);
        state.title = "Test Event".to_string();
        state
    }

    #[test]
    fn new_event_uses_settings_defaults() {
        let settings = Settings {
            default_event_start_time: "07:30".to_string(),
            default_event_duration: 30,
            ..Settings::default()
        };

        let state = EventDialogState::new_event(sample_date(), &settings);
        assert_eq!(state.start_time, NaiveTime::from_hms_opt(7, 30, 0).unwrap());
        assert_eq!(state.end_time, NaiveTime::from_hms_opt(8, 0, 0).unwrap());
    }

    #[test]
    fn build_rrule_weekly_lists_selected_days() {
        let mut state = base_state();
        state.is_recurring = true;
        state.frequency = RecurrenceFrequency::Weekly;
        state.byday_enabled = true;
        state.byday_monday = true;
        state.byday_friday = true;

        assert_eq!(
            state.build_rrule(),
            Some("FREQ=WEEKLY;BYDAY=MO,FR".to_string())
        );
    }

    #[test]
    fn build_rrule_monthly_uses_positional_pattern() {
        let mut state = base_state();
        state.is_recurring = true;
        state.frequency = RecurrenceFrequency::Monthly;
        state.pattern = RecurrencePattern::FirstWeekdayOfPeriod(
            crate::ui_egui::event_dialog::recurrence::Weekday::Tuesday,
        );

        assert_eq!(
            state.build_rrule(),
            Some("FREQ=MONTHLY;BYDAY=1TU".to_string())
        );
    }

    #[test]
    fn parse_rrule_handles_byday_and_interval() {
        let parsed = ParsedRRule::parse("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE");

        assert!(parsed.is_recurring);
        assert_eq!(parsed.frequency, RecurrenceFrequency::Weekly);
        assert_eq!(parsed.interval, 2);
        assert_eq!(parsed.pattern, RecurrencePattern::None);
        assert!(parsed.byday_flags[1]); // Monday (index 1)
        assert!(parsed.byday_flags[3]); // Wednesday (index 3)
    }

    #[test]
    fn parse_rrule_detects_positional_weekday() {
        let parsed = ParsedRRule::parse("FREQ=MONTHLY;BYDAY=1TH");

        assert_eq!(parsed.frequency, RecurrenceFrequency::Monthly);
        assert!(matches!(
            parsed.pattern,
            RecurrencePattern::FirstWeekdayOfPeriod(Weekday::Thursday)
        ));
        assert!(parsed.byday_flags.iter().all(|flag| !flag));
    }

    #[test]
    fn validate_rejects_empty_title() {
        let mut state = base_state();
        state.title.clear();
        assert!(state.validate().is_err());
    }

    #[test]
    fn validate_requires_specific_days_when_enabled() {
        let mut state = base_state();
        state.is_recurring = true;
        state.frequency = RecurrenceFrequency::Weekly;
        state.byday_enabled = true;
        let err = state.validate().unwrap_err();
        assert!(err.contains("Select at least one day"));
    }

    #[test]
    fn validate_allows_multi_day_events() {
        let mut state = base_state();
        state.end_date = state.date.succ_opt().unwrap();
        state.end_time = state.start_time; // equal times allowed when date advances
        assert!(state.validate().is_ok());
    }

    #[test]
    fn validate_rejects_end_before_start_date() {
        let mut state = base_state();
        state.end_date = state.date.pred_opt().unwrap();
        let err = state.validate().unwrap_err();
        assert!(err.contains("Event must end"));
    }

    #[test]
    fn validate_accepts_hex_colors() {
        let mut state = base_state();
        state.color = "#AABBCC".to_string();
        assert!(state.validate().is_ok());
    }

    #[test]
    fn to_event_propagates_recurrence_rule() {
        let mut state = base_state();
        state.is_recurring = true;
        state.frequency = RecurrenceFrequency::Weekly;
        state.byday_enabled = true;
        state.byday_tuesday = true;

        let event = state.to_event().expect("event should build");
        assert_eq!(event.title, "Test Event");
        assert_eq!(
            event.recurrence_rule,
            Some("FREQ=WEEKLY;BYDAY=TU".to_string())
        );
    }

    #[test]
    fn save_blocks_updates_for_synced_events() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO calendar_sources (name, source_type, ics_url, enabled, poll_interval_minutes)
             VALUES (?1, ?2, ?3, 1, 15)",
            rusqlite::params![
                "Test Source",
                "google_ics",
                "https://example.com/calendar.ics"
            ],
        )
        .unwrap();
        let source_id = conn.last_insert_rowid();

        let start = Local.with_ymd_and_hms(2026, 2, 27, 9, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2026, 2, 27, 10, 0, 0).unwrap();
        let event = Event::new("Synced Event".to_string(), start, end).unwrap();
        let event_service = EventService::new(conn);
        let created = event_service.create(event.clone()).unwrap();
        let event_id = created.id.unwrap();

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                source_id,
                "uid-test",
                event_id,
                "2026-02-27T00:00:00Z",
                "2026-02-27T00:00:00Z"
            ],
        )
        .unwrap();

        let mut state = EventDialogState::from_event(&created, &Settings::default());
        state.title = "Changed".to_string();

        let err = state.save(&db).unwrap_err();
        assert!(err.contains("read-only"));
    }
}
