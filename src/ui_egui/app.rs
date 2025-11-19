mod countdown;

use self::countdown::CountdownUiState;
use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::backup::BackupService;
use crate::services::countdown::{CountdownCardGeometry, CountdownService, CountdownWarningState};
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::services::notification::NotificationService;
use crate::services::settings::SettingsService;
use crate::services::theme::ThemeService;
use crate::ui_egui::dialogs::backup_manager::{
    render_backup_manager_dialog, BackupManagerState,
};
use crate::ui_egui::dialogs::theme_creator::{
    render_theme_creator, ThemeCreatorAction, ThemeCreatorState,
};
use crate::ui_egui::dialogs::theme_dialog::{
    render_theme_dialog, ThemeDialogAction, ThemeDialogState,
};
use crate::ui_egui::event_dialog::{render_event_dialog, EventDialogResult, EventDialogState};
use crate::ui_egui::settings_dialog::render_settings_dialog;
use crate::ui_egui::theme::CalendarTheme;
use crate::ui_egui::views::day_view::DayView;
use crate::ui_egui::views::month_view::MonthView;
use crate::ui_egui::views::week_view::WeekView;
use crate::ui_egui::views::workweek_view::WorkWeekView;
use crate::ui_egui::views::CountdownRequest;
use chrono::{Datelike, Local, NaiveDate};
#[cfg(not(debug_assertions))]
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    Day,
    Week,
    WorkWeek,
    Month,
}

const MIN_ROOT_WIDTH: f32 = 320.0;
const MIN_ROOT_HEIGHT: f32 = 220.0;

pub struct CalendarApp {
    // Core database (leaked for 'static lifetime to satisfy service requirements)
    database: &'static Database,

    // Application state
    settings: Settings,
    current_view: ViewType,
    current_date: NaiveDate,

    // Dialog states
    show_event_dialog: bool,
    show_settings_dialog: bool,
    backup_manager_state: BackupManagerState,
    theme_dialog_state: ThemeDialogState,
    theme_creator_state: ThemeCreatorState,

    // Ribbon state (mirrors self.settings.show_ribbon)
    show_ribbon: bool,

    // Currently applied theme colors
    active_theme: CalendarTheme,

    // Event dialog state
    event_dialog_state: Option<EventDialogState>,
    event_dialog_date: Option<NaiveDate>,
    event_dialog_time: Option<chrono::NaiveTime>, // Time from clicked cell (None = use default)
    event_dialog_recurrence: Option<String>,
    event_to_edit: Option<i64>, // Event ID to edit

    // Countdown cards
    countdown_service: CountdownService,
    countdown_storage_path: PathBuf,
    countdown_ui: CountdownUiState,
    pending_root_geometry: Option<CountdownCardGeometry>,

    // Notifications
    notification_service: NotificationService,
}

