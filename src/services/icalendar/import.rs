use crate::models::event::Event;
use anyhow::Result;
use chrono::Local;

use super::utils::{parse_date, parse_datetime, unescape_text};

pub(super) fn from_str(ics_content: &str) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    let lines: Vec<&str> = ics_content.lines().collect();

    let mut in_event = false;
    let mut current_event: Option<Event> = None;

    for line in lines {
        let line = line.trim();

        if line == "BEGIN:VEVENT" {
            in_event = true;
            current_event = Some(blank_event());
        } else if line == "END:VEVENT" {
            in_event = false;
            if let Some(event) = current_event.take() {
                if !event.title.is_empty() {
                    events.push(event);
                }
            }
        } else if in_event {
            if let Some(event) = current_event.as_mut() {
                parse_event_property(line, event)?;
            }
        }
    }

    Ok(events)
}

fn parse_event_property(line: &str, event: &mut Event) -> Result<()> {
    if let Some(colon_pos) = line.find(':') {
        let (key_part, value) = line.split_at(colon_pos);
        let value = &value[1..];

        let key = if let Some(semicolon) = key_part.find(';') {
            &key_part[..semicolon]
        } else {
            key_part
        };

        match key {
            "SUMMARY" => {
                event.title = unescape_text(value);
            }
            "DESCRIPTION" => {
                event.description = Some(unescape_text(value));
            }
            "LOCATION" => {
                event.location = Some(unescape_text(value));
            }
            "CATEGORIES" => {
                event.category = Some(unescape_text(value));
            }
            "X-APPLE-CALENDAR-COLOR" => {
                event.color = Some(value.to_string());
            }
            "DTSTART" => {
                if key_part.contains("VALUE=DATE") {
                    event.all_day = true;
                    event.start = parse_date(value)?;
                } else {
                    event.start = parse_datetime(value)?;
                }
            }
            "DTEND" => {
                if key_part.contains("VALUE=DATE") {
                    event.end = parse_date(value)?;
                } else {
                    event.end = parse_datetime(value)?;
                }
            }
            "RRULE" => {
                event.recurrence_rule = Some(value.to_string());
            }
            "EXDATE" => {
                let dates: Result<Vec<_>> =
                    value.split(',').map(|s| parse_datetime(s.trim())).collect();
                event.recurrence_exceptions = Some(dates?);
            }
            "CREATED" => {
                event.created_at = Some(parse_datetime(value)?);
            }
            "LAST-MODIFIED" => {
                event.updated_at = Some(parse_datetime(value)?);
            }
            _ => {}
        }
    }

    Ok(())
}

fn blank_event() -> Event {
    Event {
        id: None,
        title: String::new(),
        description: None,
        location: None,
        start: Local::now(),
        end: Local::now(),
        all_day: false,
        category: None,
        color: None,
        recurrence_rule: None,
        recurrence_exceptions: None,
        created_at: None,
        updated_at: None,
    }
}
