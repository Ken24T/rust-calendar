// iCalendar service module
// RFC 5545 .ics file import/export functionality

use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeZone};
use crate::models::event::Event;
use std::fs;
use std::path::Path;

/// Service for importing and exporting iCalendar (.ics) files
pub struct ICalendarService;

impl ICalendarService {
    /// Create a new ICalendarService
    pub fn new() -> Self {
        Self
    }

    /// Export an event to iCalendar format string
    pub fn export_event(&self, event: &Event) -> Result<String> {
        let mut ics = String::new();
        
        // iCalendar header
        ics.push_str("BEGIN:VCALENDAR\r\n");
        ics.push_str("VERSION:2.0\r\n");
        ics.push_str("PRODID:-//Rust Calendar//EN\r\n");
        ics.push_str("CALSCALE:GREGORIAN\r\n");
        
        // VEVENT
        ics.push_str("BEGIN:VEVENT\r\n");
        
        // UID - use database ID if available, otherwise generate
        if let Some(id) = event.id {
            ics.push_str(&format!("UID:rust-calendar-{}\r\n", id));
        } else {
            ics.push_str(&format!("UID:rust-calendar-temp-{}\r\n", chrono::Utc::now().timestamp()));
        }
        
        // DTSTAMP - creation timestamp
        let dtstamp = event.created_at.unwrap_or_else(|| Local::now());
        ics.push_str(&format!("DTSTAMP:{}\r\n", Self::format_datetime(&dtstamp)));
        
        // DTSTART and DTEND
        if event.all_day {
            // All-day events use DATE format (YYYYMMDD)
            ics.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", Self::format_date(&event.start)));
            ics.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", Self::format_date(&event.end)));
        } else {
            // Regular events use DATETIME format
            ics.push_str(&format!("DTSTART:{}\r\n", Self::format_datetime(&event.start)));
            ics.push_str(&format!("DTEND:{}\r\n", Self::format_datetime(&event.end)));
        }
        
        // SUMMARY (title)
        ics.push_str(&format!("SUMMARY:{}\r\n", Self::escape_text(&event.title)));
        
        // DESCRIPTION (optional)
        if let Some(desc) = &event.description {
            ics.push_str(&format!("DESCRIPTION:{}\r\n", Self::escape_text(desc)));
        }
        
        // LOCATION (optional)
        if let Some(location) = &event.location {
            ics.push_str(&format!("LOCATION:{}\r\n", Self::escape_text(location)));
        }
        
        // CATEGORIES (optional)
        if let Some(category) = &event.category {
            ics.push_str(&format!("CATEGORIES:{}\r\n", Self::escape_text(category)));
        }
        
        // COLOR (optional, X-APPLE-CALENDAR-COLOR extension)
        if let Some(color) = &event.color {
            ics.push_str(&format!("X-APPLE-CALENDAR-COLOR:{}\r\n", color));
        }
        
        // RRULE (recurrence rule, optional)
        if let Some(rrule) = &event.recurrence_rule {
            ics.push_str(&format!("RRULE:{}\r\n", rrule));
        }
        
        // EXDATE (exception dates, optional)
        if let Some(exceptions) = &event.recurrence_exceptions {
            if !exceptions.is_empty() {
                let exdates: Vec<String> = exceptions.iter()
                    .map(|dt| Self::format_datetime(dt))
                    .collect();
                ics.push_str(&format!("EXDATE:{}\r\n", exdates.join(",")));
            }
        }
        
        // LAST-MODIFIED
        if let Some(updated) = &event.updated_at {
            ics.push_str(&format!("LAST-MODIFIED:{}\r\n", Self::format_datetime(updated)));
        }
        
        // CREATED
        if let Some(created) = &event.created_at {
            ics.push_str(&format!("CREATED:{}\r\n", Self::format_datetime(created)));
        }
        
        ics.push_str("END:VEVENT\r\n");
        ics.push_str("END:VCALENDAR\r\n");
        
        Ok(ics)
    }

