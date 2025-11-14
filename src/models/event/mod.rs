// Event module
// Calendar event model with iCalendar compatibility

use chrono::{DateTime, Local};

/// Calendar event with iCalendar (RFC 5545) compatibility
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub all_day: bool,
    pub category: Option<String>,
    pub color: Option<String>,
    pub recurrence_rule: Option<String>, // RRULE string (RFC 5545)
    pub recurrence_exceptions: Option<Vec<DateTime<Local>>>, // Exception dates
    pub created_at: Option<DateTime<Local>>,
    pub updated_at: Option<DateTime<Local>>,
}

impl Event {
    /// Create a new event with required fields
    ///
    /// # Arguments
    /// * `title` - Event title (required, non-empty)
    /// * `start` - Event start time
    /// * `end` - Event end time
    ///
    /// # Returns
    /// Returns `Result<Event, String>` with validation
    ///
    /// # Examples
    /// ```
    /// use rust_calendar::models::event::Event;
    /// use chrono::Local;
    ///
    /// let start = Local::now();
    /// let end = start + chrono::Duration::hours(1);
    /// let event = Event::new("Team Meeting", start, end).unwrap();
    /// ```
    pub fn new(
        title: impl Into<String>,
        start: DateTime<Local>,
        end: DateTime<Local>,
    ) -> Result<Self, String> {
        let title = title.into();

        // Validate title
        if title.trim().is_empty() {
            return Err("Event title cannot be empty".to_string());
        }

        // Validate times
        if end <= start {
            return Err("Event end time must be after start time".to_string());
        }

        Ok(Self {
            id: None,
            title,
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
        })
    }

    /// Create a builder for constructing events with optional fields
    pub fn builder() -> EventBuilder {
        EventBuilder::new()
    }

    /// Validate the event
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("Event title cannot be empty".to_string());
        }

        if self.end <= self.start {
            return Err("Event end time must be after start time".to_string());
        }

        // Validate color format if present (should be hex color)
        if let Some(ref color) = self.color {
            if !color.starts_with('#') || (color.len() != 7 && color.len() != 4) {
                return Err("Color must be in hex format (#RRGGBB or #RGB)".to_string());
            }
        }

        Ok(())
    }

    /// Check if this is a recurring event
    pub fn is_recurring(&self) -> bool {
        self.recurrence_rule.is_some()
    }

    /// Get the duration of the event
    pub fn duration(&self) -> chrono::Duration {
        self.end - self.start
    }
}

/// Builder for creating events with optional fields
pub struct EventBuilder {
    title: Option<String>,
    description: Option<String>,
    location: Option<String>,
    start: Option<DateTime<Local>>,
    end: Option<DateTime<Local>>,
    all_day: bool,
    category: Option<String>,
    color: Option<String>,
    recurrence_rule: Option<String>,
}

impl EventBuilder {
    /// Create a new event builder
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            location: None,
            start: None,
            end: None,
            all_day: false,
            category: None,
            color: None,
            recurrence_rule: None,
        }
    }

    /// Set the event title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the event description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the event location
    pub fn location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Set the start time
    pub fn start(mut self, start: DateTime<Local>) -> Self {
        self.start = Some(start);
        self
    }

    /// Set the end time
    pub fn end(mut self, end: DateTime<Local>) -> Self {
        self.end = Some(end);
        self
    }

    /// Set as all-day event
    pub fn all_day(mut self, all_day: bool) -> Self {
        self.all_day = all_day;
        self
    }

    /// Set the event category
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set the event color (hex format)
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the recurrence rule (RRULE format)
    pub fn recurrence_rule(mut self, rule: impl Into<String>) -> Self {
        self.recurrence_rule = Some(rule.into());
        self
    }

    /// Build the event
    pub fn build(self) -> Result<Event, String> {
        let title = self.title.ok_or("Event title is required")?;
        let start = self.start.ok_or("Event start time is required")?;
        let end = self.end.ok_or("Event end time is required")?;

        let event = Event {
            id: None,
            title,
            description: self.description,
            location: self.location,
            start,
            end,
            all_day: self.all_day,
            category: self.category,
            color: self.color,
            recurrence_rule: self.recurrence_rule,
            recurrence_exceptions: None,
            created_at: None,
            updated_at: None,
        };

        event.validate()?;
        Ok(event)
    }
}

