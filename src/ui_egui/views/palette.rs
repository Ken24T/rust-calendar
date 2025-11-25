use crate::ui_egui::theme::CalendarTheme;
use egui::Color32;

fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |c1: u8, c2: u8| -> u8 { ((c1 as f32 * (1.0 - t)) + (c2 as f32 * t)).round() as u8 };
    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

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
    pub fn from_theme(theme: &CalendarTheme) -> Self {
        Self {
            regular_bg: theme.day_background,
            weekend_bg: theme.weekend_background,
            today_bg: theme.today_background,
            empty_bg: theme.calendar_background,
            border: theme.day_border,
            today_border: theme.today_border,
            text: theme.text_primary,
            today_text: theme.text_primary,
            hover_border: with_alpha(theme.today_border, if theme.is_dark { 160 } else { 120 }),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DayStripPalette {
    #[allow(dead_code)]
    pub strip_bg: Color32,
    pub strip_border: Color32,
    pub accent_line: Color32,
    #[allow(dead_code)]
    pub cell_bg: Color32,
    #[allow(dead_code)]
    pub weekend_cell_bg: Color32,
    pub today_cell_bg: Color32,
    #[allow(dead_code)]
    pub text: Color32,
    #[allow(dead_code)]
    pub date_text: Color32,
    pub today_text: Color32,
    pub today_date_text: Color32,
    pub badge_bg: Color32,
    pub badge_text: Color32,
    pub header_bg: Color32,
    pub header_text: Color32,
}

impl DayStripPalette {
    pub fn from_theme(theme: &CalendarTheme) -> Self {
        Self {
            strip_bg: blend(theme.app_background, theme.calendar_background, 0.5),
            strip_border: theme.day_border,
            accent_line: theme.today_border,
            cell_bg: theme.day_background,
            weekend_cell_bg: theme.weekend_background,
            today_cell_bg: theme.today_background,
            text: theme.text_primary,
            date_text: theme.text_secondary,
            today_text: theme.text_primary,
            today_date_text: theme.text_secondary,
            badge_bg: theme.today_border,
            badge_text: if theme.is_dark {
                Color32::from_rgb(20, 20, 20)
            } else {
                Color32::from_rgb(245, 245, 245)
            },
            header_bg: theme.header_background,
            header_text: theme.header_text,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TimeGridPalette {
    pub hour_bg: Color32,
    pub regular_bg: Color32,
    pub weekend_bg: Color32,
    pub today_bg: Color32,
    pub hour_line: Color32,
    pub slot_line: Color32,
    pub divider: Color32,
    pub hover_overlay: Color32,
}

impl TimeGridPalette {
    pub fn from_theme(theme: &CalendarTheme) -> Self {
        let divider = with_alpha(theme.day_border, 220);
        Self {
            hour_bg: blend(theme.calendar_background, theme.day_background, 0.4),
            regular_bg: theme.day_background,
            weekend_bg: theme.weekend_background,
            today_bg: theme.today_background,
            hour_line: theme.day_border,
            slot_line: with_alpha(theme.day_border, 170),
            divider,
            hover_overlay: with_alpha(theme.today_border, if theme.is_dark { 80 } else { 50 }),
        }
    }
}