    /// Export multiple events to a single iCalendar file
    pub fn export_events(&self, events: &[Event]) -> Result<String> {
        let mut ics = String::new();
        
        // iCalendar header
        ics.push_str("BEGIN:VCALENDAR\r\n");
        ics.push_str("VERSION:2.0\r\n");
        ics.push_str("PRODID:-//Rust Calendar//EN\r\n");
        ics.push_str("CALSCALE:GREGORIAN\r\n");
        
        // Add all events
        for event in events {
            ics.push_str("BEGIN:VEVENT\r\n");
            
            // UID
            if let Some(id) = event.id {
                ics.push_str(&format!("UID:rust-calendar-{}\r\n", id));
            } else {
                ics.push_str(&format!("UID:rust-calendar-temp-{}\r\n", chrono::Utc::now().timestamp()));
            }
            
            // DTSTAMP
            let dtstamp = event.created_at.unwrap_or_else(|| Local::now());
            ics.push_str(&format!("DTSTAMP:{}\r\n", Self::format_datetime(&dtstamp)));
            
            // DTSTART and DTEND
            if event.all_day {
                ics.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", Self::format_date(&event.start)));
                ics.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", Self::format_date(&event.end)));
            } else {
                ics.push_str(&format!("DTSTART:{}\r\n", Self::format_datetime(&event.start)));
                ics.push_str(&format!("DTEND:{}\r\n", Self::format_datetime(&event.end)));
            }
            
            // SUMMARY
            ics.push_str(&format!("SUMMARY:{}\r\n", Self::escape_text(&event.title)));
            
            // Optional fields
            if let Some(desc) = &event.description {
                ics.push_str(&format!("DESCRIPTION:{}\r\n", Self::escape_text(desc)));
            }
            if let Some(location) = &event.location {
                ics.push_str(&format!("LOCATION:{}\r\n", Self::escape_text(location)));
            }
            if let Some(category) = &event.category {
                ics.push_str(&format!("CATEGORIES:{}\r\n", Self::escape_text(category)));
            }
            if let Some(color) = &event.color {
                ics.push_str(&format!("X-APPLE-CALENDAR-COLOR:{}\r\n", color));
            }
            if let Some(rrule) = &event.recurrence_rule {
                ics.push_str(&format!("RRULE:{}\r\n", rrule));
            }
            if let Some(exceptions) = &event.recurrence_exceptions {
                if !exceptions.is_empty() {
                    let exdates: Vec<String> = exceptions.iter()
                        .map(|dt| Self::format_datetime(dt))
                        .collect();
                    ics.push_str(&format!("EXDATE:{}\r\n", exdates.join(",")));
                }
            }
            if let Some(updated) = &event.updated_at {
                ics.push_str(&format!("LAST-MODIFIED:{}\r\n", Self::format_datetime(updated)));
            }
            if let Some(created) = &event.created_at {
                ics.push_str(&format!("CREATED:{}\r\n", Self::format_datetime(created)));
            }
            
            ics.push_str("END:VEVENT\r\n");
        }
        
        ics.push_str("END:VCALENDAR\r\n");
        
        Ok(ics)
    }

    /// Import events from iCalendar format string
    pub fn import_events(&self, ics_content: &str) -> Result<Vec<Event>> {
        let mut events = Vec::new();
        let lines: Vec<&str> = ics_content.lines().collect();
        
        let mut in_event = false;
        let mut current_event: Option<Event> = None;
        
        for line in lines {
            let line = line.trim();
            
            if line == "BEGIN:VEVENT" {
                in_event = true;
                current_event = Some(Event {
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
                });
            } else if line == "END:VEVENT" {
                in_event = false;
                if let Some(event) = current_event.take() {
                    // Validate that required fields are present
                    if !event.title.is_empty() {
                        events.push(event);
                    }
                }
            } else if in_event {
                if let Some(event) = current_event.as_mut() {
                    Self::parse_event_property(line, event)?;
                }
            }
        }
        
        Ok(events)
    }

