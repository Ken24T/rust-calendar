use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;
use chrono::{self, Local, NaiveDate, NaiveDateTime, NaiveTime};

use super::recurrence::{parse_until_date, RecurrenceFrequency, RecurrencePattern, Weekday};

/// State for the event editing dialog
pub struct EventDialogState {
    pub event_id: Option<i64>,
    pub title: String,
    pub description: String,
    pub location: String,
    pub date: NaiveDate,
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
    #[allow(dead_code)]
    pub show_advanced: bool,
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
            show_advanced: false,
        }
    }

    pub fn from_event(event: &Event, _settings: &Settings) -> Self {
        let date = event.start.date_naive();
        let start_time = event.start.time();
        let end_time = event.end.time();

        let (is_recurring, frequency, interval, count, until_date, pattern, byday_flags) =
            if let Some(ref rrule) = event.recurrence_rule {
                Self::parse_rrule(rrule)
            } else {
                (
                    false,
                    RecurrenceFrequency::Daily,
                    1,
                    None,
                    None,
                    RecurrencePattern::None,
                    [false; 7],
                )
            };

        Self {
            event_id: event.id,
            title: event.title.clone(),
            description: event.description.clone().unwrap_or_default(),
            location: event.location.clone().unwrap_or_default(),
            date,
            start_time,
            end_time,
            all_day: event.all_day,
            color: event.color.clone().unwrap_or_else(|| "#3B82F6".to_string()),
            category: event.category.clone().unwrap_or_default(),
            is_recurring,
            frequency,
            interval,
            count,
            until_date,
            pattern,
            byday_enabled: byday_flags.iter().any(|&b| b),
            byday_sunday: byday_flags[0],
            byday_monday: byday_flags[1],
            byday_tuesday: byday_flags[2],
            byday_wednesday: byday_flags[3],
            byday_thursday: byday_flags[4],
            byday_friday: byday_flags[5],
            byday_saturday: byday_flags[6],
            error_message: None,
            show_advanced: false,
        }
    }

    pub fn save(&self, database: &Database) -> Result<Event, String> {
        let mut event = self.to_event()?;
        let service = EventService::new(database.connection());

        if let Some(id) = self.event_id {
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

    fn parse_rrule(
        rrule: &str,
    ) -> (
        bool,
        RecurrenceFrequency,
        u32,
        Option<u32>,
        Option<NaiveDate>,
        RecurrencePattern,
        [bool; 7],
    ) {
        let mut frequency = RecurrenceFrequency::Daily;
        let mut interval = 1u32;
        let mut count = None;
        let mut until_date = None;
        let mut pattern = RecurrencePattern::None;
        let mut byday_flags = [false; 7];

        for part in rrule.split(';') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "FREQ" => {
                        frequency = match value {
                            "DAILY" => RecurrenceFrequency::Daily,
                            "WEEKLY" => RecurrenceFrequency::Weekly,
                            "MONTHLY" => RecurrenceFrequency::Monthly,
                            "YEARLY" => RecurrenceFrequency::Yearly,
                            _ => RecurrenceFrequency::Daily,
                        };
                    }
                    "INTERVAL" => {
                        if let Ok(val) = value.parse::<u32>() {
                            interval = val;
                        }
                    }
                    "COUNT" => {
                        if let Ok(val) = value.parse::<u32>() {
                            count = Some(val);
                        }
                    }
                    "UNTIL" => {
                        until_date = parse_until_date(value);
                    }
                    "BYMONTHDAY" => {
                        if value == "1" {
                            pattern = RecurrencePattern::FirstDayOfPeriod;
                        } else if value == "-1" {
                            pattern = RecurrencePattern::LastDayOfPeriod;
                        }
                    }
                    "BYDAY" => {
                        for day in value.split(',') {
                            if day.len() > 2 {
                                if day.starts_with('1') && day.len() == 3 {
                                    if let Some(weekday) = Weekday::from_rrule_day(&day[1..]) {
                                        pattern = RecurrencePattern::FirstWeekdayOfPeriod(weekday);
                                    }
                                } else if day.starts_with("-1") && day.len() == 4 {
                                    if let Some(weekday) = Weekday::from_rrule_day(&day[2..]) {
                                        pattern = RecurrencePattern::LastWeekdayOfPeriod(weekday);
                                    }
                                }
                            } else {
                                match day {
                                    "SU" => byday_flags[0] = true,
                                    "MO" => byday_flags[1] = true,
                                    "TU" => byday_flags[2] = true,
                                    "WE" => byday_flags[3] = true,
                                    "TH" => byday_flags[4] = true,
                                    "FR" => byday_flags[5] = true,
                                    "SA" => byday_flags[6] = true,
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        (
            true,
            frequency,
            interval,
            count,
            until_date,
            pattern,
            byday_flags,
        )
    }

    fn build_rrule(&self) -> Option<String> {
        if !self.is_recurring {
            return None;
        }

        let mut parts = vec![format!("FREQ={}", self.frequency.to_rrule_freq())];

        if self.interval > 1 {
            parts.push(format!("INTERVAL={}", self.interval));
        }

        if matches!(
            self.frequency,
            RecurrenceFrequency::Monthly | RecurrenceFrequency::Yearly
        ) {
            match self.pattern {
                RecurrencePattern::FirstDayOfPeriod => {
                    parts.push("BYMONTHDAY=1".to_string());
                }
                RecurrencePattern::LastDayOfPeriod => {
                    parts.push("BYMONTHDAY=-1".to_string());
                }
                RecurrencePattern::FirstWeekdayOfPeriod(weekday) => {
                    parts.push(format!("BYDAY=1{}", weekday.to_rrule_day()));
                }
                RecurrencePattern::LastWeekdayOfPeriod(weekday) => {
                    parts.push(format!("BYDAY=-1{}", weekday.to_rrule_day()));
                }
                RecurrencePattern::None => self.append_standard_byday(&mut parts),
            }
        } else if self.frequency == RecurrenceFrequency::Weekly {
            self.append_standard_byday(&mut parts);
        }

        if let Some(count) = self.count {
            parts.push(format!("COUNT={}", count));
        } else if let Some(until) = self.until_date {
            parts.push(format!("UNTIL={}", until.format("%Y%m%d")));
        }

        Some(parts.join(";"))
    }

    fn append_standard_byday(&self, parts: &mut Vec<String>) {
        if !self.byday_enabled {
            return;
        }

        let mut days = Vec::new();
        if self.byday_sunday {
            days.push("SU");
        }
        if self.byday_monday {
            days.push("MO");
        }
        if self.byday_tuesday {
            days.push("TU");
        }
        if self.byday_wednesday {
            days.push("WE");
        }
        if self.byday_thursday {
            days.push("TH");
        }
        if self.byday_friday {
            days.push("FR");
        }
        if self.byday_saturday {
            days.push("SA");
        }

        if !days.is_empty() {
            parts.push(format!("BYDAY={}", days.join(",")));
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("Event title is required".to_string());
        }

        if self.title.len() > 200 {
            return Err("Event title is too long (max 200 characters)".to_string());
        }

        if !self.all_day && self.end_time <= self.start_time {
            return Err("End time must be after start time".to_string());
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

    fn to_event(&self) -> Result<Event, String> {
        self.validate()?;

        let start_datetime = self
            .date
            .and_time(self.start_time)
            .and_local_timezone(Local)
            .unwrap();
        let end_datetime = self
            .date
            .and_time(self.end_time)
            .and_local_timezone(Local)
            .unwrap();

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::Settings;
    use chrono::{NaiveDate, NaiveTime};

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
        let mut settings = Settings::default();
        settings.default_event_start_time = "07:30".to_string();
        settings.default_event_duration = 30;

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
        state.pattern = RecurrencePattern::FirstWeekdayOfPeriod(Weekday::Tuesday);

        assert_eq!(
            state.build_rrule(),
            Some("FREQ=MONTHLY;BYDAY=1TU".to_string())
        );
    }

    #[test]
    fn parse_rrule_handles_byday_and_interval() {
        let (is_recurring, frequency, interval, _count, _until, pattern, byday_flags) =
            EventDialogState::parse_rrule("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE");

        assert!(is_recurring);
        assert_eq!(frequency, RecurrenceFrequency::Weekly);
        assert_eq!(interval, 2);
        assert_eq!(pattern, RecurrencePattern::None);
        assert!(byday_flags[1]); // Monday (index 1)
        assert!(byday_flags[3]); // Wednesday (index 3)
    }

    #[test]
    fn parse_rrule_detects_positional_weekday() {
        let (_, frequency, _, _, _, pattern, byday_flags) =
            EventDialogState::parse_rrule("FREQ=MONTHLY;BYDAY=1TH");

        assert_eq!(frequency, RecurrenceFrequency::Monthly);
        assert!(matches!(
            pattern,
            RecurrencePattern::FirstWeekdayOfPeriod(Weekday::Thursday)
        ));
        assert!(byday_flags.iter().all(|flag| !flag));
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
}
