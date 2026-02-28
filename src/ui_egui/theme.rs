//! Theme module for egui calendar application
//!
//! Defines the CalendarTheme structure and provides conversion functions
//! between egui::Color32 and the database color format.

use egui::Color32;
use serde::{Deserialize, Serialize};

/// Event category colors for theming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventColors {
    pub default: Color32,
    pub work: Color32,
    pub personal: Color32,
    pub holiday: Color32,
    pub birthday: Color32,
}

impl Default for EventColors {
    fn default() -> Self {
        Self {
            default: Color32::from_rgb(100, 149, 237),  // Cornflower blue
            work: Color32::from_rgb(0, 122, 204),       // Blue
            personal: Color32::from_rgb(22, 130, 93),   // Green
            holiday: Color32::from_rgb(221, 177, 0),    // Gold
            birthday: Color32::from_rgb(244, 135, 113), // Coral
        }
    }
}

impl EventColors {
    pub fn dark() -> Self {
        Self {
            default: Color32::from_rgb(139, 111, 184),  // Purple
            work: Color32::from_rgb(0, 122, 204),       // Blue
            personal: Color32::from_rgb(22, 130, 93),   // Green
            holiday: Color32::from_rgb(221, 177, 0),    // Gold
            birthday: Color32::from_rgb(244, 135, 113), // Coral
        }
    }
}

/// A calendar theme defining all colors used in the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarTheme {
    /// Theme name (for display and identification)
    #[serde(default)]
    pub name: String,

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

    /// Header/strip background color (day name headers)
    #[serde(default = "default_header_background")]
    pub header_background: Color32,

    /// Header text color
    #[serde(default = "default_header_text")]
    pub header_text: Color32,

    /// Event category colors
    #[serde(default)]
    pub event_colors: EventColors,
}

fn default_header_background() -> Color32 {
    Color32::from_rgb(235, 235, 240)
}

fn default_header_text() -> Color32 {
    Color32::from_rgb(60, 60, 60)
}

/// Built-in theme presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    Light,
    Dark,
    SolarizedLight,
    SolarizedDark,
    Nord,
    Dracula,
    HighContrast,
    Sepia,
}

impl ThemePreset {
    /// Get all available presets
    pub fn all() -> &'static [ThemePreset] {
        &[
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::SolarizedLight,
            ThemePreset::SolarizedDark,
            ThemePreset::Nord,
            ThemePreset::Dracula,
            ThemePreset::HighContrast,
            ThemePreset::Sepia,
        ]
    }

    /// Get the display name for this preset
    pub fn name(&self) -> &'static str {
        match self {
            ThemePreset::Light => "Light",
            ThemePreset::Dark => "Dark",
            ThemePreset::SolarizedLight => "Solarized Light",
            ThemePreset::SolarizedDark => "Solarized Dark",
            ThemePreset::Nord => "Nord",
            ThemePreset::Dracula => "Dracula",
            ThemePreset::HighContrast => "High Contrast",
            ThemePreset::Sepia => "Sepia",
        }
    }

    /// Get an icon/emoji for this preset
    pub fn icon(&self) -> &'static str {
        match self {
            ThemePreset::Light => "☀",
            ThemePreset::Dark => "🌙",
            ThemePreset::SolarizedLight => "🌅",
            ThemePreset::SolarizedDark => "🌃",
            ThemePreset::Nord => "❄",
            ThemePreset::Dracula => "🧛",
            ThemePreset::HighContrast => "◐",
            ThemePreset::Sepia => "📜",
        }
    }

    /// Create a CalendarTheme from this preset
    pub fn as_theme(&self) -> CalendarTheme {
        match self {
            ThemePreset::Light => CalendarTheme::light(),
            ThemePreset::Dark => CalendarTheme::dark(),
            ThemePreset::SolarizedLight => CalendarTheme::solarized_light(),
            ThemePreset::SolarizedDark => CalendarTheme::solarized_dark(),
            ThemePreset::Nord => CalendarTheme::nord(),
            ThemePreset::Dracula => CalendarTheme::dracula(),
            ThemePreset::HighContrast => CalendarTheme::high_contrast(),
            ThemePreset::Sepia => CalendarTheme::sepia(),
        }
    }

    /// Try to match a theme name to a preset
    pub fn from_name(name: &str) -> Option<ThemePreset> {
        match name.to_lowercase().as_str() {
            "light" => Some(ThemePreset::Light),
            "dark" => Some(ThemePreset::Dark),
            "solarized light" => Some(ThemePreset::SolarizedLight),
            "solarized dark" => Some(ThemePreset::SolarizedDark),
            "nord" => Some(ThemePreset::Nord),
            "dracula" => Some(ThemePreset::Dracula),
            "high contrast" => Some(ThemePreset::HighContrast),
            "sepia" => Some(ThemePreset::Sepia),
            _ => None,
        }
    }
}

