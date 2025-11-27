// Event Template model
// Stores reusable event configurations

use chrono::{DateTime, Local};

/// Event template for quick event creation
#[derive(Debug, Clone, PartialEq)]
pub struct EventTemplate {
    pub id: Option<i64>,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub duration_minutes: i32,
    pub all_day: bool,
    pub category: Option<String>,
    pub color: Option<String>,
    pub recurrence_rule: Option<String>,
    pub created_at: Option<DateTime<Local>>,
}

impl EventTemplate {
    /// Create a new template with required fields
    pub fn new(name: impl Into<String>, title: impl Into<String>, duration_minutes: i32) -> Self {
        Self {
            id: None,
            name: name.into(),
            title: title.into(),
            description: None,
            location: None,
            duration_minutes,
            all_day: false,
            category: None,
            color: None,
            recurrence_rule: None,
            created_at: None,
        }
    }

    /// Create a builder for constructing templates
    pub fn builder() -> EventTemplateBuilder {
        EventTemplateBuilder::new()
    }

    /// Validate the template
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Template name cannot be empty".to_string());
        }

        if self.title.trim().is_empty() {
            return Err("Event title cannot be empty".to_string());
        }

        if self.duration_minutes < 1 && !self.all_day {
            return Err("Duration must be at least 1 minute".to_string());
        }

        if self.duration_minutes > 24 * 60 * 7 {
            return Err("Duration cannot exceed 1 week".to_string());
        }

        // Validate color format if present
        if let Some(ref color) = self.color {
            if !color.starts_with('#') || (color.len() != 7 && color.len() != 4) {
                return Err("Color must be in hex format (#RRGGBB or #RGB)".to_string());
            }
        }

        Ok(())
    }
}

/// Builder for creating event templates
pub struct EventTemplateBuilder {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    location: Option<String>,
    duration_minutes: i32,
    all_day: bool,
    category: Option<String>,
    color: Option<String>,
    recurrence_rule: Option<String>,
}

impl EventTemplateBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            title: None,
            description: None,
            location: None,
            duration_minutes: 60,
            all_day: false,
            category: None,
            color: None,
            recurrence_rule: None,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    pub fn duration_minutes(mut self, minutes: i32) -> Self {
        self.duration_minutes = minutes;
        self
    }

    pub fn all_day(mut self, all_day: bool) -> Self {
        self.all_day = all_day;
        self
    }

    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn recurrence_rule(mut self, rule: impl Into<String>) -> Self {
        self.recurrence_rule = Some(rule.into());
        self
    }

    pub fn build(self) -> Result<EventTemplate, String> {
        let name = self.name.ok_or("Template name is required")?;
        let title = self.title.ok_or("Event title is required")?;

        let template = EventTemplate {
            id: None,
            name,
            title,
            description: self.description,
            location: self.location,
            duration_minutes: self.duration_minutes,
            all_day: self.all_day,
            category: self.category,
            color: self.color,
            recurrence_rule: self.recurrence_rule,
            created_at: None,
        };

        template.validate()?;
        Ok(template)
    }
}

impl Default for EventTemplateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_template() {
        let template = EventTemplate::new("Meeting Template", "Team Meeting", 60);
        assert_eq!(template.name, "Meeting Template");
        assert_eq!(template.title, "Team Meeting");
        assert_eq!(template.duration_minutes, 60);
        assert!(!template.all_day);
    }

    #[test]
    fn test_builder_basic() {
        let template = EventTemplate::builder()
            .name("Quick Sync")
            .title("Daily Standup")
            .duration_minutes(15)
            .build()
            .unwrap();

        assert_eq!(template.name, "Quick Sync");
        assert_eq!(template.title, "Daily Standup");
        assert_eq!(template.duration_minutes, 15);
    }

    #[test]
    fn test_builder_full() {
        let template = EventTemplate::builder()
            .name("Full Day Workshop")
            .title("Training Workshop")
            .description("Annual training session")
            .location("Conference Room A")
            .all_day(true)
            .category("Training")
            .color("#FF5733")
            .build()
            .unwrap();

        assert_eq!(template.description, Some("Annual training session".to_string()));
        assert_eq!(template.location, Some("Conference Room A".to_string()));
        assert!(template.all_day);
    }

    #[test]
    fn test_validate_empty_name() {
        let mut template = EventTemplate::new("Test", "Event", 60);
        template.name = String::new();
        assert!(template.validate().is_err());
    }

    #[test]
    fn test_validate_empty_title() {
        let mut template = EventTemplate::new("Test", "Event", 60);
        template.title = String::new();
        assert!(template.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_color() {
        let mut template = EventTemplate::new("Test", "Event", 60);
        template.color = Some("red".to_string());
        assert!(template.validate().is_err());
    }

    #[test]
    fn test_validate_valid_color() {
        let mut template = EventTemplate::new("Test", "Event", 60);
        template.color = Some("#FF5733".to_string());
        assert!(template.validate().is_ok());
    }
}
