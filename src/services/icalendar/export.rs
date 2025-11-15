use crate::models::event::Event;
use anyhow::Result;
use chrono::{Local, Utc};

use super::utils::{escape_text, format_date, format_datetime};

pub(super) fn single(event: &Event) -> Result<String> {
    let mut ics = calendar_header();
    append_event(&mut ics, event);
    ics.push_str("END:VCALENDAR\r\n");
    Ok(ics)
}

pub(super) fn multiple(events: &[Event]) -> Result<String> {
    let mut ics = calendar_header();
    for event in events {
        append_event(&mut ics, event);
    }
    ics.push_str("END:VCALENDAR\r\n");
    Ok(ics)
}

fn calendar_header() -> String {
    let mut ics = String::new();
    ics.push_str("BEGIN:VCALENDAR\r\n");
    ics.push_str("VERSION:2.0\r\n");
    ics.push_str("PRODID:-//Rust Calendar//EN\r\n");
    ics.push_str("CALSCALE:GREGORIAN\r\n");
    ics
}

fn append_event(buffer: &mut String, event: &Event) {
    buffer.push_str("BEGIN:VEVENT\r\n");
    buffer.push_str(&format!("UID:{}\r\n", build_uid(event)));

    let dtstamp = event.created_at.unwrap_or_else(|| Local::now());
    buffer.push_str(&format!("DTSTAMP:{}\r\n", format_datetime(&dtstamp)));

    if event.all_day {
        buffer.push_str(&format!(
            "DTSTART;VALUE=DATE:{}\r\n",
            format_date(&event.start)
        ));
        buffer.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", format_date(&event.end)));
    } else {
        buffer.push_str(&format!("DTSTART:{}\r\n", format_datetime(&event.start)));
        buffer.push_str(&format!("DTEND:{}\r\n", format_datetime(&event.end)));
    }

    buffer.push_str(&format!("SUMMARY:{}\r\n", escape_text(&event.title)));

    if let Some(desc) = &event.description {
        buffer.push_str(&format!("DESCRIPTION:{}\r\n", escape_text(desc)));
    }
    if let Some(location) = &event.location {
        buffer.push_str(&format!("LOCATION:{}\r\n", escape_text(location)));
    }
    if let Some(category) = &event.category {
        buffer.push_str(&format!("CATEGORIES:{}\r\n", escape_text(category)));
    }
    if let Some(color) = &event.color {
        buffer.push_str(&format!("X-APPLE-CALENDAR-COLOR:{}\r\n", color));
    }
    if let Some(rrule) = &event.recurrence_rule {
        buffer.push_str(&format!("RRULE:{}\r\n", rrule));
    }
    if let Some(exceptions) = &event.recurrence_exceptions {
        if !exceptions.is_empty() {
            let exdates: Vec<String> = exceptions.iter().map(format_datetime).collect();
            buffer.push_str(&format!("EXDATE:{}\r\n", exdates.join(",")));
        }
    }
    if let Some(updated) = &event.updated_at {
        buffer.push_str(&format!("LAST-MODIFIED:{}\r\n", format_datetime(updated)));
    }
    if let Some(created) = &event.created_at {
        buffer.push_str(&format!("CREATED:{}\r\n", format_datetime(created)));
    }

    buffer.push_str("END:VEVENT\r\n");
}

fn build_uid(event: &Event) -> String {
    if let Some(id) = event.id {
        format!("rust-calendar-{}", id)
    } else {
        format!("rust-calendar-temp-{}", Utc::now().timestamp())
    }
}