impl CalendarTheme {
    /// Get a theme by preset name
    pub fn from_preset_name(name: &str) -> Option<Self> {
        ThemePreset::from_name(name).map(|p| p.as_theme())
    }

    /// Check if this theme name is a built-in preset
    pub fn is_builtin(name: &str) -> bool {
        ThemePreset::from_name(name).is_some()
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
        visuals.extreme_bg_color = self.app_background; // Prevents black gaps between panels

        // Override widget colors to match our theme
        visuals.widgets.noninteractive.bg_fill = self.day_background;
        visuals.widgets.inactive.bg_fill = self.day_background;
        visuals.widgets.hovered.bg_fill = self.today_background;
        visuals.widgets.active.bg_fill = self.today_background;

        // Set text colors
        visuals.override_text_color = Some(self.text_primary);

        ctx.set_visuals(visuals);
        
        // Also adjust spacing/interaction to reduce gaps between panels
        let mut style = (*ctx.style()).clone();
        style.spacing.window_margin = egui::Margin::ZERO;
        // Reduce resize grab radius to eliminate gaps
        style.interaction.resize_grab_radius_side = 0.0;
        ctx.set_style(style);
    }

    /// Convert Color32 to hex string for display
    pub fn color_to_hex(color: Color32) -> String {
        format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
    }

    /// Parse hex string to Color32
    #[allow(dead_code)]
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

    /// Export theme to TOML format
    pub fn to_toml(&self) -> String {
        format!(
            r#"[theme]
name = "{}"
is_dark = {}

[colors]
app_background = "{}"
calendar_background = "{}"
weekend_background = "{}"
today_background = "{}"
today_border = "{}"
day_background = "{}"
day_border = "{}"
text_primary = "{}"
text_secondary = "{}"
header_background = "{}"
header_text = "{}"

[event_colors]
default = "{}"
work = "{}"
personal = "{}"
holiday = "{}"
birthday = "{}"
"#,
            self.name,
            self.is_dark,
            Self::color_to_hex(self.app_background),
            Self::color_to_hex(self.calendar_background),
            Self::color_to_hex(self.weekend_background),
            Self::color_to_hex(self.today_background),
            Self::color_to_hex(self.today_border),
            Self::color_to_hex(self.day_background),
            Self::color_to_hex(self.day_border),
            Self::color_to_hex(self.text_primary),
            Self::color_to_hex(self.text_secondary),
            Self::color_to_hex(self.header_background),
            Self::color_to_hex(self.header_text),
            Self::color_to_hex(self.event_colors.default),
            Self::color_to_hex(self.event_colors.work),
            Self::color_to_hex(self.event_colors.personal),
            Self::color_to_hex(self.event_colors.holiday),
            Self::color_to_hex(self.event_colors.birthday),
        )
    }

