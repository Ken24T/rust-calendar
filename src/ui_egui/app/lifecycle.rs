use super::context::AppContext;
use super::countdown::CountdownUiState;
use super::state::{AppState, ViewType};
use super::toast::ToastManager;
use super::confirm::ConfirmDialogState;
use super::CalendarApp;
use crate::models::settings::Settings;
use crate::services::backup::BackupService;
use crate::services::countdown::CountdownService;
use crate::services::database::Database;
use crate::services::notification::NotificationService;
use crate::services::settings::SettingsService;
use crate::ui_egui::dialogs::backup_manager::BackupManagerState;
use crate::ui_egui::theme::CalendarTheme;
use crate::ui_egui::views::CountdownRequest;
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

        let mut countdown_service = load_countdown_service(&countdown_storage_path, database);
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
            toast_manager: ToastManager::new(),
            confirm_dialog: ConfirmDialogState::new(),
            active_category_filter: None,
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

        // If use_system_theme is enabled, detect and use system preference
        let theme_name = if self.settings.use_system_theme {
            match dark_light::detect() {
                dark_light::Mode::Dark => "Dark".to_string(),
                dark_light::Mode::Light => "Light".to_string(),
                dark_light::Mode::Default => self.settings.theme.clone(),
            }
        } else {
            self.settings.theme.clone()
        };

        if let Ok(theme) = theme_service.get_theme(&theme_name) {
            theme.apply_to_context(ctx);
            self.active_theme = theme;
        } else {
            log::warn!("Theme '{}' not found, using fallback.", theme_name);
            let fallback = Self::fallback_theme_for_settings(&self.settings);
            fallback.apply_to_context(ctx);
            self.active_theme = fallback;
        }
    }

    pub(super) fn handle_update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_file_drops(ctx);

        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ctx);

        // Sanitize geometries on first frame when we have monitor info
        if !self.state.geometry_sanitized {
            self.sanitize_countdown_geometries(ctx);
            self.state.geometry_sanitized = true;
        }

        // Apply pending theme change (from menu selection)
        if self.state.pending_theme_apply {
            self.apply_theme_from_db(ctx);
            self.state.pending_theme_apply = false;
        }

        self.apply_pending_root_geometry(ctx);

        self.render_menu_bar(ctx);

        // Render status bar (before sidebar and CentralPanel so it takes bottom space)
        self.render_status_bar(ctx);

        // Render sidebar (before CentralPanel so it takes left space)
        self.render_sidebar(ctx);

        let mut countdown_requests: Vec<CountdownRequest> = Vec::new();
        self.render_main_panel(ctx, &mut countdown_requests);

        if !countdown_requests.is_empty() {
            self.consume_countdown_requests(countdown_requests);
        }

        let render_result = self.countdown_ui.render_cards(
            ctx,
            self.context.countdown_service_mut(),
            self.settings.default_card_width,
            self.settings.default_card_height,
        );

        // Handle requests to open event dialog from countdown cards
        for request in render_result.event_dialog_requests {
            self.open_event_dialog_for_card(request);
        }

        // Handle requests to navigate to a date
        for request in render_result.go_to_date_requests {
            self.current_date = request.date;
        }
        
        // Handle delete confirmation requests for countdown cards
        for request in render_result.delete_card_requests {
            self.confirm_dialog.request(super::confirm::ConfirmAction::DeleteCountdownCard {
                card_id: request.card_id,
                card_title: request.card_title,
            });
        }

        self.countdown_ui
            .render_settings_dialogs(ctx, self.context.countdown_service_mut());
        
        // Handle delete requests from settings dialogs
        for request in self.countdown_ui.drain_delete_requests() {
            self.confirm_dialog.request(super::confirm::ConfirmAction::DeleteCountdownCard {
                card_id: request.card_id,
                card_title: request.card_title,
            });
        }
        
        self.flush_pending_event_bodies();
        self.handle_dialogs(ctx);

        // Render the date picker popup (if open)
        self.render_date_picker_popup(ctx);

        // Refresh countdown timers and check for notifications
        self.refresh_countdowns(ctx);
        self.check_and_show_countdown_notifications(ctx);

        self.persist_countdowns_if_needed();

        // Handle confirmation dialogs
        self.handle_confirm_dialog(ctx);

        // Render toast notifications (last, so they appear on top)
        let is_dark = self.active_theme.is_dark;
        self.toast_manager.render(ctx, is_dark);
    }

    pub(super) fn handle_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist_countdowns_if_needed();
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
            log::warn!("Failed to load settings: {}, using defaults", e);
            Settings::default()
        }
    }
}

fn load_countdown_service(path: &PathBuf, database: &Database) -> CountdownService {
    // First, try to migrate from JSON to database if JSON file exists
    if path.exists() {
        match CountdownService::migrate_json_to_database(path, database.connection()) {
            Ok(true) => {
                log::info!("Successfully migrated countdown cards from JSON to database");
            }
            Ok(false) => {
                // No migration needed (JSON file didn't exist)
            }
            Err(e) => {
                log::error!("Failed to migrate countdown cards from JSON: {e:?}");
                // Migration failed, but we should still try to load from database
                // The database may have cards from previous runs
            }
        }
    }

    // Load from database (primary source of truth)
    match CountdownService::load_from_database(database.connection()) {
        Ok(service) => service,
        Err(err) => {
            log::warn!("Failed to load countdown cards from database: {err:?}");
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