    /// Import events from a .ics file
    pub fn import_from_file(&self, path: &Path) -> Result<Vec<Event>> {
        let content = fs::read_to_string(path)
            .context(format!("Failed to read .ics file: {:?}", path))?;
        self.import_events(&content)
    }

    /// Export event to a .ics file
    pub fn export_to_file(&self, event: &Event, path: &Path) -> Result<()> {
        let content = self.export_event(event)?;
        fs::write(path, content)
            .context(format!("Failed to write .ics file: {:?}", path))?;
        Ok(())
    }

    /// Export multiple events to a .ics file
    pub fn export_events_to_file(&self, events: &[Event], path: &Path) -> Result<()> {
        let content = self.export_events(events)?;
        fs::write(path, content)
            .context(format!("Failed to write .ics file: {:?}", path))?;
        Ok(())
    }

    // Helper methods

    /// Format datetime in iCalendar format (YYYYMMDDTHHMMSSZ)
    fn format_datetime(dt: &DateTime<Local>) -> String {
        dt.format("%Y%m%dT%H%M%S").to_string()
    }

    /// Format date in iCalendar format (YYYYMMDD)
    fn format_date(dt: &DateTime<Local>) -> String {
        dt.format("%Y%m%d").to_string()
    }

    /// Escape special characters in text fields
    fn escape_text(text: &str) -> String {
        text.replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace(',', "\\,")
            .replace(';', "\\;")
    }

    /// Unescape text from iCalendar format
    fn unescape_text(text: &str) -> String {
        text.replace("\\n", "\n")
            .replace("\\,", ",")
            .replace("\\;", ";")
            .replace("\\\\", "\\")
    }

    /// Parse a single event property line
    fn parse_event_property(line: &str, event: &mut Event) -> Result<()> {
        if let Some(colon_pos) = line.find(':') {
            let (key_part, value) = line.split_at(colon_pos);
            let value = &value[1..]; // Skip the colon
            
            // Handle parameters (e.g., DTSTART;VALUE=DATE:20250101)
            let key = if let Some(semicolon) = key_part.find(';') {
                &key_part[..semicolon]
            } else {
                key_part
            };
            
            match key {
                "SUMMARY" => {
                    event.title = Self::unescape_text(value);
                }
                "DESCRIPTION" => {
                    event.description = Some(Self::unescape_text(value));
                }
                "LOCATION" => {
                    event.location = Some(Self::unescape_text(value));
                }
                "CATEGORIES" => {
                    event.category = Some(Self::unescape_text(value));
                }
                "X-APPLE-CALENDAR-COLOR" => {
                    event.color = Some(value.to_string());
                }
                "DTSTART" => {
                    if key_part.contains("VALUE=DATE") {
                        event.all_day = true;
                        event.start = Self::parse_date(value)?;
                    } else {
                        event.start = Self::parse_datetime(value)?;
                    }
                }
                "DTEND" => {
                    if key_part.contains("VALUE=DATE") {
                        event.end = Self::parse_date(value)?;
                    } else {
                        event.end = Self::parse_datetime(value)?;
                    }
                }
                "RRULE" => {
                    event.recurrence_rule = Some(value.to_string());
                }
                "EXDATE" => {
                    let dates: Result<Vec<DateTime<Local>>> = value.split(',')
                        .map(|s| Self::parse_datetime(s.trim()))
                        .collect();
                    event.recurrence_exceptions = Some(dates?);
                }
                "CREATED" => {
                    event.created_at = Some(Self::parse_datetime(value)?);
                }
                "LAST-MODIFIED" => {
                    event.updated_at = Some(Self::parse_datetime(value)?);
                }
                _ => {
                    // Ignore unknown properties
                }
            }
        }
        
        Ok(())
    }

