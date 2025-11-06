// Settings module
// User preferences and application settings

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub id: Option<i64>,
    pub theme: String,
    pub first_day_of_week: u8,
    pub time_format: String,
    pub date_format: String,
    pub show_my_day: bool,
    pub my_day_position_right: bool,
    pub show_ribbon: bool,
    pub current_view: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: Some(1),
            theme: "light".to_string(),
            first_day_of_week: 0, // Sunday
            time_format: "12h".to_string(),
            date_format: "MM/DD/YYYY".to_string(),
            show_my_day: false,
            my_day_position_right: false,
            show_ribbon: false,
            current_view: "Month".to_string(),
        }
    }
}

impl Settings {
    /// Validate settings values
    pub fn validate(&self) -> Result<(), String> {
        // Validate theme
        if !["light", "dark"].contains(&self.theme.as_str()) {
            return Err(format!("Invalid theme: {}", self.theme));
        }
        
        // Validate first_day_of_week (0-6, Sunday to Saturday)
        if self.first_day_of_week > 6 {
            return Err(format!("Invalid first_day_of_week: {}", self.first_day_of_week));
        }
        
        // Validate time_format
        if !["12h", "24h"].contains(&self.time_format.as_str()) {
            return Err(format!("Invalid time_format: {}", self.time_format));
        }
        
        // Validate current_view
        if !["Day", "WorkWeek", "Week", "Month", "Quarter"].contains(&self.current_view.as_str()) {
            return Err(format!("Invalid current_view: {}", self.current_view));
        }
        
        Ok(())
    }
    
    /// Validate settings values without checking theme (theme validated by app.rs)
    pub fn validate_without_theme(&self) -> Result<(), String> {
        // Validate first_day_of_week (0-6, Sunday to Saturday)
        if self.first_day_of_week > 6 {
            return Err(format!("Invalid first_day_of_week: {}", self.first_day_of_week));
        }
        
        // Validate time_format
        if !["12h", "24h"].contains(&self.time_format.as_str()) {
            return Err(format!("Invalid time_format: {}", self.time_format));
        }
        
        // Validate current_view
        if !["Day", "WorkWeek", "Week", "Month", "Quarter"].contains(&self.current_view.as_str()) {
            return Err(format!("Invalid current_view: {}", self.current_view));
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
        assert_eq!(settings.date_format, "MM/DD/YYYY");
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
            assert!(settings.validate().is_ok(), "Theme '{}' should be valid", theme);
        }
    }
    
    #[test]
    fn test_validate_all_valid_views() {
        for view in ["Day", "WorkWeek", "Week", "Month", "Quarter"] {
            let mut settings = Settings::default();
            settings.current_view = view.to_string();
            assert!(settings.validate().is_ok(), "View '{}' should be valid", view);
        }
    }
}
