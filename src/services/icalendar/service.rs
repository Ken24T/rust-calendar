use super::{export, import};
use crate::models::event::Event;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Service for importing and exporting iCalendar (.ics) files
pub struct ICalendarService;

impl ICalendarService {
    /// Create a new ICalendarService
    pub fn new() -> Self {
        Self
    }

    /// Export a single event to an iCalendar formatted string
    pub fn export_event(&self, event: &Event) -> Result<String> {
        export::single(event)
    }

    /// Export multiple events to an iCalendar formatted string
    pub fn export_events(&self, events: &[Event]) -> Result<String> {
        export::multiple(events)
    }

    /// Import events from an iCalendar formatted string
    pub fn import_events(&self, ics_content: &str) -> Result<Vec<Event>> {
        import::from_str(ics_content)
    }

    /// Import events from a .ics file on disk
    pub fn import_from_file(&self, path: &Path) -> Result<Vec<Event>> {
        let content =
            fs::read_to_string(path).context(format!("Failed to read .ics file: {:?}", path))?;
        self.import_events(&content)
    }

    /// Export an event to a .ics file on disk
    pub fn export_to_file(&self, event: &Event, path: &Path) -> Result<()> {
        let content = self.export_event(event)?;
        fs::write(path, content).context(format!("Failed to write .ics file: {:?}", path))?;
        Ok(())
    }

    /// Export multiple events to a .ics file on disk
    pub fn export_events_to_file(&self, events: &[Event], path: &Path) -> Result<()> {
        let content = self.export_events(events)?;
        fs::write(path, content).context(format!("Failed to write .ics file: {:?}", path))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ICalendarService;
    use crate::models::event::Event;
    use chrono::{Duration, Local, TimeZone};

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
        assert_eq!(
            events[0].recurrence_rule,
            Some("FREQ=WEEKLY;BYDAY=MO".to_string())
        );
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
        assert_eq!(ics.matches("BEGIN:VCALENDAR").count(), 1);
        assert_eq!(ics.matches("END:VCALENDAR").count(), 1);
        assert_eq!(ics.matches("BEGIN:VEVENT").count(), 2);
        assert_eq!(ics.matches("END:VEVENT").count(), 2);
    }

    #[test]
    fn test_round_trip() {
        let service = ICalendarService::new();
        let original = sample_event();

        let ics = service.export_event(&original).unwrap();
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
        use super::super::utils::escape_text;

        let text = "Line1\nLine2,with,commas;and;semicolons\\backslash";
        let escaped = escape_text(text);
        assert!(escaped.contains("\\n"));
        assert!(escaped.contains("\\,"));
        assert!(escaped.contains("\\;"));
        assert!(escaped.contains("\\\\"));
    }

    #[test]
    fn test_unescape_text() {
        use super::super::utils::unescape_text;

        let escaped = "Line1\\nLine2\\,with\\,commas\\;and\\;semicolons\\\\backslash";
        let unescaped = unescape_text(escaped);
        assert!(unescaped.contains("\n"));
        assert!(unescaped.contains(",with,commas"));
        assert!(unescaped.contains(";and;semicolons"));
        assert!(unescaped.contains("\\backslash"));
    }
}
