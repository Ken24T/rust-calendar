use super::EventService;
use crate::models::event::Event;
use anyhow::Result;
use chrono::{DateTime, Local};

mod parser;
mod utils;
mod weekly;
mod monthly;
mod yearly;
mod daily;

use parser::{detect_frequency, parse_count, parse_until, RecurrenceFrequency};

impl<'a> EventService<'a> {
    /// Expand recurring events into individual occurrences within the date range.
    /// Non-recurring events are returned as-is.
    pub fn expand_recurring_events(
        &self,
        start: DateTime<Local>,
        end: DateTime<Local>,
    ) -> Result<Vec<Event>> {
        let base_events = self.find_by_date_range(start, end)?;
        let mut expanded_events = Vec::new();

        for event in base_events {
            if let Some(ref rrule) = event.recurrence_rule {
                if rrule != "None" && !rrule.is_empty() {
                    let occurrences = self.generate_occurrences(&event, start, end)?;
                    expanded_events.extend(occurrences);
                } else {
                    expanded_events.push(event);
                }
            } else {
                expanded_events.push(event);
            }
        }

        expanded_events.sort_by(|a, b| a.start.cmp(&b.start));
        Ok(expanded_events)
    }

    /// Generate occurrences of a recurring event within a date range.
    pub(super) fn generate_occurrences(
        &self,
        event: &Event,
        range_start: DateTime<Local>,
        range_end: DateTime<Local>,
    ) -> Result<Vec<Event>> {
        let Some(ref rrule) = event.recurrence_rule else {
            return Ok(Vec::new());
        };

        let duration = event.end - event.start;
        let max_count = parse_count(rrule);
        let until_date = parse_until(rrule);

        let occurrences = match detect_frequency(rrule) {
            RecurrenceFrequency::Weekly => weekly::generate(
                event,
                rrule,
                range_start,
                range_end,
                duration,
                max_count,
                until_date,
            ),
            RecurrenceFrequency::Monthly => monthly::generate(
                event,
                rrule,
                range_start,
                range_end,
                duration,
                max_count,
                until_date,
            ),
            RecurrenceFrequency::Yearly => yearly::generate(
                event,
                rrule,
                range_start,
                range_end,
                duration,
                max_count,
                until_date,
            ),
            RecurrenceFrequency::Daily => daily::generate(
                event,
                rrule,
                range_start,
                range_end,
                duration,
                max_count,
                until_date,
            ),
        };

        Ok(occurrences)
    }
}
