// Custom theme system for Rust Calendar
// Provides configurable colors for all UI elements

use iced::{Color, Theme};

/// Custom color scheme for calendar application
#[derive(Debug, Clone)]
pub struct CalendarTheme {
    /// Base theme (Light or Dark)
    pub base: Theme,
    /// Background color for main application area
    pub app_background: Color,
    /// Background color for calendar grid
    pub calendar_background: Color,
    /// Background color for weekends
    pub weekend_background: Color,
    /// Background color for today's date
    pub today_background: Color,
    /// Border color for today's date
    pub today_border: Color,
    /// Background color for regular days
    pub day_background: Color,
    /// Border color for day cells
    pub day_border: Color,
    /// Text color for primary text
    pub text_primary: Color,
    /// Text color for secondary text
    pub text_secondary: Color,
}

impl CalendarTheme {
    /// Create a light theme with custom colors
    pub fn light() -> Self {
        Self {
            base: Theme::Light,
            // Subtle off-white background for main app
            app_background: Color::from_rgb(0.98, 0.98, 0.98),
            // Pure white for calendar grid
            calendar_background: Color::from_rgb(1.0, 1.0, 1.0),
            // Very light gray for weekends
            weekend_background: Color::from_rgb(0.96, 0.96, 0.96),
            // Light blue for today
            today_background: Color::from_rgb(0.85, 0.92, 1.0),
            // Darker blue border for today
            today_border: Color::from_rgb(0.3, 0.5, 0.9),
            // White for regular days
            day_background: Color::from_rgb(1.0, 1.0, 1.0),
            // Light gray borders
            day_border: Color::from_rgb(0.85, 0.85, 0.85),
            // Dark text
            text_primary: Color::from_rgb(0.1, 0.1, 0.1),
            text_secondary: Color::from_rgb(0.4, 0.4, 0.4),
        }
    }

    /// Create a dark theme with custom colors
    pub fn dark() -> Self {
        Self {
            base: Theme::Dark,
            // Subtle dark gray background for main app
            app_background: Color::from_rgb(0.12, 0.12, 0.12),
            // Darker background for calendar grid
            calendar_background: Color::from_rgb(0.15, 0.15, 0.15),
            // Slightly lighter for weekends
            weekend_background: Color::from_rgb(0.18, 0.18, 0.18),
            // Dark blue for today
            today_background: Color::from_rgb(0.2, 0.3, 0.5),
            // Brighter blue border for today
            today_border: Color::from_rgb(0.4, 0.6, 1.0),
            // Dark gray for regular days
            day_background: Color::from_rgb(0.15, 0.15, 0.15),
            // Medium gray borders
            day_border: Color::from_rgb(0.3, 0.3, 0.3),
            // Light text
            text_primary: Color::from_rgb(0.95, 0.95, 0.95),
            text_secondary: Color::from_rgb(0.7, 0.7, 0.7),
        }
    }

    /// Create a custom theme (for future user customization)
    pub fn custom(is_dark: bool) -> Self {
        if is_dark {
            Self::dark()
        } else {
            Self::light()
        }
    }
}

impl Default for CalendarTheme {
    fn default() -> Self {
        Self::light()
    }
}
