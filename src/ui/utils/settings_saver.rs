use std::sync::{Arc, Mutex};
use crate::services::database::Database;
use crate::services::settings::SettingsService;
use crate::ui::view_type::ViewType;

/// Save current settings to database
pub fn save_settings(
    db: &Arc<Mutex<Database>>,
    theme_name: &str,
    show_my_day: bool,
    my_day_position_right: bool,
    show_ribbon: bool,
    current_view: ViewType,
    time_format: &str,
    first_day_of_week: u8,
    date_format: &str,
) {
    let db = match db.lock() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Error: Failed to acquire database lock: {}", e);
            return;
        }
    };

    let settings_service = SettingsService::new(&db);

    let view_str = match current_view {
        ViewType::Day => "Day",
        ViewType::WorkWeek => "WorkWeek",
        ViewType::Week => "Week",
        ViewType::Month => "Month",
        ViewType::Quarter => "Quarter",
    };

    // Load existing settings to preserve other fields
    let mut settings = match settings_service.get() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: Failed to load settings for update: {}", e);
            crate::models::settings::Settings::default()
        }
    };

    // Update with current UI state
    settings.theme = theme_name.to_string();
    settings.show_my_day = show_my_day;
    settings.my_day_position_right = my_day_position_right;
    settings.show_ribbon = show_ribbon;
    settings.current_view = view_str.to_string();
    settings.time_format = time_format.to_string();
    settings.first_day_of_week = first_day_of_week;
    settings.date_format = date_format.to_string();

    // Save to database
    if let Err(e) = settings_service.update(&settings) {
        eprintln!("Error: Failed to save settings: {}", e);
    }
}
