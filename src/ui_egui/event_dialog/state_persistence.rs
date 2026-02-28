//! Validation, persistence, and warning logic for `EventDialogState`.
//!
//! Extracted from `state.rs` — handles save, validate, check_warnings,
//! to_event, start_end_datetimes, and build_rrule.

use chrono::{self, Datelike, Local, LocalResult, NaiveDateTime, NaiveTime};

use crate::models::event::Event;
use crate::services::calendar_sync::mapping::EventSyncMapService;
use crate::services::database::Database;
use crate::services::event::EventService;

use super::recurrence::{RRuleBuilder, RecurrenceFrequency};
use super::state::EventDialogState;

impl EventDialogState {
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

    pub(super) fn start_end_datetimes(
        &self,
    ) -> Result<(chrono::DateTime<Local>, chrono::DateTime<Local>), String> {
        // For all-day events, normalise times to midnight and use exclusive end
        // date (iCal convention): a single-day event on Mar 14 is stored as
        // start=Mar 14 00:00, end=Mar 15 00:00.  This keeps local and imported
        // events consistent so event_display_end_date works uniformly.
        let (start_time, end_date, end_time) = if self.all_day {
            let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
            let exclusive_end = self
                .end_date
                .succ_opt()
                .unwrap_or(self.end_date);
            (midnight, exclusive_end, midnight)
        } else {
            (self.start_time, self.end_date, self.end_time)
        };

        let start_naive = NaiveDateTime::new(self.date, start_time);
        let end_naive = NaiveDateTime::new(end_date, end_time);

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

    pub(super) fn build_rrule(&self) -> Option<String> {
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

    pub(super) fn validate(&self) -> Result<(), String> {
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

    pub(crate) fn to_event(&self) -> Result<Event, String> {
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
}
