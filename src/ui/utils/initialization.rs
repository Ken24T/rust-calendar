use std::sync::{Arc, Mutex};
use chrono::Local;
use iced::Theme;

use crate::services::database::Database;
use crate::services::settings::SettingsService;
use crate::services::theme::ThemeService;
use crate::ui::theme::CalendarTheme;
use crate::ui::view_type::ViewType;

/// Initialization data returned from app setup
pub struct AppInitData {
    pub theme: Theme,
    pub calendar_theme: CalendarTheme,
    pub theme_name: String,
    pub available_themes: Vec<String>,
    pub show_my_day: bool,
    pub my_day_position_right: bool,
    pub show_ribbon: bool,
    pub current_view: ViewType,
    pub db: Arc<Mutex<Database>>,
    pub time_format: String,
    pub first_day_of_week: u8,
    pub date_format: String,
    pub current_date: chrono::NaiveDate,
    pub time_slot_interval: u32,
}

/// Initialize the application by loading database, settings, and themes
pub fn initialize_app(db_path: &str) -> AppInitData {
    // Initialize database
    let db = match Database::new(db_path) {
        Ok(db) => {
            if let Err(e) = db.initialize_schema() {
                eprintln!("Warning: Failed to initialize database schema: {}", e);
            }
            db
        }
        Err(e) => {
            eprintln!("Warning: Failed to open database, using defaults: {}", e);
            // Create in-memory database as fallback
            Database::new(":memory:").expect("Failed to create fallback in-memory database")
        }
    };

    // Load settings from database
    let settings_service = SettingsService::new(&db);
    let settings = settings_service.get().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load settings, using defaults: {}", e);
        crate::models::settings::Settings::default()
    });

    // Load available themes
    let theme_service = ThemeService::new(&db);
    let available_themes = theme_service.list_themes().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load themes: {}", e);
        vec!["Light".to_string(), "Dark".to_string()]
    });
    
    // Load the selected theme from database or use default
    let theme_name = settings.theme.clone();
    let calendar_theme = theme_service.get_theme(&theme_name)
        .unwrap_or_else(|_| CalendarTheme::light());
    
    let theme = calendar_theme.base.clone();

    // Parse current view
    let current_view = match settings.current_view.as_str() {
        "Day" => ViewType::Day,
        "WorkWeek" => ViewType::WorkWeek,
        "Week" => ViewType::Week,
        "Quarter" => ViewType::Quarter,
        _ => ViewType::Month,
    };

    AppInitData {
        theme,
        calendar_theme,
        theme_name,
        available_themes,
        show_my_day: settings.show_my_day,
        my_day_position_right: settings.my_day_position_right,
        show_ribbon: settings.show_ribbon,
        current_view,
        db: Arc::new(Mutex::new(db)),
        time_format: settings.time_format.clone(),
        first_day_of_week: settings.first_day_of_week,
        date_format: settings.date_format.clone(),
        current_date: Local::now().naive_local().date(),
        time_slot_interval: settings.time_slot_interval,
    }
}