    /// Import theme from TOML format
    pub fn from_toml(toml_str: &str) -> Result<Self, String> {
        let value: toml::Value = toml::from_str(toml_str)
            .map_err(|e| format!("Failed to parse TOML: {}", e))?;

        let theme = value.get("theme").ok_or("Missing [theme] section")?;
        let colors = value.get("colors").ok_or("Missing [colors] section")?;

        let name = theme.get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing theme name")?
            .to_string();

        let is_dark = theme.get("is_dark")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let get_color = |section: &toml::Value, key: &str, default: Color32| -> Color32 {
            section.get(key)
                .and_then(|v| v.as_str())
                .and_then(|s| Self::hex_to_color(s).ok())
                .unwrap_or(default)
        };

        let base = if is_dark { Self::dark() } else { Self::light() };

        let event_colors = if let Some(ec) = value.get("event_colors") {
            EventColors {
                default: get_color(ec, "default", base.event_colors.default),
                work: get_color(ec, "work", base.event_colors.work),
                personal: get_color(ec, "personal", base.event_colors.personal),
                holiday: get_color(ec, "holiday", base.event_colors.holiday),
                birthday: get_color(ec, "birthday", base.event_colors.birthday),
            }
        } else {
            base.event_colors.clone()
        };

        Ok(Self {
            name,
            is_dark,
            app_background: get_color(colors, "app_background", base.app_background),
            calendar_background: get_color(colors, "calendar_background", base.calendar_background),
            weekend_background: get_color(colors, "weekend_background", base.weekend_background),
            today_background: get_color(colors, "today_background", base.today_background),
            today_border: get_color(colors, "today_border", base.today_border),
            day_background: get_color(colors, "day_background", base.day_background),
            day_border: get_color(colors, "day_border", base.day_border),
            text_primary: get_color(colors, "text_primary", base.text_primary),
            text_secondary: get_color(colors, "text_secondary", base.text_secondary),
            header_background: get_color(colors, "header_background", base.header_background),
            header_text: get_color(colors, "header_text", base.header_text),
            event_colors,
        })
    }

    /// Get representative colors for theme preview swatches
    pub fn preview_colors(&self) -> [Color32; 4] {
        [
            self.app_background,
            self.today_background,
            self.today_border,
            self.text_primary,
        ]
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
        assert_eq!(theme.name, "Light");
    }

    #[test]
    fn test_dark_theme() {
        let theme = CalendarTheme::dark();
        assert!(theme.is_dark);
        assert_eq!(theme.app_background, Color32::from_rgb(30, 30, 30));
        assert_eq!(theme.name, "Dark");
    }

    #[test]
    fn test_preset_themes() {
        for preset in ThemePreset::all() {
            let theme = preset.as_theme();
            assert!(!theme.name.is_empty());
            // Just verify they can be created without panicking
        }
    }

    #[test]
    fn test_preset_from_name() {
        assert_eq!(ThemePreset::from_name("light"), Some(ThemePreset::Light));
        assert_eq!(ThemePreset::from_name("Dark"), Some(ThemePreset::Dark));
        assert_eq!(ThemePreset::from_name("Nord"), Some(ThemePreset::Nord));
        assert_eq!(ThemePreset::from_name("unknown"), None);
    }

    #[test]
    fn test_toml_roundtrip() {
        let theme = CalendarTheme::nord();
        let toml = theme.to_toml();
        let parsed = CalendarTheme::from_toml(&toml).unwrap();
        
        assert_eq!(parsed.name, theme.name);
        assert_eq!(parsed.is_dark, theme.is_dark);
        assert_eq!(parsed.app_background, theme.app_background);
        assert_eq!(parsed.event_colors.work, theme.event_colors.work);
    }

    #[test]
    fn test_is_builtin() {
        assert!(CalendarTheme::is_builtin("Light"));
        assert!(CalendarTheme::is_builtin("dark"));
        assert!(CalendarTheme::is_builtin("Nord"));
        assert!(!CalendarTheme::is_builtin("MyCustomTheme"));
    }
}