impl CalendarApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize database and leak it for 'static lifetime
        // This is necessary because eframe requires 'static App implementations
        let database = {
            // Use separate database files for debug and release builds
            #[cfg(debug_assertions)]
            let db_path = "calendar.db".to_string();
            
            #[cfg(not(debug_assertions))]
            let db_path = {
                // For release builds, use AppData directory
                if let Some(proj_dirs) = ProjectDirs::from("com", "KenBoyle", "RustCalendar") {
                    let data_dir = proj_dirs.data_dir();
                    std::fs::create_dir_all(data_dir).expect("Failed to create data directory");
                    data_dir.join("calendar.db").to_string_lossy().to_string()
                } else {
                    // Fallback to current directory if AppData not available
                    "calendar_prod.db".to_string()
                }
            };
            
            let db = Database::new(&db_path).expect("Failed to create database connection");

            // Initialize schema (create tables and insert defaults)
            db.initialize_schema()
                .expect("Failed to initialize database schema");

            // Create auto-backup on startup
            if let Err(e) = BackupService::auto_backup_on_startup(std::path::Path::new(&db_path), Some(5)) {
                log::warn!("Failed to create automatic backup on startup: {}", e);
            } else {
                log::info!("Automatic backup created successfully");
            }

            Box::leak(Box::new(db))
        };

        // Create temporary services to load settings
        let settings_service = SettingsService::new(database);

        // Load settings or create defaults
        let settings = match settings_service.get() {
            Ok(settings) => settings,
            Err(e) => {
                eprintln!("Failed to load settings: {}, using defaults", e);
                // No settings found, create and save defaults
                let defaults = Settings::default();
                // Note: The database INSERT OR IGNORE should handle initial creation,
                // but if somehow the row doesn't exist, this won't work.
                // The database initialization should ensure the row exists.
                defaults
            }
        };
        log::info!(
            "Loaded settings: default_card_width={}, default_card_height={}",
            settings.default_card_width,
            settings.default_card_height
        );

        // Parse current view from settings
        let current_view = Self::parse_view_type(&settings.current_view);

        let countdown_storage_path = Self::resolve_countdown_storage_path();
        cc.egui_ctx.set_embed_viewports(false);
        let mut countdown_service = match CountdownService::load_from_disk(&countdown_storage_path) {
            Ok(service) => service,
            Err(err) => {
                log::warn!(
                    "Failed to load countdown cards from {}: {err:?}",
                    countdown_storage_path.display()
                );
                CountdownService::new()
            }
        };

        Self::hydrate_countdown_titles_from_events(&mut countdown_service, database);

        let pending_root_geometry = countdown_service.app_window_geometry();
        let countdown_ui = CountdownUiState::new(&countdown_service);

        let show_ribbon = settings.show_ribbon;

        // Initialize notification service
        let notification_service = NotificationService::new();

        // Initialize backup manager state
        #[cfg(debug_assertions)]
        let db_path_for_backup = PathBuf::from("calendar.db");
        
        #[cfg(not(debug_assertions))]
        let db_path_for_backup = {
            if let Some(proj_dirs) = ProjectDirs::from("com", "KenBoyle", "RustCalendar") {
                let data_dir = proj_dirs.data_dir();
                data_dir.join("calendar.db")
            } else {
                PathBuf::from("calendar_prod.db")
            }
        };
        
        let backup_manager_state = BackupManagerState::new(db_path_for_backup);

        let mut app = Self {
            database,
            settings,
            current_view,
            current_date: Local::now().date_naive(),
            show_event_dialog: false,
            show_settings_dialog: false,
            backup_manager_state,
            theme_dialog_state: ThemeDialogState::new(),
            theme_creator_state: ThemeCreatorState::new(),
            show_ribbon,
            event_dialog_state: None,
            event_dialog_date: None,
            event_dialog_time: None,
            event_dialog_recurrence: None,
            event_to_edit: None,
            countdown_service,
            countdown_storage_path,
            countdown_ui,
            pending_root_geometry,
            notification_service,
            active_theme: CalendarTheme::light(),
        };

        // Apply theme from database (including custom themes)
        app.apply_theme_from_db(&cc.egui_ctx);

        app
    }

    fn parse_view_type(view_str: &str) -> ViewType {
        match view_str {
            "Day" => ViewType::Day,
            "Week" => ViewType::Week,
            "WorkWeek" => ViewType::WorkWeek,
            "Month" => ViewType::Month,
            "Quarter" => ViewType::Month,
            _ => ViewType::Month, // Default fallback
        }
    }

    fn fallback_theme_for_settings(settings: &Settings) -> CalendarTheme {
        if settings.theme.to_lowercase().contains("dark") {
            CalendarTheme::dark()
        } else {
            CalendarTheme::light()
        }
    }

    fn apply_theme_from_db(&mut self, ctx: &egui::Context) {
        let theme_service = ThemeService::new(self.database);

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

impl eframe::App for CalendarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle drag-and-drop of ICS files from file system
        ctx.input(|i| {
            if i.raw.dropped_files.is_empty() {
                return;
            }

            for file in &i.raw.dropped_files {
                let Some(path) = &file.path else {
                    continue;
                };

                match std::fs::read_to_string(path) {
                    Ok(ics_content) => {
                        if !(ics_content.contains("BEGIN:VCALENDAR")
                            || ics_content.contains("BEGIN:VEVENT"))
                        {
                            log::warn!(
                                "Dropped file {:?} does not look like an iCalendar file",
                                path
                            );
                            continue;
                        }

                        use crate::services::event::EventService;
                        use crate::services::icalendar::import;

                        match import::from_str(&ics_content) {
                            Ok(events) => {
                                if events.is_empty() {
                                    log::info!("No events found in dropped file {:?}", path);
                                    continue;
                                }

                                let service = EventService::new(self.database.connection());
                                let mut imported_count = 0;
                                let mut failed_count = 0;
                                let mut duplicate_count = 0;

                                if self.settings.edit_before_import {
                                    let first_event = &events[0];
                                    let existing_events = service.list_all().unwrap_or_default();
                                    let is_duplicate = existing_events.iter().any(|e| {
                                        e.title == first_event.title
                                            && e.start == first_event.start
                                            && e.end == first_event.end
                                    });

                                    if is_duplicate {
                                        log::info!(
                                            "Skipping duplicate event (edit mode): '{}'",
                                            first_event.title
                                        );
                                    } else {
                                        match service.create(first_event.clone()) {
                                            Ok(created_event) => {
                                                if let Some(event_id) = created_event.id {
                                                    self.event_to_edit = Some(event_id);
                                                    self.show_event_dialog = true;
                                                    log::info!(
                                                        "Opening event '{}' for editing",
                                                        first_event.title
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                log::error!(
                                                    "Failed to create event for editing: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }

                                    if events.len() > 1 {
                                        log::info!(
                                            "Note: Only the first event was opened for editing. {} other events were not imported.",
                                            events.len() - 1
                                        );
                                    }
                                } else {
                                    for event in events {
                                        let event_title = event.title.clone();
                                        let event_start = event.start;
                                        let event_end = event.end;

                                        let existing_events = service.list_all().unwrap_or_default();
                                        let is_duplicate = existing_events.iter().any(|e| {
                                            e.title == event_title
                                                && e.start == event_start
                                                && e.end == event_end
                                        });

                                        if is_duplicate {
                                            log::info!(
                                                "Skipping duplicate event: '{}'",
                                                event_title
                                            );
                                            duplicate_count += 1;
                                            continue;
                                        }

                                        match service.create(event) {
                                            Ok(created_event) => {
                                                imported_count += 1;

                                                if self.settings.auto_create_countdown_on_import
                                                    && event_start > Local::now()
                                                {
                                                    if let Some(event_id) = created_event.id {
                                                        use crate::services::countdown::RgbaColor;

                                                        let event_color = created_event
                                                            .color
                                                            .as_ref()
                                                            .and_then(|hex| {
                                                                if hex.starts_with('#')
                                                                    && hex.len() == 7
                                                                {
                                                                    u32::from_str_radix(
                                                                        &hex[1..],
                                                                        16,
                                                                    )
                                                                    .ok()
                                                                    .map(|rgb| {
                                                                        let r =
                                                                            ((rgb >> 16)
                                                                                & 0xFF)
                                                                                as u8;
                                                                        let g =
                                                                            ((rgb >> 8)
                                                                                & 0xFF)
                                                                                as u8;
                                                                        let b =
                                                                            (rgb & 0xFF)
                                                                                as u8;
                                                                        RgbaColor::new(
                                                                            r, g, b, 255,
                                                                        )
                                                                    })
                                                                } else {
                                                                    None
                                                                }
                                                            });

                                                        let location_label = created_event
                                                            .location
                                                            .as_deref()
                                                            .map(str::trim)
                                                            .filter(|loc| !loc.is_empty())
                                                            .map(|loc| loc.to_string());

                                                        let card_id = self.countdown_service.create_card(
                                                            Some(event_id),
                                                            created_event.title.clone(),
                                                            created_event.start,
                                                            event_color,
                                                            created_event.description.clone(),
                                                            self.settings.default_card_width,
                                                            self.settings.default_card_height,
                                                        );

                                                        if let Some(label) = location_label {
                                                            self.countdown_service
                                                                .set_auto_title_override(
                                                                    card_id,
                                                                    Some(label),
                                                                );
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                log::error!(
                                                    "Failed to import dropped event '{}': {}",
                                                    event_title,
                                                    e
                                                );
                                                failed_count += 1;
                                            }
                                        }
                                    }

                                    if duplicate_count > 0 {
                                        log::info!(
                                            "Import complete: {} events imported, {} duplicates skipped, {} failed",
                                            imported_count,
                                            duplicate_count,
                                            failed_count
                                        );
                                    } else {
                                        log::info!(
                                            "Import complete: {} events imported, {} failed",
                                            imported_count,
                                            failed_count
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to parse dropped ICS file {:?}: {}",
                                    path,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read dropped file {:?}: {}", path, e);
                    }
                }
            }
        });

        // Handle keyboard shortcuts
        ctx.input(|i| {
            // Escape - Close dialogs in priority order
            if i.key_pressed(egui::Key::Escape) {
                if self.show_event_dialog {
                    self.show_event_dialog = false;
                    self.event_dialog_state = None;
                    self.event_dialog_date = None;
                    self.event_dialog_time = None;
                    self.event_dialog_recurrence = None;
                    self.event_to_edit = None;
                } else if self.show_settings_dialog {
                    self.show_settings_dialog = false;
                } else if self.theme_dialog_state.is_open {
                    self.theme_dialog_state.close();
                }
            }
            
            // Ctrl+N - New Event
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) && !self.show_event_dialog {
                self.show_event_dialog = true;
                self.event_dialog_date = Some(self.current_date);
                self.event_dialog_time = None;
                self.event_dialog_recurrence = None;
                self.event_to_edit = None;
            }
            
            // Ctrl+T - Today (navigate to current date)
            if i.modifiers.ctrl && i.key_pressed(egui::Key::T) {
                self.current_date = Local::now().date_naive();
            }
            
            // Ctrl+S - Settings
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                self.show_settings_dialog = true;
            }
            
            // Ctrl+B - Backup Database
            if i.modifiers.ctrl && i.key_pressed(egui::Key::B) {
                if let Err(e) = self.backup_manager_state.create_backup() {
                    log::error!("Failed to create backup: {}", e);
                }
            }
            
            // Arrow keys for navigation (only when dialogs are closed)
            if !self.show_event_dialog && !self.show_settings_dialog && !self.theme_dialog_state.is_open {
                // Left/Right - Navigate backwards/forwards
                if i.key_pressed(egui::Key::ArrowLeft) {
                    self.current_date = match self.current_view {
                        ViewType::Day => self.current_date - chrono::Duration::days(1),
                        ViewType::Week | ViewType::WorkWeek => self.current_date - chrono::Duration::weeks(1),
                        ViewType::Month => {
                            // Navigate to previous month
                            let new_date = if self.current_date.month() == 1 {
                                NaiveDate::from_ymd_opt(self.current_date.year() - 1, 12, 1).unwrap()
                            } else {
                                NaiveDate::from_ymd_opt(self.current_date.year(), self.current_date.month() - 1, 1).unwrap()
                            };
                            // Try to keep the same day, fall back to last day of month if invalid
                            let max_day = chrono::NaiveDate::from_ymd_opt(new_date.year(), new_date.month() + 1, 1)
                                .unwrap_or(NaiveDate::from_ymd_opt(new_date.year() + 1, 1, 1).unwrap())
                                .pred_opt()
                                .unwrap()
                                .day();
                            let day = self.current_date.day().min(max_day);
                            NaiveDate::from_ymd_opt(new_date.year(), new_date.month(), day).unwrap()
                        }
                    };
                }
                if i.key_pressed(egui::Key::ArrowRight) {
                    self.current_date = match self.current_view {
                        ViewType::Day => self.current_date + chrono::Duration::days(1),
                        ViewType::Week | ViewType::WorkWeek => self.current_date + chrono::Duration::weeks(1),
                        ViewType::Month => {
                            // Navigate to next month
                            let new_date = if self.current_date.month() == 12 {
                                NaiveDate::from_ymd_opt(self.current_date.year() + 1, 1, 1).unwrap()
                            } else {
                                NaiveDate::from_ymd_opt(self.current_date.year(), self.current_date.month() + 1, 1).unwrap()
                            };
                            // Try to keep the same day, fall back to last day of month if invalid
                            let max_day = chrono::NaiveDate::from_ymd_opt(new_date.year(), new_date.month() + 1, 1)
                                .unwrap_or(NaiveDate::from_ymd_opt(new_date.year() + 1, 1, 1).unwrap())
                                .pred_opt()
                                .unwrap()
                                .day();
                            let day = self.current_date.day().min(max_day);
                            NaiveDate::from_ymd_opt(new_date.year(), new_date.month(), day).unwrap()
                        }
                    };
                }
                
                // Up/Down - Navigate up/down (weeks in week view, months in month view)
                if i.key_pressed(egui::Key::ArrowUp) {
                    self.current_date = match self.current_view {
                        ViewType::Day => self.current_date - chrono::Duration::days(7), // Move by week in day view
                        ViewType::Week | ViewType::WorkWeek => self.current_date - chrono::Duration::weeks(1),
                        ViewType::Month => {
                            // Navigate to previous month (same as left arrow)
                            let new_date = if self.current_date.month() == 1 {
                                NaiveDate::from_ymd_opt(self.current_date.year() - 1, 12, 1).unwrap()
                            } else {
                                NaiveDate::from_ymd_opt(self.current_date.year(), self.current_date.month() - 1, 1).unwrap()
                            };
                            let max_day = chrono::NaiveDate::from_ymd_opt(new_date.year(), new_date.month() + 1, 1)
                                .unwrap_or(NaiveDate::from_ymd_opt(new_date.year() + 1, 1, 1).unwrap())
                                .pred_opt()
                                .unwrap()
                                .day();
                            let day = self.current_date.day().min(max_day);
                            NaiveDate::from_ymd_opt(new_date.year(), new_date.month(), day).unwrap()
                        }
                    };
                }
                if i.key_pressed(egui::Key::ArrowDown) {
                    self.current_date = match self.current_view {
                        ViewType::Day => self.current_date + chrono::Duration::days(7), // Move by week in day view
                        ViewType::Week | ViewType::WorkWeek => self.current_date + chrono::Duration::weeks(1),
                        ViewType::Month => {
                            // Navigate to next month (same as right arrow)
                            let new_date = if self.current_date.month() == 12 {
                                NaiveDate::from_ymd_opt(self.current_date.year() + 1, 1, 1).unwrap()
                            } else {
                                NaiveDate::from_ymd_opt(self.current_date.year(), self.current_date.month() + 1, 1).unwrap()
                            };
                            let max_day = chrono::NaiveDate::from_ymd_opt(new_date.year(), new_date.month() + 1, 1)
                                .unwrap_or(NaiveDate::from_ymd_opt(new_date.year() + 1, 1, 1).unwrap())
                                .pred_opt()
                                .unwrap()
                                .day();
                            let day = self.current_date.day().min(max_day);
                            NaiveDate::from_ymd_opt(new_date.year(), new_date.month(), day).unwrap()
                        }
                    };
                }
            }
        });

        self.apply_pending_root_geometry(ctx);

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("💾 Backup Database...    Ctrl+B").clicked() {
                        if let Err(e) = self.backup_manager_state.create_backup() {
                            log::error!("Failed to create backup: {}", e);
                        }
                        ui.close_menu();
                    }
                    if ui.button("📂 Manage Backups...").clicked() {
                        self.backup_manager_state.open();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Settings    Ctrl+S").clicked() {
                        self.show_settings_dialog = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Import Event...").clicked() {
                        // Import ICS file
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("iCalendar", &["ics"])
                            .pick_file()
                        {
                            match std::fs::read_to_string(&path) {
                                Ok(ics_content) => {
                                    use crate::services::icalendar::import;
                                    use crate::services::event::EventService;
                                    
                                    match import::from_str(&ics_content) {
                                        Ok(events) => {
                                            let service = EventService::new(self.database.connection());
                                            let mut imported_count = 0;
                                            let mut failed_count = 0;
                                            let mut duplicate_count = 0;
                                            
                                            for event in events {
                                                let event_title = event.title.clone();
                                                let event_start = event.start;
                                                let event_end = event.end;
                                                
                                                // Check for duplicates (same title, start, and end time)
                                                let existing_events = service.list_all().unwrap_or_default();
                                                let is_duplicate = existing_events.iter().any(|e| {
                                                    e.title == event_title &&
                                                    e.start == event_start &&
                                                    e.end == event_end
                                                });
                                                
                                                if is_duplicate {
                                                    log::info!("Skipping duplicate event: '{}'", event_title);
                                                    duplicate_count += 1;
                                                    continue;
                                                }
                                                
                                                match service.create(event) {
                                                    Ok(created_event) => {
                                                        imported_count += 1;
                                                        
                                                        // Auto-create countdown if enabled and event is in the future
                                                        if self.settings.auto_create_countdown_on_import && event_start > Local::now() {
                                                            if let Some(event_id) = created_event.id {
                                                                // Parse color string to RgbaColor if present
                                                                use crate::services::countdown::RgbaColor;
                                                                let event_color = created_event.color.as_ref()
                                                                    .and_then(|hex| {
                                                                        if hex.starts_with('#') && hex.len() == 7 {
                                                                            u32::from_str_radix(&hex[1..], 16)
                                                                                .ok()
                                                                                .map(|rgb| {
                                                                                    let r = ((rgb >> 16) & 0xFF) as u8;
                                                                                    let g = ((rgb >> 8) & 0xFF) as u8;
                                                                                    let b = (rgb & 0xFF) as u8;
                                                                                    RgbaColor::new(r, g, b, 255)
                                                                                })
                                                                        } else {
                                                                            None
                                                                        }
                                                                    });
                                                                
                                                                let location_label = created_event
                                                                    .location
                                                                    .as_deref()
                                                                    .map(str::trim)
                                                                    .filter(|loc| !loc.is_empty())
                                                                    .map(|loc| loc.to_string());

                                                                let card_id = self.countdown_service.create_card(
                                                                    Some(event_id),
                                                                    created_event.title.clone(),
                                                                    created_event.start,
                                                                    event_color,
                                                                    created_event.description.clone(),
                                                                    self.settings.default_card_width,
                                                                    self.settings.default_card_height,
                                                                );

                                                                if let Some(label) = location_label {
                                                                    self.countdown_service
                                                                        .set_auto_title_override(card_id, Some(label));
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        log::error!("Failed to import event '{}': {}", event_title, e);
                                                        failed_count += 1;
                                                    }
                                                }
                                            }
                                            
                                            if duplicate_count > 0 {
                                                log::info!("Import complete: {} events imported, {} duplicates skipped, {} failed", imported_count, duplicate_count, failed_count);
                                            } else {
                                                log::info!("Import complete: {} events imported, {} failed", imported_count, failed_count);
                                            }
                                        }
                                        Err(e) => {
                                            log::error!("Failed to parse ICS file: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("Failed to read ICS file: {}", e);
                                }
                            }
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.checkbox(&mut self.show_ribbon, "Show All-Day Events Ribbon").clicked() {
                        self.settings.show_ribbon = self.show_ribbon;
                        let settings_service = SettingsService::new(self.database);
                        if let Err(err) = settings_service.update(&self.settings) {
                            log::error!("Failed to persist ribbon setting: {err}");
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .selectable_label(self.current_view == ViewType::Day, "Day")
                        .clicked()
                    {
                        self.current_view = ViewType::Day;
                        ui.close_menu();
                    }
                    if ui
                        .selectable_label(self.current_view == ViewType::Week, "Week")
                        .clicked()
                    {
                        self.current_view = ViewType::Week;
                        ui.close_menu();
                    }
                    if ui
                        .selectable_label(self.current_view == ViewType::WorkWeek, "Work Week")
                        .clicked()
                    {
                        self.current_view = ViewType::WorkWeek;
                        ui.close_menu();
                    }
                    if ui
                        .selectable_label(self.current_view == ViewType::Month, "Month")
                        .clicked()
                    {
                        self.current_view = ViewType::Month;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Theme", |ui| {
                    if ui.button("Themes...").clicked() {
                        self.theme_dialog_state.open();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Events", |ui| {
                    if ui.button("New Event...    Ctrl+N").clicked() {
                        self.show_event_dialog = true;
                        self.event_dialog_state = Some(EventDialogState::new_event(
                            self.current_date,
                            &self.settings,
                        ));
                        ui.close_menu();
                    }
                });
            });
        });

        let mut countdown_requests: Vec<CountdownRequest> = Vec::new();

        let active_countdown_events: std::collections::HashSet<i64> = self
            .countdown_service
            .cards()
            .iter()
            .filter_map(|card| card.event_id)
            .collect();

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!(
                "{} View - {}",
                match self.current_view {
                    ViewType::Day => "Day",
                    ViewType::Week => "Week",
                    ViewType::WorkWeek => "Work Week",
                    ViewType::Month => "Month",
                },
                self.current_date.format("%B %Y")
            ));

            ui.separator();

            // Navigation buttons
            ui.horizontal(|ui| {
                if ui.button("◀ Previous").clicked() {
                    self.navigate_previous();
                }
                if ui.button("Today    Ctrl+T").clicked() {
                    self.current_date = Local::now().date_naive();
                }
                if ui.button("Next ▶").clicked() {
                    self.navigate_next();
                }
                
                ui.separator();
                
                // Year/Month picker
                ui.label("Jump to:");
                
                // Month dropdown
                let month_names = [
                    "January", "February", "March", "April", "May", "June",
                    "July", "August", "September", "October", "November", "December"
                ];
                let current_month = self.current_date.month() as usize;
                egui::ComboBox::from_id_source("month_picker")
                    .selected_text(month_names[current_month - 1])
                    .show_ui(ui, |ui| {
                        for (idx, month_name) in month_names.iter().enumerate() {
                            if ui.selectable_value(&mut (idx + 1), current_month, *month_name).clicked() {
                                let new_month = (idx + 1) as u32;
                                if let Some(new_date) = chrono::NaiveDate::from_ymd_opt(
                                    self.current_date.year(),
                                    new_month,
                                    1
                                ) {
                                    self.current_date = new_date;
                                }
                            }
                        }
                    });
                
                // Year spinner
                let mut year = self.current_date.year();
                ui.add(egui::DragValue::new(&mut year).range(1900..=2100).speed(1.0));
                if year != self.current_date.year() {
                    if let Some(new_date) = chrono::NaiveDate::from_ymd_opt(
                        year,
                        self.current_date.month(),
                        1
                    ) {
                        self.current_date = new_date;
                    }
                }
            });

            ui.separator();

            // View content (ribbon is now rendered inside week view)
            match self.current_view {
                ViewType::Day => {
                    self.render_day_view(ui, &mut countdown_requests, &active_countdown_events)
                }
                ViewType::Week => {
                    self.render_week_view(ui, &mut countdown_requests, &active_countdown_events, self.show_ribbon)
                }
                ViewType::WorkWeek => {
                    self.render_workweek_view(ui, &mut countdown_requests, &active_countdown_events)
                }
                ViewType::Month => self.render_month_view(ui),
            }
        });

        if !countdown_requests.is_empty() {
            self.consume_countdown_requests(countdown_requests);
        }

        self.countdown_ui
            .render_cards(ctx, &mut self.countdown_service);
        self.countdown_ui
            .render_settings_dialogs(ctx, &mut self.countdown_service);
        self.flush_pending_event_bodies();

        // Dialogs (to be implemented)
        self.capture_root_geometry(ctx);
        if self.show_event_dialog {
            // Create dialog state if not already present
            if self.event_dialog_state.is_none() {
                // Check if we're editing an existing event
                if let Some(event_id) = self.event_to_edit {
                    // Load event from database
                    let service = EventService::new(self.database.connection());
                    if let Ok(Some(event)) = service.get(event_id) {
                        self.event_dialog_state =
                            Some(EventDialogState::from_event(&event, &self.settings));
                    } else {
                        // Event not found, create new one instead
                        self.event_dialog_state = Some(EventDialogState::new_event(
                            self.event_dialog_date.unwrap_or(self.current_date),
                            &self.settings,
                        ));
                    }
                } else {
                    // Creating a new event
                    self.event_dialog_state = Some(EventDialogState::new_event_with_time(
                        self.event_dialog_date.unwrap_or(self.current_date),
                        self.event_dialog_time,
                        &self.settings,
                    ));
                    // Apply any recurrence rule from click
                    if let (Some(ref mut state), Some(ref rrule)) =
                        (&mut self.event_dialog_state, &self.event_dialog_recurrence)
                    {
                        // Parse and set recurrence from the click handler
                        state.is_recurring = true;
                        if rrule.contains("WEEKLY") {
                            state.frequency =
                                crate::ui_egui::event_dialog::RecurrenceFrequency::Weekly;
                        } else if rrule.contains("MONTHLY") {
                            state.frequency =
                                crate::ui_egui::event_dialog::RecurrenceFrequency::Monthly;
                        } else if rrule.contains("YEARLY") {
                            state.frequency =
                                crate::ui_egui::event_dialog::RecurrenceFrequency::Yearly;
                        } else {
                            state.frequency =
                                crate::ui_egui::event_dialog::RecurrenceFrequency::Daily;
                        }
                    }
                }
            }

            self.render_event_dialog(ctx);
        }

        if self.show_settings_dialog {
            self.render_settings_dialog(ctx);
        }

        // Periodically refresh countdown cards even before their UI arrives.
        let changed_counts = self.countdown_service.refresh_days_remaining(Local::now());
        if !changed_counts.is_empty() {
            ctx.request_repaint();
        }

        // Check for notification triggers (warning state transitions)
        let now = Local::now();
        let notification_triggers = self
            .countdown_service
            .check_notification_triggers(now);
        
        if !notification_triggers.is_empty() {
            // Get notification config to check if system notifications are enabled
            let notification_config = self.countdown_service.notification_config();
            
            if notification_config.use_system_notifications {
                for (card_id, _old_state, new_state) in &notification_triggers {
                    // Find the card to get event details
                    if let Some(card) = self
                        .countdown_service
                        .cards()
                        .iter()
                        .find(|c| c.id == *card_id)
                    {
                        let (message, urgency) = Self::notification_message_for_state(*new_state, card.start_at, now);
                        
                        if let Err(e) = self.notification_service.show_countdown_alert(
                            card.effective_title(),
                            &message,
                            urgency,
                        ) {
                            log::warn!("Failed to show system notification: {}", e);
                        } else {
                            log::info!(
                                "Showed system notification for card {:?} ({}) - state: {:?}",
                                card_id,
                                card.effective_title(),
                                new_state
                            );
                        }
                    }
                }
            }
            
            // Log all transitions
            for (card_id, old_state, new_state) in notification_triggers {
                log::info!(
                    "Countdown notification trigger: card {:?} transitioned from {:?} to {:?}",
                    card_id,
                    old_state,
                    new_state
                );
            }
            
            ctx.request_repaint();
        }

        // Check for auto-dismiss
        let dismissed_cards = self.countdown_service.check_auto_dismiss(now);
        if !dismissed_cards.is_empty() {
            log::info!("Auto-dismissed {} countdown card(s)", dismissed_cards.len());
            ctx.request_repaint();
        }

        self.persist_countdowns_if_needed();

        // Render unified theme dialog and creator
        self.render_theme_dialog(ctx);
        self.render_theme_creator(ctx);
        
        // Render backup manager dialog
        let should_reload_db = render_backup_manager_dialog(ctx, &mut self.backup_manager_state);
        if should_reload_db {
            // Note: Database restoration requires app restart
            // Show message and close app
            log::info!("Database restored. Application should be restarted.");
            // TODO: Implement graceful restart or reload mechanism
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist_countdowns_if_needed();
    }
}

impl CalendarApp {
    /// Generate notification message and urgency based on countdown warning state
    fn notification_message_for_state(
        state: CountdownWarningState,
        event_time: chrono::DateTime<Local>,
        now: chrono::DateTime<Local>,
    ) -> (String, crate::services::notification::NotificationUrgency) {
        use crate::services::notification::NotificationUrgency;
        
        let remaining = event_time.signed_duration_since(now);
        
        match state {
            CountdownWarningState::Critical => {
                let minutes = remaining.num_minutes();
                let message = if minutes > 0 {
                    format!("Starting in {} minute{}", minutes, if minutes == 1 { "" } else { "s" })
                } else {
                    "Starting very soon!".to_string()
                };
                (message, NotificationUrgency::Critical)
            }
            CountdownWarningState::Imminent => {
                let hours = remaining.num_hours();
                let minutes = remaining.num_minutes() % 60;
                let message = if hours > 0 {
                    format!("Starting in {} hour{} {} minute{}", 
                        hours, if hours == 1 { "" } else { "s" },
                        minutes, if minutes == 1 { "" } else { "s" })
                } else {
                    format!("Starting in {} minute{}", minutes, if minutes == 1 { "" } else { "s" })
                };
                (message, NotificationUrgency::Critical)
            }
            CountdownWarningState::Approaching => {
                let hours = remaining.num_hours();
                let message = format!("Starting in {} hour{}", hours, if hours == 1 { "" } else { "s" });
                (message, NotificationUrgency::Normal)
            }
            CountdownWarningState::Starting => {
                ("Event is starting now!".to_string(), NotificationUrgency::Critical)
            }
            CountdownWarningState::Normal => {
                ("Event approaching".to_string(), NotificationUrgency::Normal)
            }
        }
    }

    fn navigate_previous(&mut self) {
        use chrono::Datelike;

        self.current_date = match self.current_view {
            ViewType::Day => self.current_date - chrono::Duration::days(1),
            ViewType::Week | ViewType::WorkWeek => self.current_date - chrono::Duration::weeks(1),
            ViewType::Month => {
                let prev_month = if self.current_date.month() == 1 {
                    12
                } else {
                    self.current_date.month() - 1
                };
                let year = if self.current_date.month() == 1 {
                    self.current_date.year() - 1
                } else {
                    self.current_date.year()
                };
                NaiveDate::from_ymd_opt(year, prev_month, 1).unwrap()
            }
        };
    }

    fn navigate_next(&mut self) {
        use chrono::Datelike;

        self.current_date = match self.current_view {
            ViewType::Day => self.current_date + chrono::Duration::days(1),
            ViewType::Week | ViewType::WorkWeek => self.current_date + chrono::Duration::weeks(1),
            ViewType::Month => {
                let next_month = if self.current_date.month() == 12 {
                    1
                } else {
                    self.current_date.month() + 1
                };
                let year = if self.current_date.month() == 12 {
                    self.current_date.year() + 1
                } else {
                    self.current_date.year()
                };
                NaiveDate::from_ymd_opt(year, next_month, 1).unwrap()
            }
        };
    }

    // View renderers
    fn render_day_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &std::collections::HashSet<i64>,
    ) {
        if let Some(clicked_event) = DayView::show(
            ui,
            &mut self.current_date,
            self.database,
            &self.settings,
            &self.active_theme,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_time,
            &mut self.event_dialog_recurrence,
            countdown_requests,
            active_countdown_events,
        ) {
            // User clicked on an event - open dialog with event details
            self.event_dialog_state =
                Some(EventDialogState::from_event(&clicked_event, &self.settings));
            self.show_event_dialog = true;
        }
    }

    fn render_week_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &std::collections::HashSet<i64>,
        show_ribbon: bool,
    ) {
        // Get all-day events for the ribbon
        let all_day_events = if show_ribbon {
            use chrono::TimeZone;
            let event_service = EventService::new(self.database.connection());
            
            // Calculate week range
            let weekday = self.current_date.weekday().num_days_from_sunday() as i64;
            let offset = (weekday - self.settings.first_day_of_week as i64 + 7) % 7;
            let week_start = self.current_date - chrono::Duration::days(offset);
            let week_end = week_start + chrono::Duration::days(6);
            
            let start_datetime = Local
                .from_local_datetime(&week_start.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .unwrap();
            let end_datetime = Local
                .from_local_datetime(&week_end.and_hms_opt(23, 59, 59).unwrap())
                .single()
                .unwrap();
            
            event_service
                .expand_recurring_events(start_datetime, end_datetime)
                .unwrap_or_default()
                .into_iter()
                .filter(|e| e.all_day)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        if let Some(clicked_event) = WeekView::show(
            ui,
            &mut self.current_date,
            self.database,
            &self.settings,
            &self.active_theme,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_time,
            &mut self.event_dialog_recurrence,
            countdown_requests,
            active_countdown_events,
            show_ribbon,
            &all_day_events,
        ) {
            // User clicked on an event - open dialog with event details
            self.event_dialog_state =
                Some(EventDialogState::from_event(&clicked_event, &self.settings));
            self.show_event_dialog = true;
        }
    }

    fn render_workweek_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &std::collections::HashSet<i64>,
    ) {
        if let Some(clicked_event) = WorkWeekView::show(
            ui,
            &mut self.current_date,
            self.database,
            &self.settings,
            &self.active_theme,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_time,
            &mut self.event_dialog_recurrence,
            countdown_requests,
            active_countdown_events,
        ) {
            // User clicked on an event - open dialog with event details
            self.event_dialog_state =
                Some(EventDialogState::from_event(&clicked_event, &self.settings));
            self.show_event_dialog = true;
        }
    }

    fn render_month_view(&mut self, ui: &mut egui::Ui) {
        MonthView::show(
            ui,
            &mut self.current_date,
            self.database,
            &self.settings,
            &self.active_theme,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_recurrence,
            &mut self.event_to_edit,
        );
    }

    

    // Placeholder dialog renderers
    fn render_event_dialog(&mut self, ctx: &egui::Context) {
        if let Some(ref mut state) = self.event_dialog_state {
            let EventDialogResult { saved_event } = render_event_dialog(
                ctx,
                state,
                self.database,
                &self.settings,
                &mut self.show_event_dialog,
            );

            let auto_create_card = state.create_countdown && state.event_id.is_none();
            let event_saved = saved_event.is_some();
            if let Some(event) = saved_event {
                if auto_create_card {
                    self.consume_countdown_requests(vec![CountdownRequest::from_event(&event)]);
                }
                self.sync_cards_from_event(&event);
            }

            // If saved, clear the dialog state
            if event_saved || !self.show_event_dialog {
                self.event_dialog_state = None;
                self.event_dialog_time = None;
            }
        } else {
            // No state - shouldn't happen, but close dialog if it does
            self.show_event_dialog = false;
        }
    }

    fn hydrate_countdown_titles_from_events(
        countdown_service: &mut CountdownService,
        database: &'static Database,
    ) {
        let mut seen_ids = std::collections::HashSet::new();
        let mut event_ids = Vec::new();

        for card in countdown_service.cards() {
            if let Some(event_id) = card.event_id {
                if seen_ids.insert(event_id) {
                    event_ids.push(event_id);
                }
            }
        }

        if event_ids.is_empty() {
            return;
        }

        let event_service = EventService::new(database.connection());
        for event_id in event_ids {
            match event_service.get(event_id) {
                Ok(Some(event)) => {
                    let location_label = event
                        .location
                        .as_deref()
                        .map(str::trim)
                        .filter(|loc| !loc.is_empty())
                        .map(|loc| loc.to_string());

                    countdown_service.sync_title_for_event(event_id, event.title.clone());
                    countdown_service
                        .sync_title_override_for_event(event_id, location_label);
                }
                Ok(None) => {
                    log::warn!(
                        "Countdown card references missing event id {} while syncing titles",
                        event_id
                    );
                }
                Err(err) => {
                    log::error!(
                        "Failed to load event {} while syncing countdown titles: {}",
                        event_id,
                        err
                    );
                }
            }
        }
    }

    fn flush_pending_event_bodies(&mut self) {
        let updates = self.countdown_ui.drain_pending_event_bodies();
        if updates.is_empty() {
            return;
        }

        let service = EventService::new(self.database.connection());
        for (event_id, body) in updates {
            match service.get(event_id) {
                Ok(Some(mut event)) => {
                    event.description = body.clone();
                    if let Err(err) = service.update(&event) {
                        log::error!(
                            "Failed to update event {} body from countdown settings: {err}",
                            event_id
                        );
                        continue;
                    }
                    self.countdown_service
                        .sync_comment_for_event(event_id, body.clone());
                }
                Ok(None) => {
                    log::warn!(
                        "Countdown requested update for missing event id {}",
                        event_id
                    );
                }
                Err(err) => {
                    log::error!(
                        "Failed to load event {} for countdown body sync: {err}",
                        event_id
                    );
                }
            }
        }
    }

    fn sync_cards_from_event(&mut self, event: &Event) {
        if let Some(event_id) = event.id {
            let location_label = event
                .location
                .as_deref()
                .map(str::trim)
                .filter(|loc| !loc.is_empty())
                .map(|loc| loc.to_string());

            self.countdown_service
                .sync_title_for_event(event_id, event.title.clone());
            self.countdown_service
                .sync_title_override_for_event(event_id, location_label);
            self.countdown_service
                .sync_comment_for_event(event_id, event.description.clone());
        }
    }

    fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        let response = render_settings_dialog(
            ctx,
            &mut self.settings,
            self.database,
            &mut self.show_settings_dialog,
        );

        if response.show_ribbon_changed || response.saved {
            self.show_ribbon = self.settings.show_ribbon;
        }

        // If settings were saved, apply theme
        if response.saved {
            self.apply_theme_from_db(ctx);
        }
    }

    fn render_theme_dialog(&mut self, ctx: &egui::Context) {
        // Get available themes from database
        let theme_service = ThemeService::new(self.database);
        let available_themes = theme_service.list_themes().unwrap_or_default();

        let action = render_theme_dialog(
            ctx,
            &mut self.theme_dialog_state,
            &available_themes,
            &self.settings.theme,
        );

        match action {
            ThemeDialogAction::None => {}
            ThemeDialogAction::CreateTheme => {
                // Open theme creator with current theme as base
                let base_theme = theme_service
                    .get_theme(&self.settings.theme)
                    .unwrap_or_else(|_| CalendarTheme::light());
                self.theme_creator_state.open_create(base_theme);
            }
            ThemeDialogAction::EditTheme(name) => {
                // Load and edit the theme
                if let Ok(theme) = theme_service.get_theme(&name) {
                    self.theme_creator_state.open_edit(name, theme);
                }
            }
            ThemeDialogAction::DeleteTheme(name) => {
                // Delete the theme
                if let Err(e) = theme_service.delete_theme(&name) {
                    eprintln!("Failed to delete theme: {}", e);
                } else {
                    eprintln!("Successfully deleted theme: {}", name);
                }
            }
            ThemeDialogAction::ApplyTheme(name) => {
                // Apply the selected theme
                self.settings.theme = name.clone();

                // Apply the custom theme or built-in theme
                if let Ok(theme) = theme_service.get_theme(&name) {
                    theme.apply_to_context(ctx);
                    self.active_theme = theme;
                } else {
                    let fallback = Self::fallback_theme_for_settings(&self.settings);
                    fallback.apply_to_context(ctx);
                    self.active_theme = fallback;
                }

                // Save to database
                let settings_service = SettingsService::new(self.database);
                if let Err(e) = settings_service.update(&self.settings) {
                    eprintln!("Failed to save theme setting: {}", e);
                }
            }
            ThemeDialogAction::Close => {
                self.theme_dialog_state.close();
            }
        }
    }

    fn render_theme_creator(&mut self, ctx: &egui::Context) {
        let action = render_theme_creator(ctx, &mut self.theme_creator_state);

        match action {
            ThemeCreatorAction::None => {}
            ThemeCreatorAction::Save(name, theme) => {
                // Save the theme to database
                let theme_service = ThemeService::new(self.database);
                if let Err(e) = theme_service.save_theme(&theme, &name) {
                    eprintln!("Failed to save theme: {}", e);
                    self.theme_creator_state.validation_error =
                        Some(format!("Failed to save: {}", e));
                    self.theme_creator_state.is_open = true; // Reopen to show error
                } else {
                    eprintln!("Successfully saved theme: {}", name);

                    // Apply the new theme
                    self.settings.theme = name.clone();
                    theme.apply_to_context(ctx);
                    self.active_theme = theme.clone();

                    // Save settings
                    let settings_service = SettingsService::new(self.database);
                    if let Err(e) = settings_service.update(&self.settings) {
                        eprintln!("Failed to save settings: {}", e);
                    }

                    self.theme_creator_state.close();
                }
            }
            ThemeCreatorAction::Cancel => {
                self.theme_creator_state.close();
            }
        }
    }

    fn apply_pending_root_geometry(&mut self, ctx: &egui::Context) {
        if let Some(geometry) = self.pending_root_geometry.take() {
            if !Self::is_plausible_root_geometry(&geometry) {
                log::warn!(
                    "Ignoring persisted root geometry due to implausible size: {:?}",
                    geometry
                );
                return;
            }
            log::debug!("Applying persisted root geometry: {:?}", geometry);
            if geometry.width > 40.0 && geometry.height > 40.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    geometry.x, geometry.y,
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    geometry.width,
                    geometry.height,
                )));
            }
        }
    }

    fn capture_root_geometry(&mut self, ctx: &egui::Context) {
        if let Some(info) = viewport_info(ctx, egui::ViewportId::ROOT) {
            if let Some(geometry) = geometry_from_viewport_info(&info) {
                if !Self::is_plausible_root_geometry(&geometry) {
                    log::debug!(
                        "Skipping root geometry capture due to implausible size: {:?}",
                        geometry
                    );
                    return;
                }
                let needs_update = match self.countdown_service.app_window_geometry() {
                    Some(current) => geometry_changed(current, geometry),
                    None => true,
                };
                if needs_update {
                    log::debug!("Captured new root geometry: {:?}", geometry);
                    self.countdown_service.update_app_window_geometry(geometry);
                }
            }
        }
    }

    fn is_plausible_root_geometry(geometry: &CountdownCardGeometry) -> bool {
        geometry.width >= MIN_ROOT_WIDTH && geometry.height >= MIN_ROOT_HEIGHT
    }
}

fn viewport_info(ctx: &egui::Context, viewport_id: egui::ViewportId) -> Option<egui::ViewportInfo> {
    ctx.input(|input| input.raw.viewports.get(&viewport_id).cloned())
}

fn geometry_from_viewport_info(info: &egui::ViewportInfo) -> Option<CountdownCardGeometry> {
    let inner = match info.inner_rect {
        Some(rect) => rect,
        None => return None,
    };
    let (outer_left, outer_top) = info
        .outer_rect
        .map(|outer| (outer.left(), outer.top()))
        .unwrap_or((inner.left(), inner.top()));

    Some(CountdownCardGeometry {
        x: outer_left,
        y: outer_top,
        width: inner.width(),
        height: inner.height(),
    })
}

fn geometry_changed(a: CountdownCardGeometry, b: CountdownCardGeometry) -> bool {
    (a.x - b.x).abs() > 2.0
        || (a.y - b.y).abs() > 2.0
        || (a.width - b.width).abs() > 1.0
        || (a.height - b.height).abs() > 1.0
}

