use crate::ui_egui::theme::{CalendarTheme, EventColors};
use egui::Color32;
use rusqlite::Row;

pub fn row_to_theme(row: &Row) -> Result<CalendarTheme, rusqlite::Error> {
    let name: String = row.get(0)?;
    let is_dark: i32 = row.get(1)?;

    // Get base theme to use for fallback colors
    let base = if is_dark != 0 {
        CalendarTheme::dark()
    } else {
        CalendarTheme::light()
    };

    // Header colors may be NULL in older databases, so use Option and fallback to base theme
    let header_background: Option<String> = row.get(11).ok();
    let header_text: Option<String> = row.get(12).ok();

    Ok(CalendarTheme {
        name,
        is_dark: is_dark != 0,
        app_background: CalendarTheme::string_to_color(&row.get::<_, String>(2)?)
            .unwrap_or(Color32::BLACK),
        calendar_background: CalendarTheme::string_to_color(&row.get::<_, String>(3)?)
            .unwrap_or(Color32::WHITE),
        weekend_background: CalendarTheme::string_to_color(&row.get::<_, String>(4)?)
            .unwrap_or(Color32::LIGHT_GRAY),
        today_background: CalendarTheme::string_to_color(&row.get::<_, String>(5)?)
            .unwrap_or(Color32::LIGHT_BLUE),
        today_border: CalendarTheme::string_to_color(&row.get::<_, String>(6)?)
            .unwrap_or(Color32::BLUE),
        day_background: CalendarTheme::string_to_color(&row.get::<_, String>(7)?)
            .unwrap_or(Color32::WHITE),
        day_border: CalendarTheme::string_to_color(&row.get::<_, String>(8)?)
            .unwrap_or(Color32::GRAY),
        text_primary: CalendarTheme::string_to_color(&row.get::<_, String>(9)?)
            .unwrap_or(Color32::BLACK),
        text_secondary: CalendarTheme::string_to_color(&row.get::<_, String>(10)?)
            .unwrap_or(Color32::GRAY),
        header_background: header_background
            .and_then(|s| CalendarTheme::string_to_color(&s).ok())
            .unwrap_or(base.header_background),
        header_text: header_text
            .and_then(|s| CalendarTheme::string_to_color(&s).ok())
            .unwrap_or(base.header_text),
        event_colors: EventColors::default(), // TODO: Load from DB when schema supports it
    })
}
