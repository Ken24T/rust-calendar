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
use crate::ui_egui::dialogs::backup_manager::{render_backup_manager_dialog, BackupManagerState};
use crate::ui_egui::event_dialog::EventDialogState;
use crate::ui_egui::theme::CalendarTheme;
use crate::ui_egui::views::CountdownRequest;
use chrono::{Datelike, Local};
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

    pub(super) fn handle_update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_file_drops(ctx);

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
                } else if self.state.theme_dialog_state.is_open {
                    self.state.theme_dialog_state.close();
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
                self.jump_to_today();
            }

            // Ctrl+S - Settings
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                self.show_settings_dialog = true;
            }

            // Ctrl+B - Backup Database
            if i.modifiers.ctrl && i.key_pressed(egui::Key::B) {
                if let Err(e) = self.state.backup_manager_state.create_backup() {
                    log::error!("Failed to create backup: {}", e);
                }
            }

            // Arrow keys for navigation (only when dialogs are closed)
            if !self.show_event_dialog
                && !self.show_settings_dialog
                && !self.state.theme_dialog_state.is_open
            {
                // Left/Right - Navigate backwards/forwards
                if i.key_pressed(egui::Key::ArrowLeft) {
                    self.navigate_previous();
                }
                if i.key_pressed(egui::Key::ArrowRight) {
                    self.navigate_next();
                }

                // Up/Down - Navigate up/down (weeks in week view, months in month view)
                if i.key_pressed(egui::Key::ArrowUp) {
                    self.navigate_up();
                }
                if i.key_pressed(egui::Key::ArrowDown) {
                    self.navigate_down();
                }
            }
        });

        self.apply_pending_root_geometry(ctx);

        self.render_menu_bar(ctx);

        let mut countdown_requests: Vec<CountdownRequest> = Vec::new();

        let active_countdown_events: std::collections::HashSet<i64> = self
            .context
            .countdown_service()
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
                    self.jump_to_today();
                }
                if ui.button("Next ▶").clicked() {
                    self.navigate_next();
                }

                ui.separator();

                // Year/Month picker
                ui.label("Jump to:");

                // Month dropdown
                let month_names = [
                    "January",
                    "February",
                    "March",
                    "April",
                    "May",
                    "June",
                    "July",
                    "August",
                    "September",
                    "October",
                    "November",
                    "December",
                ];
                let current_month = self.current_date.month() as usize;
                egui::ComboBox::from_id_source("month_picker")
                    .selected_text(month_names[current_month - 1])
                    .show_ui(ui, |ui| {
                        for (idx, month_name) in month_names.iter().enumerate() {
                            if ui
                                .selectable_value(&mut (idx + 1), current_month, *month_name)
                                .clicked()
                            {
                                let new_month = (idx + 1) as u32;
                                if let Some(new_date) = chrono::NaiveDate::from_ymd_opt(
                                    self.current_date.year(),
                                    new_month,
                                    1,
                                ) {
                                    self.current_date = new_date;
                                }
                            }
                        }
                    });

                // Year spinner
                let mut year = self.current_date.year();
                ui.add(
                    egui::DragValue::new(&mut year)
                        .range(1900..=2100)
                        .speed(1.0),
                );
                if year != self.current_date.year() {
                    if let Some(new_date) =
                        chrono::NaiveDate::from_ymd_opt(year, self.current_date.month(), 1)
                    {
                        self.current_date = new_date;
                    }
                }
            });

            ui.separator();

            // View content (ribbon is now rendered inside week view)
            let mut focus_request = self.pending_focus.take();
            match self.current_view {
                ViewType::Day => self.render_day_view(
                    ui,
                    &mut countdown_requests,
                    &active_countdown_events,
                    &mut focus_request,
                ),
                ViewType::Week => self.render_week_view(
                    ui,
                    &mut countdown_requests,
                    &active_countdown_events,
                    self.show_ribbon,
                    &mut focus_request,
                ),
                ViewType::WorkWeek => self.render_workweek_view(
                    ui,
                    &mut countdown_requests,
                    &active_countdown_events,
                    &mut focus_request,
                ),
                ViewType::Month => self.render_month_view(ui),
            }
            self.pending_focus = focus_request;
        });

        if !countdown_requests.is_empty() {
            self.consume_countdown_requests(countdown_requests);
        }

        self.countdown_ui
            .render_cards(ctx, self.context.countdown_service_mut());
        self.countdown_ui
            .render_settings_dialogs(ctx, self.context.countdown_service_mut());
        self.flush_pending_event_bodies();

        // Dialogs (to be implemented)
        self.capture_root_geometry(ctx);
        if self.show_event_dialog {
            // Create dialog state if not already present
            if self.event_dialog_state.is_none() {
                // Check if we're editing an existing event
                if let Some(event_id) = self.event_to_edit {
                    // Load event from database
                    let service = self.context.event_service();
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
        let changed_counts = self
            .context
            .countdown_service_mut()
            .refresh_days_remaining(Local::now());
        if !changed_counts.is_empty() {
            ctx.request_repaint();
        }

        // Check for notification triggers (warning state transitions)
        let now = Local::now();
        let notification_triggers = self
            .context
            .countdown_service_mut()
            .check_notification_triggers(now);

        if !notification_triggers.is_empty() {
            // Get notification config to check if system notifications are enabled
            let notification_config = self.context.countdown_service().notification_config();

            if notification_config.use_system_notifications {
                for (card_id, _old_state, new_state) in &notification_triggers {
                    let card_info = self
                        .context
                        .countdown_service()
                        .cards()
                        .iter()
                        .find(|c| c.id == *card_id)
                        .map(|card| (card.effective_title().to_owned(), card.start_at));

                    if let Some((title, start_at)) = card_info {
                        let (message, urgency) =
                            Self::notification_message_for_state(*new_state, start_at, now);

                        if let Err(e) = self
                            .context
                            .notification_service_mut()
                            .show_countdown_alert(&title, &message, urgency)
                        {
                            log::warn!("Failed to show system notification: {}", e);
                        } else {
                            log::info!(
                                "Showed system notification for card {:?} ({}) - state: {:?}",
                                card_id,
                                title,
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
        let dismissed_cards = self.context.countdown_service_mut().check_auto_dismiss(now);
        if !dismissed_cards.is_empty() {
            log::info!("Auto-dismissed {} countdown card(s)", dismissed_cards.len());
            ctx.request_repaint();
        }

        self.persist_countdowns_if_needed();

        // Render unified theme dialog and creator
        self.render_theme_dialog(ctx);
        self.render_theme_creator(ctx);

        // Render backup manager dialog
        let should_reload_db =
            render_backup_manager_dialog(ctx, &mut self.state.backup_manager_state);
        if should_reload_db {
            // Note: Database restoration requires app restart
            // Show message and close app
            log::info!("Database restored. Application should be restarted.");
            // TODO: Implement graceful restart or reload mechanism
        }
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
