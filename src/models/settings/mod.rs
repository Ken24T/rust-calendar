// Settings module
// User preferences and application settings

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub id: Option<i64>,
    pub theme: String,
    pub first_day_of_week: u8,
    pub first_day_of_work_week: u8,
    pub last_day_of_work_week: u8,
    pub time_format: String,
    pub date_format: String,
    pub show_my_day: bool,
    pub my_day_position_right: bool,
    pub show_ribbon: bool,
    pub current_view: String,
    pub default_event_duration: u32,
    pub default_event_start_time: String,
    pub default_card_width: f32,
    pub default_card_height: f32,
    pub auto_create_countdown_on_import: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: Some(1),
            theme: "light".to_string(),
            first_day_of_week: 0,      // Sunday
            first_day_of_work_week: 1, // Monday
            last_day_of_work_week: 5,  // Friday
            time_format: "12h".to_string(),
            date_format: "DD/MM/YYYY".to_string(),
            show_my_day: false,
            my_day_position_right: false,
            show_ribbon: false,
            current_view: "Month".to_string(),
            default_event_duration: 60,
            default_event_start_time: "08:00".to_string(),
            default_card_width: 120.0,
            default_card_height: 110.0,
            auto_create_countdown_on_import: false,
        }
    }
}

impl Settings {
    /// Validate settings values
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<(), String> {
        // Validate theme
        if !["light", "dark"].contains(&self.theme.as_str()) {
            return Err(format!("Invalid theme: {}", self.theme));
        }

        // Validate first_day_of_week (0-6, Sunday to Saturday)
        if self.first_day_of_week > 6 {
            return Err(format!(
                "Invalid first_day_of_week: {}",
                self.first_day_of_week
            ));
        }

        // Validate work week days (1-5 for Monday to Friday)
        if self.first_day_of_work_week < 1 || self.first_day_of_work_week > 5 {
            return Err(format!(
                "Invalid first_day_of_work_week: {}",
                self.first_day_of_work_week
            ));
        }
        if self.last_day_of_work_week < 1 || self.last_day_of_work_week > 5 {
            return Err(format!(
                "Invalid last_day_of_work_week: {}",
                self.last_day_of_work_week
            ));
        }
        if self.first_day_of_work_week > self.last_day_of_work_week {
            return Err("first_day_of_work_week cannot be after last_day_of_work_week".to_string());
        }

        // Validate time_format
        if !["12h", "24h"].contains(&self.time_format.as_str()) {
            return Err(format!("Invalid time_format: {}", self.time_format));
        }

        // Validate current_view
        if !["Day", "WorkWeek", "Week", "Month", "Quarter"].contains(&self.current_view.as_str()) {
            return Err(format!("Invalid current_view: {}", self.current_view));
        }

        // Validate default_event_duration (15, 30, 45, 60, 90, 120 minutes)
        if ![15, 30, 45, 60, 90, 120].contains(&self.default_event_duration) {
            return Err(format!(
                "Invalid default_event_duration: {}",
                self.default_event_duration
            ));
        }

        // Validate default_event_start_time format (HH:MM)
        if !self.default_event_start_time.contains(':') {
            return Err("Invalid default_event_start_time format".to_string());
        }

        Self::validate_card_dimensions(self.default_card_width, self.default_card_height)?;

        Ok(())
    }

    /// Validate settings values without checking theme (theme validated by app.rs)
    pub fn validate_without_theme(&self) -> Result<(), String> {
        // Validate first_day_of_week (0-6, Sunday to Saturday)
        if self.first_day_of_week > 6 {
            return Err(format!(
                "Invalid first_day_of_week: {}",
                self.first_day_of_week
            ));
        }

        // Validate work week days (1-5 for Monday to Friday)
        if self.first_day_of_work_week < 1 || self.first_day_of_work_week > 5 {
            return Err(format!(
                "Invalid first_day_of_work_week: {}",
                self.first_day_of_work_week
            ));
        }
        if self.last_day_of_work_week < 1 || self.last_day_of_work_week > 5 {
            return Err(format!(
                "Invalid last_day_of_work_week: {}",
                self.last_day_of_work_week
            ));
        }
        if self.first_day_of_work_week > self.last_day_of_work_week {
            return Err("first_day_of_work_week cannot be after last_day_of_work_week".to_string());
        }

        // Validate time_format
        if !["12h", "24h"].contains(&self.time_format.as_str()) {
            return Err(format!("Invalid time_format: {}", self.time_format));
        }

        // Validate current_view
        if !["Day", "WorkWeek", "Week", "Month", "Quarter"].contains(&self.current_view.as_str()) {
            return Err(format!("Invalid current_view: {}", self.current_view));
        }

        // Validate default_event_duration (15, 30, 45, 60, 90, 120 minutes)
        if ![15, 30, 45, 60, 90, 120].contains(&self.default_event_duration) {
            return Err(format!(
                "Invalid default_event_duration: {}",
                self.default_event_duration
            ));
        }

        // Validate default_event_start_time format (HH:MM)
        if !self.default_event_start_time.contains(':') {
            return Err("Invalid default_event_start_time format".to_string());
        }

        Self::validate_card_dimensions(self.default_card_width, self.default_card_height)?;

        Ok(())
    }

    fn validate_card_dimensions(width: f32, height: f32) -> Result<(), String> {
        const MIN_WIDTH: f32 = 20.0;
        const MAX_WIDTH: f32 = 600.0;
        const MIN_HEIGHT: f32 = 20.0;
        const MAX_HEIGHT: f32 = 600.0;

        if !(MIN_WIDTH..=MAX_WIDTH).contains(&width) {
            return Err(format!(
                "Invalid default_card_width: {:.1} (expected between {:.0} and {:.0})",
                width, MIN_WIDTH, MAX_WIDTH
            ));
        }

        if !(MIN_HEIGHT..=MAX_HEIGHT).contains(&height) {
            return Err(format!(
                "Invalid default_card_height: {:.1} (expected between {:.0} and {:.0})",
                height, MIN_HEIGHT, MAX_HEIGHT
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.id, Some(1));
        assert_eq!(settings.theme, "light");
        assert_eq!(settings.first_day_of_week, 0);
        assert_eq!(settings.time_format, "12h");
        assert_eq!(settings.date_format, "DD/MM/YYYY");
        assert_eq!(settings.show_my_day, false);
        assert_eq!(settings.show_ribbon, false);
        assert_eq!(settings.current_view, "Month");
    }

    #[test]
    fn test_validate_valid_settings() {
        let settings = Settings::default();
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_theme() {
        let mut settings = Settings::default();
        settings.theme = "invalid".to_string();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_first_day_of_week() {
        let mut settings = Settings::default();
        settings.first_day_of_week = 7;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_time_format() {
        let mut settings = Settings::default();
        settings.time_format = "invalid".to_string();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_view() {
        let mut settings = Settings::default();
        settings.current_view = "Invalid".to_string();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_validate_all_valid_themes() {
        for theme in ["light", "dark"] {
            let mut settings = Settings::default();
            settings.theme = theme.to_string();
            assert!(
                settings.validate().is_ok(),
                "Theme '{}' should be valid",
                theme
            );
        }
    }

    #[test]
    fn test_validate_all_valid_views() {
        for view in ["Day", "WorkWeek", "Week", "Month", "Quarter"] {
            let mut settings = Settings::default();
            settings.current_view = view.to_string();
            assert!(
                settings.validate().is_ok(),
                "View '{}' should be valid",
                view
            );
        }
    }
}
