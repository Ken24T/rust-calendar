use egui::{Color32, Ui};

#[derive(Clone, Copy)]
pub(crate) struct CalendarCellPalette {
    pub regular_bg: Color32,
    pub weekend_bg: Color32,
    pub today_bg: Color32,
    pub empty_bg: Color32,
    pub border: Color32,
    pub today_border: Color32,
    pub text: Color32,
    pub today_text: Color32,
    pub hover_border: Color32,
}

impl CalendarCellPalette {
    pub fn from_ui(ui: &Ui) -> Self {
        Self::from_dark_mode(ui.style().visuals.dark_mode)
    }

    fn from_dark_mode(dark_mode: bool) -> Self {
        if dark_mode {
            Self {
                regular_bg: Color32::from_gray(40),
                weekend_bg: Color32::from_gray(35),
                today_bg: Color32::from_rgb(60, 90, 150),
                empty_bg: Color32::from_gray(30),
                border: Color32::from_gray(60),
                today_border: Color32::from_rgb(100, 130, 200),
                text: Color32::LIGHT_GRAY,
                today_text: Color32::WHITE,
                hover_border: Color32::from_rgb(100, 150, 255),
            }
        } else {
            Self {
                regular_bg: Color32::from_rgb(246, 248, 252),
                weekend_bg: Color32::from_rgb(236, 240, 248),
                today_bg: Color32::from_rgb(221, 235, 255),
                empty_bg: Color32::from_rgb(236, 239, 245),
                border: Color32::from_rgb(205, 210, 220),
                today_border: Color32::from_rgb(118, 156, 224),
                text: Color32::from_rgb(55, 65, 85),
                today_text: Color32::from_rgb(30, 45, 90),
                hover_border: Color32::from_rgb(90, 140, 220),
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DayStripPalette {
    pub strip_bg: Color32,
    pub strip_border: Color32,
    pub accent_line: Color32,
    pub cell_bg: Color32,
    pub today_cell_bg: Color32,
    pub text: Color32,
    pub date_text: Color32,
    pub today_text: Color32,
    pub today_date_text: Color32,
    pub badge_bg: Color32,
    pub badge_text: Color32,
}

impl DayStripPalette {
    pub fn from_ui(ui: &Ui) -> Self {
        Self::from_dark_mode(ui.style().visuals.dark_mode)
    }

    fn from_dark_mode(dark_mode: bool) -> Self {
        if dark_mode {
            Self {
                strip_bg: Color32::from_rgb(30, 33, 41),
                strip_border: Color32::from_rgb(55, 60, 72),
                accent_line: Color32::from_rgb(100, 150, 255),
                cell_bg: Color32::from_rgb(40, 44, 54),
                today_cell_bg: Color32::from_rgb(60, 90, 150),
                text: Color32::from_rgb(215, 220, 232),
                date_text: Color32::from_rgb(140, 146, 160),
                today_text: Color32::from_rgb(240, 245, 255),
                today_date_text: Color32::from_rgb(200, 220, 255),
                badge_bg: Color32::from_rgb(100, 150, 255),
                badge_text: Color32::from_rgb(20, 24, 36),
            }
        } else {
            Self {
                strip_bg: Color32::from_rgb(245, 248, 255),
                strip_border: Color32::from_rgb(210, 215, 230),
                accent_line: Color32::from_rgb(130, 170, 240),
                cell_bg: Color32::from_rgb(255, 255, 255),
                today_cell_bg: Color32::from_rgb(227, 237, 255),
                text: Color32::from_rgb(55, 65, 90),
                date_text: Color32::from_rgb(115, 125, 150),
                today_text: Color32::from_rgb(40, 70, 120),
                today_date_text: Color32::from_rgb(70, 105, 165),
                badge_bg: Color32::from_rgb(120, 160, 230),
                badge_text: Color32::WHITE,
            }
        }
    }
}
