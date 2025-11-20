use super::context::AppContext;
use super::countdown::CountdownUiState;
use super::state::{AppState, ViewType};
use super::CalendarApp;
use crate::models::settings::Settings;
use crate::services::backup::BackupService;
use crate::services::countdown::CountdownService;
use crate::services::database::Database;
use crate::services::notification::NotificationService;
use crate::services::settings::SettingsService;
use crate::ui_egui::dialogs::backup_manager::BackupManagerState;
use crate::ui_egui::theme::CalendarTheme;
use chrono::Local;
#[cfg(not(debug_assertions))]
use directories::ProjectDirs;
use std::path::PathBuf;

impl CalendarApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize database and leak it for 'static lifetime required by eframe
        let database = initialize_database();

        // Create temporary settings service to load persisted settings
        let settings_service = SettingsService::new(database);
        let settings = load_settings_or_default(&settings_service);
        log::info!(
            "Loaded settings: default_card_width={}, default_card_height={}",
            settings.default_card_width,
            settings.default_card_height
        );

        let current_view = Self::parse_view_type(&settings.current_view);
        let countdown_storage_path = Self::resolve_countdown_storage_path();
        cc.egui_ctx.set_embed_viewports(false);

        let mut countdown_service = load_countdown_service(&countdown_storage_path);
        Self::hydrate_countdown_titles_from_events(&mut countdown_service, database);

        let pending_root_geometry = countdown_service.app_window_geometry();
        let countdown_ui = CountdownUiState::new(&countdown_service);
        let notification_service = NotificationService::new();

        let context = AppContext::new(
            database,
            countdown_service,
            countdown_storage_path,
            notification_service,
        );

        let backup_manager_state = BackupManagerState::new(resolve_backup_db_path());
        let show_ribbon = settings.show_ribbon;

        let mut app = Self {
            context,
            settings,
            current_view,
            current_date: Local::now().date_naive(),
            show_event_dialog: false,
            show_settings_dialog: false,
            show_ribbon,
            active_theme: CalendarTheme::light(),
            event_dialog_state: None,
            event_dialog_date: None,
            event_dialog_time: None,
            event_dialog_recurrence: None,
            event_to_edit: None,
            pending_focus: None,
            countdown_ui,
            state: AppState::new(backup_manager_state, pending_root_geometry),
        };

        app.apply_theme_from_db(&cc.egui_ctx);
        app.focus_on_current_time_if_visible();
        app
    }

    fn parse_view_type(view_str: &str) -> ViewType {
        match view_str {
            "Day" => ViewType::Day,
            "Week" => ViewType::Week,
            "WorkWeek" => ViewType::WorkWeek,
            "Month" => ViewType::Month,
            "Quarter" => ViewType::Month,
            _ => ViewType::Month,
        }
    }

    pub(super) fn fallback_theme_for_settings(settings: &Settings) -> CalendarTheme {
        if settings.theme.to_lowercase().contains("dark") {
            CalendarTheme::dark()
        } else {
            CalendarTheme::light()
        }
    }

    pub(super) fn apply_theme_from_db(&mut self, ctx: &egui::Context) {
        let theme_service = self.context.theme_service();

        if let Ok(theme) = theme_service.get_theme(&self.settings.theme) {
            theme.apply_to_context(ctx);
            self.active_theme = theme;
        } else {
            log::warn!("Theme '{}' not found, using fallback.", self.settings.theme);
            let fallback = Self::fallback_theme_for_settings(&self.settings);
            fallback.apply_to_context(ctx);
            self.active_theme = fallback;
        }
    }
}

fn initialize_database() -> &'static Database {
    #[cfg(debug_assertions)]
    let db_path = "calendar.db".to_string();

    #[cfg(not(debug_assertions))]
    let db_path = {
        if let Some(proj_dirs) = ProjectDirs::from("com", "KenBoyle", "RustCalendar") {
            let data_dir = proj_dirs.data_dir();
            std::fs::create_dir_all(data_dir).expect("Failed to create data directory");
            data_dir.join("calendar.db").to_string_lossy().to_string()
        } else {
            "calendar_prod.db".to_string()
        }
    };

    let db = Database::new(&db_path).expect("Failed to create database connection");
    db.initialize_schema()
        .expect("Failed to initialize database schema");

    if let Err(e) = BackupService::auto_backup_on_startup(std::path::Path::new(&db_path), Some(5)) {
        log::warn!("Failed to create automatic backup on startup: {}", e);
    } else {
        log::info!("Automatic backup created successfully");
    }

    Box::leak(Box::new(db))
}

fn load_settings_or_default(settings_service: &SettingsService) -> Settings {
    match settings_service.get() {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("Failed to load settings: {}, using defaults", e);
            Settings::default()
        }
    }
}

fn load_countdown_service(path: &PathBuf) -> CountdownService {
    match CountdownService::load_from_disk(path) {
        Ok(service) => service,
        Err(err) => {
            log::warn!(
                "Failed to load countdown cards from {}: {err:?}",
                path.display()
            );
            CountdownService::new()
        }
    }
}

fn resolve_backup_db_path() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        PathBuf::from("calendar.db")
    }

    #[cfg(not(debug_assertions))]
    {
        if let Some(proj_dirs) = ProjectDirs::from("com", "KenBoyle", "RustCalendar") {
            let data_dir = proj_dirs.data_dir();
            data_dir.join("calendar.db")
        } else {
            PathBuf::from("calendar_prod.db")
        }
    }
}