impl Default for EventBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_start() -> DateTime<Local> {
        Local::now()
    }

    fn sample_end() -> DateTime<Local> {
        Local::now() + Duration::hours(1)
    }

    #[test]
    fn test_new_event_success() {
        let start = sample_start();
        let end = sample_end();
        let result = Event::new("Meeting", start, end);

        assert!(result.is_ok());
        let event = result.unwrap();
        assert_eq!(event.title, "Meeting");
        assert_eq!(event.start, start);
        assert_eq!(event.end, end);
        assert!(!event.all_day);
        assert!(event.description.is_none());
    }

    #[test]
    fn test_new_event_empty_title() {
        let result = Event::new("", sample_start(), sample_end());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Event title cannot be empty");
    }

    #[test]
    fn test_new_event_whitespace_title() {
        let result = Event::new("   ", sample_start(), sample_end());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Event title cannot be empty");
    }

    #[test]
    fn test_new_event_invalid_times() {
        let start = sample_start();
        let end = start - Duration::hours(1);
        let result = Event::new("Meeting", start, end);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Event end time must be after start time"
        );
    }

    #[test]
    fn test_new_event_equal_times() {
        let start = sample_start();
        let result = Event::new("Meeting", start, start);

        assert!(result.is_err());
    }

    #[test]
    fn test_builder_basic() {
        let start = sample_start();
        let end = sample_end();

        let result = Event::builder()
            .title("Team Standup")
            .start(start)
            .end(end)
            .build();

        assert!(result.is_ok());
        let event = result.unwrap();
        assert_eq!(event.title, "Team Standup");
        assert_eq!(event.start, start);
        assert_eq!(event.end, end);
    }

    #[test]
    fn test_builder_with_optional_fields() {
        let start = sample_start();
        let end = sample_end();

        let event = Event::builder()
            .title("Conference")
            .description("Annual tech conference")
            .location("Convention Center")
            .start(start)
            .end(end)
            .category("Work")
            .color("#FF5733")
            .build()
            .unwrap();

        assert_eq!(event.title, "Conference");
        assert_eq!(
            event.description,
            Some("Annual tech conference".to_string())
        );
        assert_eq!(event.location, Some("Convention Center".to_string()));
        assert_eq!(event.category, Some("Work".to_string()));
        assert_eq!(event.color, Some("#FF5733".to_string()));
    }

    #[test]
    fn test_builder_missing_title() {
        let result = Event::builder()
            .start(sample_start())
            .end(sample_end())
            .build();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Event title is required");
    }

    #[test]
    fn test_builder_missing_start() {
        let result = Event::builder().title("Meeting").end(sample_end()).build();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Event start time is required");
    }

    #[test]
    fn test_builder_missing_end() {
        let result = Event::builder()
            .title("Meeting")
            .start(sample_start())
            .build();

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Event end time is required");
    }

    #[test]
    fn test_validate_success() {
        let event = Event::new("Meeting", sample_start(), sample_end()).unwrap();
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_color() {
        let mut event = Event::new("Meeting", sample_start(), sample_end()).unwrap();
        event.color = Some("red".to_string());

        let result = event.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("hex format"));
    }

    #[test]
    fn test_validate_valid_color_long() {
        let mut event = Event::new("Meeting", sample_start(), sample_end()).unwrap();
        event.color = Some("#FF5733".to_string());
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_validate_valid_color_short() {
        let mut event = Event::new("Meeting", sample_start(), sample_end()).unwrap();
        event.color = Some("#F57".to_string());
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_is_recurring_false() {
        let event = Event::new("Meeting", sample_start(), sample_end()).unwrap();
        assert!(!event.is_recurring());
    }

    #[test]
    fn test_is_recurring_true() {
        let mut event = Event::new("Meeting", sample_start(), sample_end()).unwrap();
        event.recurrence_rule = Some("FREQ=DAILY".to_string());
        assert!(event.is_recurring());
    }

    #[test]
    fn test_duration() {
        let start = sample_start();
        let end = start + Duration::hours(2);
        let event = Event::new("Meeting", start, end).unwrap();

        assert_eq!(event.duration(), Duration::hours(2));
    }

    #[test]
    fn test_all_day_event() {
        let event = Event::builder()
            .title("Holiday")
            .start(sample_start())
            .end(sample_end())
            .all_day(true)
            .build()
            .unwrap();

        assert!(event.all_day);
    }

    #[test]
    fn test_recurrence_rule() {
        let event = Event::builder()
            .title("Weekly Meeting")
            .start(sample_start())
            .end(sample_end())
            .recurrence_rule("FREQ=WEEKLY;BYDAY=MO")
            .build()
            .unwrap();

        assert_eq!(
            event.recurrence_rule,
            Some("FREQ=WEEKLY;BYDAY=MO".to_string())
        );
        assert!(event.is_recurring());
    }
}
