//! Theme module for egui calendar application
//!
//! Defines the CalendarTheme structure and provides conversion functions
//! between egui::Color32 and the database color format.

use egui::Color32;
use serde::{Deserialize, Serialize};

/// A calendar theme defining all colors used in the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarTheme {
    /// Whether this is a dark theme (affects base egui::Visuals)
    pub is_dark: bool,

    /// Application background color
    pub app_background: Color32,

    /// Calendar grid background color
    pub calendar_background: Color32,

    /// Weekend day background color
    pub weekend_background: Color32,

    /// Today's date background color
    pub today_background: Color32,

    /// Today's date border color
    pub today_border: Color32,

    /// Regular day background color
    pub day_background: Color32,

    /// Day cell border color
    pub day_border: Color32,

    /// Primary text color (headings, dates)
    pub text_primary: Color32,

    /// Secondary text color (secondary info)
    pub text_secondary: Color32,
}

impl CalendarTheme {
    /// Create the default Light theme
    pub fn light() -> Self {
        Self {
            is_dark: false,
            app_background: Color32::from_rgb(245, 245, 245),
            calendar_background: Color32::from_rgb(255, 255, 255),
            weekend_background: Color32::from_rgb(250, 250, 252),
            today_background: Color32::from_rgb(230, 240, 255),
            today_border: Color32::from_rgb(100, 150, 255),
            day_background: Color32::from_rgb(255, 255, 255),
            day_border: Color32::from_rgb(220, 220, 220),
            text_primary: Color32::from_rgb(40, 40, 40),
            text_secondary: Color32::from_rgb(100, 100, 100),
        }
    }

    /// Create the default Dark theme
    pub fn dark() -> Self {
        Self {
            is_dark: true,
            app_background: Color32::from_rgb(30, 30, 30),
            calendar_background: Color32::from_rgb(40, 40, 40),
            weekend_background: Color32::from_rgb(35, 35, 38),
            today_background: Color32::from_rgb(50, 60, 80),
            today_border: Color32::from_rgb(100, 150, 255),
            day_background: Color32::from_rgb(40, 40, 40),
            day_border: Color32::from_rgb(60, 60, 60),
            text_primary: Color32::from_rgb(240, 240, 240),
            text_secondary: Color32::from_rgb(170, 170, 170),
        }
    }

    /// Convert a Color32 to a database-compatible string format "r,g,b"
    pub fn color_to_string(color: Color32) -> String {
        format!("{},{},{}", color.r(), color.g(), color.b())
    }

    /// Parse a database color string "r,g,b" to Color32
    pub fn string_to_color(s: &str) -> Result<Color32, String> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid color format: {}", s));
        }

        let r = parts[0]
            .trim()
            .parse::<u8>()
            .map_err(|e| format!("Invalid red value: {}", e))?;
        let g = parts[1]
            .trim()
            .parse::<u8>()
            .map_err(|e| format!("Invalid green value: {}", e))?;
        let b = parts[2]
            .trim()
            .parse::<u8>()
            .map_err(|e| format!("Invalid blue value: {}", e))?;

        Ok(Color32::from_rgb(r, g, b))
    }

    /// Apply this theme to an egui context
    pub fn apply_to_context(&self, ctx: &egui::Context) {
        let mut visuals = if self.is_dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        // Customize visuals based on our theme
        visuals.window_fill = self.app_background;
        visuals.panel_fill = self.app_background;

        // Override widget colors to match our theme
        visuals.widgets.noninteractive.bg_fill = self.day_background;
        visuals.widgets.inactive.bg_fill = self.day_background;
        visuals.widgets.hovered.bg_fill = self.today_background;
        visuals.widgets.active.bg_fill = self.today_background;

        // Set text colors
        visuals.override_text_color = Some(self.text_primary);

        ctx.set_visuals(visuals);
    }

    /// Convert Color32 to hex string for display
    pub fn color_to_hex(color: Color32) -> String {
        format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
    }

    /// Parse hex string to Color32
    pub fn hex_to_color(hex: &str) -> Result<Color32, String> {
        let hex = hex.trim_start_matches('#');

        if hex.len() != 6 {
            return Err("Hex color must be 6 characters".to_string());
        }

        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex color")?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex color")?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex color")?;

        Ok(Color32::from_rgb(r, g, b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_string() {
        let color = Color32::from_rgb(255, 128, 64);
        assert_eq!(CalendarTheme::color_to_string(color), "255,128,64");
    }

    #[test]
    fn test_string_to_color() {
        let result = CalendarTheme::string_to_color("255,128,64").unwrap();
        assert_eq!(result, Color32::from_rgb(255, 128, 64));
    }

    #[test]
    fn test_color_to_hex() {
        let color = Color32::from_rgb(255, 128, 64);
        assert_eq!(CalendarTheme::color_to_hex(color), "#FF8040");
    }

    #[test]
    fn test_hex_to_color() {
        let result = CalendarTheme::hex_to_color("#FF8040").unwrap();
        assert_eq!(result, Color32::from_rgb(255, 128, 64));

        let result2 = CalendarTheme::hex_to_color("FF8040").unwrap();
        assert_eq!(result2, Color32::from_rgb(255, 128, 64));
    }

    #[test]
    fn test_light_theme() {
        let theme = CalendarTheme::light();
        assert!(!theme.is_dark);
        assert_eq!(theme.app_background, Color32::from_rgb(245, 245, 245));
    }

    #[test]
    fn test_dark_theme() {
        let theme = CalendarTheme::dark();
        assert!(theme.is_dark);
        assert_eq!(theme.app_background, Color32::from_rgb(30, 30, 30));
    }
}