    /// Parse iCalendar datetime format (YYYYMMDDTHHMMSS or YYYYMMDDTHHMMSSZ)
    fn parse_datetime(s: &str) -> Result<DateTime<Local>> {
        let s = s.trim_end_matches('Z'); // Remove trailing Z if present
        
        if s.len() < 15 {
            return Err(anyhow::anyhow!("Invalid datetime format: {}", s));
        }
        
        let year: i32 = s[0..4].parse()?;
        let month: u32 = s[4..6].parse()?;
        let day: u32 = s[6..8].parse()?;
        let hour: u32 = s[9..11].parse()?;
        let minute: u32 = s[11..13].parse()?;
        let second: u32 = s[13..15].parse()?;
        
        Local.with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .ok_or_else(|| anyhow::anyhow!("Invalid datetime: {}", s))
    }

    /// Parse iCalendar date format (YYYYMMDD)
    fn parse_date(s: &str) -> Result<DateTime<Local>> {
        if s.len() < 8 {
            return Err(anyhow::anyhow!("Invalid date format: {}", s));
        }
        
        let year: i32 = s[0..4].parse()?;
        let month: u32 = s[4..6].parse()?;
        let day: u32 = s[6..8].parse()?;
        
        Local.with_ymd_and_hms(year, month, day, 0, 0, 0)
            .single()
            .ok_or_else(|| anyhow::anyhow!("Invalid date: {}", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_event() -> Event {
        let start = Local.with_ymd_and_hms(2025, 11, 7, 14, 0, 0).unwrap();
        let end = start + Duration::hours(1);
        
        Event::builder()
            .title("Team Meeting")
            .description("Quarterly planning meeting")
            .location("Conference Room A")
            .start(start)
            .end(end)
            .category("Work")
            .color("#FF5733")
            .build()
            .unwrap()
    }

    #[test]
    fn test_export_event() {
        let service = ICalendarService::new();
        let event = sample_event();
        
        let result = service.export_event(&event);
        assert!(result.is_ok());
        
        let ics = result.unwrap();
        assert!(ics.contains("BEGIN:VCALENDAR"));
        assert!(ics.contains("BEGIN:VEVENT"));
        assert!(ics.contains("SUMMARY:Team Meeting"));
        assert!(ics.contains("DESCRIPTION:Quarterly planning meeting"));
        assert!(ics.contains("LOCATION:Conference Room A"));
        assert!(ics.contains("END:VEVENT"));
        assert!(ics.contains("END:VCALENDAR"));
    }

    #[test]
    fn test_export_all_day_event() {
        let service = ICalendarService::new();
        let start = Local.with_ymd_and_hms(2025, 11, 7, 0, 0, 0).unwrap();
        let end = start + Duration::days(1);
        
        let event = Event::builder()
            .title("All Day Event")
            .start(start)
            .end(end)
            .all_day(true)
            .build()
            .unwrap();
        
        let ics = service.export_event(&event).unwrap();
        assert!(ics.contains("DTSTART;VALUE=DATE:20251107"));
        assert!(ics.contains("DTEND;VALUE=DATE:20251108"));
    }

    #[test]
    fn test_export_with_recurrence() {
        let service = ICalendarService::new();
        let mut event = sample_event();
        event.recurrence_rule = Some("FREQ=WEEKLY;BYDAY=MO,WE,FR".to_string());
        
        let ics = service.export_event(&event).unwrap();
        assert!(ics.contains("RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR"));
    }

    #[test]
    fn test_import_basic_event() {
        let service = ICalendarService::new();
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//EN
BEGIN:VEVENT
UID:test-123
DTSTART:20251107T140000
DTEND:20251107T150000
SUMMARY:Test Event
DESCRIPTION:Test description
LOCATION:Test Location
END:VEVENT
END:VCALENDAR"#;
        
        let events = service.import_events(ics).unwrap();
        assert_eq!(events.len(), 1);
        
        let event = &events[0];
        assert_eq!(event.title, "Test Event");
        assert_eq!(event.description, Some("Test description".to_string()));
        assert_eq!(event.location, Some("Test Location".to_string()));
        assert!(!event.all_day);
    }

    #[test]
    fn test_import_all_day_event() {
        let service = ICalendarService::new();
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:test-allday
DTSTART;VALUE=DATE:20251107
DTEND;VALUE=DATE:20251108
SUMMARY:All Day Test
END:VEVENT
END:VCALENDAR"#;
        
        let events = service.import_events(ics).unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].all_day);
    }

    #[test]
    fn test_import_with_recurrence() {
        let service = ICalendarService::new();
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:test-recurring
DTSTART:20251107T140000
DTEND:20251107T150000
SUMMARY:Weekly Meeting
RRULE:FREQ=WEEKLY;BYDAY=MO
END:VEVENT
END:VCALENDAR"#;
        
        let events = service.import_events(ics).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].recurrence_rule, Some("FREQ=WEEKLY;BYDAY=MO".to_string()));
    }

    #[test]
    fn test_import_multiple_events() {
        let service = ICalendarService::new();
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:event-1
DTSTART:20251107T140000
DTEND:20251107T150000
SUMMARY:Event 1
END:VEVENT
BEGIN:VEVENT
UID:event-2
DTSTART:20251107T160000
DTEND:20251107T170000
SUMMARY:Event 2
END:VEVENT
END:VCALENDAR"#;
        
        let events = service.import_events(ics).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].title, "Event 1");
        assert_eq!(events[1].title, "Event 2");
    }

    #[test]
    fn test_export_multiple_events() {
        let service = ICalendarService::new();
        let event1 = Event::builder()
            .title("Event 1")
            .start(Local.with_ymd_and_hms(2025, 11, 7, 14, 0, 0).unwrap())
            .end(Local.with_ymd_and_hms(2025, 11, 7, 15, 0, 0).unwrap())
            .build()
            .unwrap();
        
        let event2 = Event::builder()
            .title("Event 2")
            .start(Local.with_ymd_and_hms(2025, 11, 7, 16, 0, 0).unwrap())
            .end(Local.with_ymd_and_hms(2025, 11, 7, 17, 0, 0).unwrap())
            .build()
            .unwrap();
        
        let ics = service.export_events(&[event1, event2]).unwrap();
        assert!(ics.contains("SUMMARY:Event 1"));
        assert!(ics.contains("SUMMARY:Event 2"));
        
        // Should have exactly one VCALENDAR wrapper
        assert_eq!(ics.matches("BEGIN:VCALENDAR").count(), 1);
        assert_eq!(ics.matches("END:VCALENDAR").count(), 1);
        
        // Should have two events
        assert_eq!(ics.matches("BEGIN:VEVENT").count(), 2);
        assert_eq!(ics.matches("END:VEVENT").count(), 2);
    }

    #[test]
    fn test_round_trip() {
        let service = ICalendarService::new();
        let original = sample_event();
        
        // Export to ICS
        let ics = service.export_event(&original).unwrap();
        
        // Import back
        let imported = service.import_events(&ics).unwrap();
        assert_eq!(imported.len(), 1);
        
        let imported_event = &imported[0];
        assert_eq!(imported_event.title, original.title);
        assert_eq!(imported_event.description, original.description);
        assert_eq!(imported_event.location, original.location);
        assert_eq!(imported_event.category, original.category);
    }

    #[test]
    fn test_escape_text() {
        let text = "Line1\nLine2,with,commas;and;semicolons\\backslash";
        let escaped = ICalendarService::escape_text(text);
        assert!(escaped.contains("\\n"));
        assert!(escaped.contains("\\,"));
        assert!(escaped.contains("\\;"));
        assert!(escaped.contains("\\\\"));
    }

    #[test]
    fn test_unescape_text() {
        let escaped = "Line1\\nLine2\\,with\\,commas\\;and\\;semicolons\\\\backslash";
        let unescaped = ICalendarService::unescape_text(escaped);
        assert!(unescaped.contains("\n"));
        assert!(unescaped.contains(",with,commas"));
        assert!(unescaped.contains(";and;semicolons"));
        assert!(unescaped.contains("\\backslash"));
    }
}
