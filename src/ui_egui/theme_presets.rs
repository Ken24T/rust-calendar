//! Built-in theme preset constructors for CalendarTheme.

use egui::Color32;

use super::theme::{CalendarTheme, EventColors};

impl CalendarTheme {
    /// Create the default Light theme
    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
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
            header_background: Color32::from_rgb(230, 232, 238),
            header_text: Color32::from_rgb(50, 55, 70),
            event_colors: EventColors::default(),
        }
    }

    /// Create the default Dark theme
    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),
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
            header_background: Color32::from_rgb(50, 52, 58),
            header_text: Color32::from_rgb(220, 220, 225),
            event_colors: EventColors::dark(),
        }
    }

    /// Solarized Light theme
    pub fn solarized_light() -> Self {
        Self {
            name: "Solarized Light".to_string(),
            is_dark: false,
            app_background: Color32::from_rgb(253, 246, 227),    // Base3
            calendar_background: Color32::from_rgb(238, 232, 213), // Base2
            weekend_background: Color32::from_rgb(238, 232, 213),
            today_background: Color32::from_rgb(211, 230, 227),
            today_border: Color32::from_rgb(38, 139, 210),       // Blue
            day_background: Color32::from_rgb(253, 246, 227),
            day_border: Color32::from_rgb(147, 161, 161),        // Base1
            text_primary: Color32::from_rgb(101, 123, 131),      // Base00
            text_secondary: Color32::from_rgb(147, 161, 161),    // Base1
            header_background: Color32::from_rgb(238, 232, 213), // Base2
            header_text: Color32::from_rgb(88, 110, 117),        // Base01
            event_colors: EventColors {
                default: Color32::from_rgb(38, 139, 210),        // Blue
                work: Color32::from_rgb(42, 161, 152),           // Cyan
                personal: Color32::from_rgb(133, 153, 0),        // Green
                holiday: Color32::from_rgb(181, 137, 0),         // Yellow
                birthday: Color32::from_rgb(211, 54, 130),       // Magenta
            },
        }
    }

    /// Solarized Dark theme
    pub fn solarized_dark() -> Self {
        Self {
            name: "Solarized Dark".to_string(),
            is_dark: true,
            app_background: Color32::from_rgb(0, 43, 54),        // Base03
            calendar_background: Color32::from_rgb(7, 54, 66),   // Base02
            weekend_background: Color32::from_rgb(7, 54, 66),
            today_background: Color32::from_rgb(30, 70, 90),
            today_border: Color32::from_rgb(38, 139, 210),       // Blue
            day_background: Color32::from_rgb(0, 43, 54),
            day_border: Color32::from_rgb(88, 110, 117),         // Base01
            text_primary: Color32::from_rgb(147, 161, 161),      // Base1
            text_secondary: Color32::from_rgb(88, 110, 117),     // Base01
            header_background: Color32::from_rgb(7, 54, 66),     // Base02
            header_text: Color32::from_rgb(131, 148, 150),       // Base0
            event_colors: EventColors {
                default: Color32::from_rgb(38, 139, 210),        // Blue
                work: Color32::from_rgb(42, 161, 152),           // Cyan
                personal: Color32::from_rgb(133, 153, 0),        // Green
                holiday: Color32::from_rgb(181, 137, 0),         // Yellow
                birthday: Color32::from_rgb(211, 54, 130),       // Magenta
            },
        }
    }

    /// Nord theme (dark, arctic-inspired)
    pub fn nord() -> Self {
        Self {
            name: "Nord".to_string(),
            is_dark: true,
            app_background: Color32::from_rgb(46, 52, 64),       // Nord0
            calendar_background: Color32::from_rgb(59, 66, 82),  // Nord1
            weekend_background: Color32::from_rgb(67, 76, 94),   // Nord2
            today_background: Color32::from_rgb(76, 86, 106),    // Nord3
            today_border: Color32::from_rgb(136, 192, 208),      // Nord8
            day_background: Color32::from_rgb(59, 66, 82),
            day_border: Color32::from_rgb(76, 86, 106),          // Nord3
            text_primary: Color32::from_rgb(236, 239, 244),      // Nord6
            text_secondary: Color32::from_rgb(216, 222, 233),    // Nord4
            header_background: Color32::from_rgb(67, 76, 94),    // Nord2
            header_text: Color32::from_rgb(229, 233, 240),       // Nord5
            event_colors: EventColors {
                default: Color32::from_rgb(129, 161, 193),       // Nord9
                work: Color32::from_rgb(136, 192, 208),          // Nord8
                personal: Color32::from_rgb(163, 190, 140),      // Nord14
                holiday: Color32::from_rgb(235, 203, 139),       // Nord13
                birthday: Color32::from_rgb(180, 142, 173),      // Nord15
            },
        }
    }

    /// Dracula theme (dark, vibrant)
    pub fn dracula() -> Self {
        Self {
            name: "Dracula".to_string(),
            is_dark: true,
            app_background: Color32::from_rgb(40, 42, 54),       // Background
            calendar_background: Color32::from_rgb(68, 71, 90),  // Current line
            weekend_background: Color32::from_rgb(68, 71, 90),
            today_background: Color32::from_rgb(98, 114, 164),   // Comment
            today_border: Color32::from_rgb(139, 233, 253),      // Cyan
            day_background: Color32::from_rgb(40, 42, 54),
            day_border: Color32::from_rgb(98, 114, 164),
            text_primary: Color32::from_rgb(248, 248, 242),      // Foreground
            text_secondary: Color32::from_rgb(189, 147, 249),    // Purple
            header_background: Color32::from_rgb(68, 71, 90),    // Current line
            header_text: Color32::from_rgb(248, 248, 242),       // Foreground
            event_colors: EventColors {
                default: Color32::from_rgb(189, 147, 249),       // Purple
                work: Color32::from_rgb(139, 233, 253),          // Cyan
                personal: Color32::from_rgb(80, 250, 123),       // Green
                holiday: Color32::from_rgb(241, 250, 140),       // Yellow
                birthday: Color32::from_rgb(255, 121, 198),      // Pink
            },
        }
    }

    /// High Contrast theme (accessibility)
    pub fn high_contrast() -> Self {
        Self {
            name: "High Contrast".to_string(),
            is_dark: true,
            app_background: Color32::from_rgb(0, 0, 0),
            calendar_background: Color32::from_rgb(0, 0, 0),
            weekend_background: Color32::from_rgb(30, 30, 30),
            today_background: Color32::from_rgb(0, 60, 120),
            today_border: Color32::from_rgb(0, 200, 255),
            day_background: Color32::from_rgb(0, 0, 0),
            day_border: Color32::from_rgb(255, 255, 255),
            text_primary: Color32::from_rgb(255, 255, 255),
            text_secondary: Color32::from_rgb(255, 255, 0),
            header_background: Color32::from_rgb(40, 40, 40),
            header_text: Color32::from_rgb(255, 255, 255),
            event_colors: EventColors {
                default: Color32::from_rgb(0, 200, 255),
                work: Color32::from_rgb(0, 255, 0),
                personal: Color32::from_rgb(255, 255, 0),
                holiday: Color32::from_rgb(255, 165, 0),
                birthday: Color32::from_rgb(255, 0, 255),
            },
        }
    }

    /// Sepia theme (warm, easy on eyes)
    pub fn sepia() -> Self {
        Self {
            name: "Sepia".to_string(),
            is_dark: false,
            app_background: Color32::from_rgb(251, 241, 219),
            calendar_background: Color32::from_rgb(245, 235, 213),
            weekend_background: Color32::from_rgb(240, 225, 195),
            today_background: Color32::from_rgb(230, 210, 170),
            today_border: Color32::from_rgb(139, 90, 43),
            day_background: Color32::from_rgb(251, 241, 219),
            day_border: Color32::from_rgb(200, 180, 150),
            text_primary: Color32::from_rgb(90, 70, 50),
            text_secondary: Color32::from_rgb(139, 110, 80),
            header_background: Color32::from_rgb(235, 220, 190),
            header_text: Color32::from_rgb(80, 60, 40),
            event_colors: EventColors {
                default: Color32::from_rgb(139, 90, 43),
                work: Color32::from_rgb(70, 100, 130),
                personal: Color32::from_rgb(80, 120, 70),
                holiday: Color32::from_rgb(180, 130, 50),
                birthday: Color32::from_rgb(160, 80, 80),
            },
        }
    }
}
