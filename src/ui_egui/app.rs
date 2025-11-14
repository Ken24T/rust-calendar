use crate::models::settings::Settings;
use crate::services::countdown::{
    CountdownCardGeometry, CountdownCardId, CountdownCardState, CountdownCardVisuals,
    CountdownService, RgbaColor,
};
use crate::services::database::Database;
use crate::services::settings::SettingsService;
use crate::services::theme::ThemeService;
use crate::ui_egui::dialogs::theme_creator::{
    render_theme_creator, ThemeCreatorAction, ThemeCreatorState,
};
use crate::ui_egui::dialogs::theme_dialog::{
    render_theme_dialog, ThemeDialogAction, ThemeDialogState,
};
use crate::ui_egui::event_dialog::{render_event_dialog, EventDialogState};
use crate::ui_egui::settings_dialog::render_settings_dialog;
use crate::ui_egui::theme::CalendarTheme;
use crate::ui_egui::views::day_view::DayView;
use crate::ui_egui::views::month_view::MonthView;
use crate::ui_egui::views::quarter_view::QuarterView;
use crate::ui_egui::views::week_view::WeekView;
use crate::ui_egui::views::workweek_view::WorkWeekView;
use crate::ui_egui::views::CountdownRequest;
use chrono::{Duration as ChronoDuration, Local, LocalResult, NaiveDate, NaiveTime, TimeZone};
use directories::ProjectDirs;
use egui_extras::DatePickerButton;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::Duration as StdDuration,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    Day,
    Week,
    WorkWeek,
    Month,
    Quarter,
}

enum CountdownCardUiAction {
    None,
    Close,
    OpenSettings,
    GeometrySettled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CountdownRenderSnapshot {
    waiting_on_geometry: bool,
    geometry: CountdownCardGeometry,
}

struct CountdownSettingsUiResult {
    commands: Vec<CountdownSettingsCommand>,
    close_requested: bool,
}

impl CountdownSettingsUiResult {
    fn new() -> Self {
        Self {
            commands: Vec::new(),
            close_requested: false,
        }
    }
}

enum CountdownSettingsCommand {
    SetTitleOverride(CountdownCardId, Option<String>),
    SetComment(CountdownCardId, Option<String>),
    SetAlwaysOnTop(CountdownCardId, bool),
    SetCompactMode(CountdownCardId, bool),
    SetDaysFontSize(CountdownCardId, f32),
    SetTitleBgColor(CountdownCardId, RgbaColor),
    SetTitleFgColor(CountdownCardId, RgbaColor),
    SetBodyBgColor(CountdownCardId, RgbaColor),
    SetDaysFgColor(CountdownCardId, RgbaColor),
    ApplyVisualDefaults(CountdownCardId),
    DeleteCard(CountdownCardId),
    SetStartAt(CountdownCardId, chrono::DateTime<chrono::Local>),
    SetDefaultTitleBgColor(RgbaColor),
    ResetDefaultTitleBgColor,
    SetDefaultTitleFgColor(RgbaColor),
    ResetDefaultTitleFgColor,
    SetDefaultBodyBgColor(RgbaColor),
    ResetDefaultBodyBgColor,
    SetDefaultDaysFgColor(RgbaColor),
    ResetDefaultDaysFgColor,
    SetDefaultDaysFontSize(f32),
    ResetDefaultDaysFontSize,
}

const MAX_PENDING_GEOMETRY_FRAMES: u32 = 120;
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
    theme_dialog_state: ThemeDialogState,
    theme_creator_state: ThemeCreatorState,

    // Event dialog state
    event_dialog_state: Option<EventDialogState>,
    event_dialog_date: Option<NaiveDate>,
    event_dialog_time: Option<chrono::NaiveTime>, // Time from clicked cell (None = use default)
    event_dialog_recurrence: Option<String>,
    event_to_edit: Option<i64>, // Event ID to edit

    // Countdown cards
    countdown_service: CountdownService,
    countdown_storage_path: PathBuf,
    open_countdown_settings: HashSet<CountdownCardId>,
    countdown_settings_geometry: HashMap<CountdownCardId, CountdownCardGeometry>,
    countdown_settings_needs_layout: HashSet<CountdownCardId>,
    countdown_cards_pending_visibility: HashSet<CountdownCardId>,
    countdown_cards_geometry_attempts: HashMap<CountdownCardId, u32>,
    pending_root_geometry: Option<CountdownCardGeometry>,
    countdown_render_log_state: HashMap<CountdownCardId, CountdownRenderSnapshot>,
}

impl CalendarApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize database and leak it for 'static lifetime
        // This is necessary because eframe requires 'static App implementations
        let database = {
            let db = Database::new("calendar.db").expect("Failed to create database connection");

            // Initialize schema (create tables and insert defaults)
            db.initialize_schema()
                .expect("Failed to initialize database schema");

            Box::leak(Box::new(db))
        };

        // Create temporary services to load settings
        let settings_service = SettingsService::new(database);

        // Load settings or create defaults
        let settings = match settings_service.get() {
            Ok(settings) => {
                eprintln!("Loaded settings from database - theme: {}", settings.theme);
                settings
            }
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

        eprintln!("Applying theme: {}", settings.theme);

        // Parse current view from settings
        let current_view = Self::parse_view_type(&settings.current_view);

        let countdown_storage_path = Self::resolve_countdown_storage_path();
        cc.egui_ctx.set_embed_viewports(false);
        let countdown_service = match CountdownService::load_from_disk(&countdown_storage_path) {
            Ok(service) => service,
            Err(err) => {
                log::warn!(
                    "Failed to load countdown cards from {}: {err:?}",
                    countdown_storage_path.display()
                );
                CountdownService::new()
            }
        };

        let pending_root_geometry = countdown_service.app_window_geometry();
        let countdown_cards_pending_visibility: HashSet<CountdownCardId> = countdown_service
            .cards()
            .iter()
            .map(|card| card.id)
            .collect();
        let countdown_cards_geometry_attempts: HashMap<CountdownCardId, u32> =
            countdown_cards_pending_visibility
                .iter()
                .map(|id| (*id, 0))
                .collect();

        let app = Self {
            database,
            settings,
            current_view,
            current_date: Local::now().date_naive(),
            show_event_dialog: false,
            show_settings_dialog: false,
            theme_dialog_state: ThemeDialogState::new(),
            theme_creator_state: ThemeCreatorState::new(),
            event_dialog_state: None,
            event_dialog_date: None,
            event_dialog_time: None,
            event_dialog_recurrence: None,
            event_to_edit: None,
            countdown_service,
            countdown_storage_path,
            open_countdown_settings: HashSet::new(),
            countdown_settings_geometry: HashMap::new(),
            countdown_settings_needs_layout: HashSet::new(),
            countdown_cards_pending_visibility,
            countdown_cards_geometry_attempts,
            pending_root_geometry,
            countdown_render_log_state: HashMap::new(),
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
            "Quarter" => ViewType::Quarter,
            _ => ViewType::Month, // Default fallback
        }
    }

    fn apply_theme(ctx: &egui::Context, settings: &Settings) {
        // Try to load custom theme from database
        // Note: We need a database reference, but we're in a static method
        // For now, we'll just apply basic Light/Dark themes
        // Custom themes are applied directly in render_theme_manager
        let visuals = if settings.theme.to_lowercase().contains("dark") {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        ctx.set_visuals(visuals);
    }

    fn apply_theme_from_db(&self, ctx: &egui::Context) {
        let theme_service = ThemeService::new(self.database);

        // Try to load the theme
        if let Ok(theme) = theme_service.get_theme(&self.settings.theme) {
            eprintln!("Applying custom theme: {}", self.settings.theme);
            theme.apply_to_context(ctx);
        } else {
            eprintln!(
                "Theme not found, using default for: {}",
                self.settings.theme
            );
            Self::apply_theme(ctx, &self.settings);
        }
    }
}

fn rgba_to_color32(color: RgbaColor) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

fn color32_to_rgba(color: egui::Color32) -> RgbaColor {
    RgbaColor {
        r: color.r(),
        g: color.g(),
        b: color.b(),
        a: color.a(),
    }
}

impl eframe::App for CalendarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            // Close dialogs in priority order
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

        self.apply_pending_root_geometry(ctx);

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Settings").clicked() {
                        self.show_settings_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
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
                    if ui
                        .selectable_label(self.current_view == ViewType::Quarter, "Quarter")
                        .clicked()
                    {
                        self.current_view = ViewType::Quarter;
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
                    if ui.button("New Event...").clicked() {
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

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!(
                "{} View - {}",
                match self.current_view {
                    ViewType::Day => "Day",
                    ViewType::Week => "Week",
                    ViewType::WorkWeek => "Work Week",
                    ViewType::Month => "Month",
                    ViewType::Quarter => "Quarter",
                },
                self.current_date.format("%B %Y")
            ));

            ui.separator();

            // Navigation buttons
            ui.horizontal(|ui| {
                if ui.button("◀ Previous").clicked() {
                    self.navigate_previous();
                }
                if ui.button("Today").clicked() {
                    self.current_date = Local::now().date_naive();
                }
                if ui.button("Next ▶").clicked() {
                    self.navigate_next();
                }
            });

            ui.separator();

            // View content (placeholder for now)
            match self.current_view {
                ViewType::Day => self.render_day_view(ui, &mut countdown_requests),
                ViewType::Week => self.render_week_view(ui, &mut countdown_requests),
                ViewType::WorkWeek => self.render_workweek_view(ui, &mut countdown_requests),
                ViewType::Month => self.render_month_view(ui),
                ViewType::Quarter => self.render_quarter_view(ui),
            }
        });

        if !countdown_requests.is_empty() {
            self.consume_countdown_requests(countdown_requests);
        }

        self.render_countdown_cards(ctx);
        self.render_countdown_settings_dialogs(ctx);

        // Dialogs (to be implemented)
        self.capture_root_geometry(ctx);
        if self.show_event_dialog {
            // Create dialog state if not already present
            if self.event_dialog_state.is_none() {
                // Check if we're editing an existing event
                if let Some(event_id) = self.event_to_edit {
                    // Load event from database
                    use crate::services::event::EventService;
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
        self.persist_countdowns_if_needed();

        // Render unified theme dialog and creator
        self.render_theme_dialog(ctx);
        self.render_theme_creator(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist_countdowns_if_needed();
    }
}

impl CalendarApp {
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
            ViewType::Quarter => {
                let new_month = if self.current_date.month() <= 3 {
                    10
                } else {
                    self.current_date.month() - 3
                };
                let year = if self.current_date.month() <= 3 {
                    self.current_date.year() - 1
                } else {
                    self.current_date.year()
                };
                NaiveDate::from_ymd_opt(year, new_month, 1).unwrap()
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
            ViewType::Quarter => {
                let new_month = if self.current_date.month() >= 10 {
                    1
                } else {
                    self.current_date.month() + 3
                };
                let year = if self.current_date.month() >= 10 {
                    self.current_date.year() + 1
                } else {
                    self.current_date.year()
                };
                NaiveDate::from_ymd_opt(year, new_month, 1).unwrap()
            }
        };
    }

    // View renderers
    fn render_day_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
    ) {
        if let Some(clicked_event) = DayView::show(
            ui,
            &mut self.current_date,
            self.database,
            &self.settings,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_time,
            &mut self.event_dialog_recurrence,
            countdown_requests,
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
    ) {
        if let Some(clicked_event) = WeekView::show(
            ui,
            &mut self.current_date,
            self.database,
            &self.settings,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_time,
            &mut self.event_dialog_recurrence,
            countdown_requests,
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
    ) {
        if let Some(clicked_event) = WorkWeekView::show(
            ui,
            &mut self.current_date,
            self.database,
            &self.settings,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_time,
            &mut self.event_dialog_recurrence,
            countdown_requests,
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
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_recurrence,
            &mut self.event_to_edit,
        );
    }

    fn render_quarter_view(&mut self, ui: &mut egui::Ui) {
        QuarterView::show(
            ui,
            &mut self.current_date,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_recurrence,
            &self.settings,
        );
    }

    // Placeholder dialog renderers
    fn render_event_dialog(&mut self, ctx: &egui::Context) {
        if let Some(ref mut state) = self.event_dialog_state {
            let saved = render_event_dialog(
                ctx,
                state,
                self.database,
                &self.settings,
                &mut self.show_event_dialog,
            );

            // If saved, clear the dialog state
            if saved || !self.show_event_dialog {
                self.event_dialog_state = None;
                self.event_dialog_time = None;
            }
        } else {
            // No state - shouldn't happen, but close dialog if it does
            self.show_event_dialog = false;
        }
    }

    fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        let saved = render_settings_dialog(
            ctx,
            &mut self.settings,
            self.database,
            &mut self.show_settings_dialog,
        );

        // If settings were saved, apply theme
        if saved {
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
                eprintln!("Applying theme: {}", name);

                // Apply the custom theme or built-in theme
                if let Ok(theme) = theme_service.get_theme(&name) {
                    theme.apply_to_context(ctx);
                } else {
                    Self::apply_theme(ctx, &self.settings);
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

    fn persist_countdowns_if_needed(&mut self) {
        if !self.countdown_service.is_dirty() {
            return;
        }
        if let Err(err) = self
            .countdown_service
            .save_to_disk(&self.countdown_storage_path)
        {
            log::error!(
                "Failed to persist countdown cards to {}: {err:?}",
                self.countdown_storage_path.display()
            );
        } else {
            self.countdown_service.mark_clean();
        }
    }

    fn resolve_countdown_storage_path() -> PathBuf {
        if let Some(dirs) = ProjectDirs::from("com", "RustCalendar", "DesktopApp") {
            dirs.data_dir().join("countdown_cards.json")
        } else {
            PathBuf::from("countdown_cards.json")
        }
    }

    fn consume_countdown_requests(&mut self, requests: Vec<CountdownRequest>) {
        let now = Local::now();
        for request in requests {
            if request.start_at <= now {
                log::info!(
                    "Skipping countdown for past event {:?} ({}): {:?}",
                    request.event_id,
                    request.title,
                    request.start_at
                );
                continue;
            }
            let card_id = self.countdown_service.create_card(
                request.event_id,
                request.title,
                request.start_at,
            );
            log::info!("created countdown card {:?}", card_id);
        }
    }

    fn should_wait_on_card_geometry(&mut self, card_id: CountdownCardId) -> bool {
        if !self.countdown_cards_pending_visibility.contains(&card_id) {
            return false;
        }

        let exceeded_limit = {
            let attempts = self
                .countdown_cards_geometry_attempts
                .entry(card_id)
                .or_insert(0);
            log::debug!(
                "card {:?} geometry attempt {} (limit {})",
                card_id,
                *attempts + 1,
                MAX_PENDING_GEOMETRY_FRAMES
            );
            if *attempts >= MAX_PENDING_GEOMETRY_FRAMES {
                true
            } else {
                *attempts += 1;
                false
            }
        };

        if exceeded_limit {
            self.clear_geometry_wait_state(&card_id);
            log::warn!(
                "Countdown card {:?} geometry did not settle after {} frames; forcing visibility",
                card_id,
                MAX_PENDING_GEOMETRY_FRAMES
            );
            false
        } else {
            true
        }
    }

    fn clear_geometry_wait_state(&mut self, card_id: &CountdownCardId) {
        self.countdown_cards_pending_visibility.remove(card_id);
        self.countdown_cards_geometry_attempts.remove(card_id);
    }

    fn render_countdown_cards(&mut self, ctx: &egui::Context) {
        let cards = self.countdown_service.cards().to_vec();
        if cards.is_empty() {
            return;
        }

        let now = Local::now();
        let mut removals = Vec::new();

        for card in cards {
            let viewport_id = egui::ViewportId::from_hash_of(("countdown_card", card.id.0));
            let waiting_on_geometry = self.should_wait_on_card_geometry(card.id);
            let snapshot = CountdownRenderSnapshot {
                waiting_on_geometry,
                geometry: card.geometry,
            };
            let should_log = self
                .countdown_render_log_state
                .get(&card.id)
                .map(|last| last != &snapshot)
                .unwrap_or(true);
            if should_log {
                log::debug!(
                    "rendering card {:?} title='{}' waiting={} geom={:?}",
                    card.id,
                    card.effective_title(),
                    waiting_on_geometry,
                    card.geometry
                );
                self.countdown_render_log_state.insert(card.id, snapshot);
            }
            let builder = self.viewport_builder_for_card(&card, waiting_on_geometry);

            let card_clone = card.clone();
            let action =
                ctx.show_viewport_immediate(viewport_id, builder, move |child_ctx, class| {
                    Self::render_countdown_card_ui(
                        child_ctx,
                        class,
                        viewport_id,
                        &card_clone,
                        now,
                        waiting_on_geometry,
                    )
                });

            let viewport_info = viewport_info(ctx, viewport_id);
            let close_via_window = viewport_info
                .as_ref()
                .map(|info| info.close_requested())
                .unwrap_or(false);

            let mut queued_close = close_via_window;

            match action {
                CountdownCardUiAction::None => {}
                CountdownCardUiAction::Close => queued_close = true,
                CountdownCardUiAction::OpenSettings => {
                    self.open_countdown_settings.insert(card.id);
                    let default_geometry = self.default_settings_geometry_for(&card);
                    self.countdown_settings_geometry
                        .entry(card.id)
                        .or_insert(default_geometry);
                    self.countdown_settings_needs_layout.insert(card.id);
                }
                CountdownCardUiAction::GeometrySettled => {
                    self.clear_geometry_wait_state(&card.id);
                    log::debug!("card {:?} geometry settled", card.id);
                }
            }

            if queued_close {
                removals.push(card.id);
                continue;
            }

            if let Some(info) = viewport_info.as_ref() {
                if !waiting_on_geometry {
                    if viewport_title_matches(info, card.effective_title()) {
                        if let Some(current_geometry) = geometry_from_viewport_info(info) {
                            if geometry_changed(card.geometry, current_geometry)
                                && self
                                    .countdown_service
                                    .queue_geometry_update(card.id, current_geometry)
                            {
                                log::debug!(
                                    "queue geometry update for card {:?}: {:?} -> {:?}",
                                    card.id,
                                    card.geometry,
                                    current_geometry
                                );
                            }
                        }
                    }
                }
            }
        }

        for id in removals {
            self.countdown_service.remove_card(id);
            self.open_countdown_settings.remove(&id);
            self.countdown_settings_geometry.remove(&id);
            self.countdown_settings_needs_layout.remove(&id);
            self.clear_geometry_wait_state(&id);
            self.countdown_render_log_state.remove(&id);
        }

        self.countdown_service.flush_geometry_updates();
    }

    fn render_countdown_settings_dialogs(&mut self, ctx: &egui::Context) {
        if self.open_countdown_settings.is_empty() {
            return;
        }

        let cards_snapshot = self.countdown_service.cards().to_vec();
        self.open_countdown_settings
            .retain(|id| cards_snapshot.iter().any(|card| &card.id == id));

        let mut dialogs_to_close = Vec::new();
        let defaults_snapshot = self.countdown_service.defaults().clone();
        let open_windows: Vec<_> = self.open_countdown_settings.iter().copied().collect();

        for id in open_windows {
            if let Some(card) = cards_snapshot.iter().find(|card| card.id == id) {
                let default_geometry = self.default_settings_geometry_for(card);
                let geometry_copy = {
                    let entry = self
                        .countdown_settings_geometry
                        .entry(id)
                        .or_insert(default_geometry);
                    *entry
                };
                let viewport_id = egui::ViewportId::from_hash_of(("countdown_settings", card.id.0));
                let apply_layout = self.countdown_settings_needs_layout.remove(&id);
                let settings_title = format!("Settings: {}", card.effective_title());
                let builder = self.viewport_builder_for_settings(
                    if apply_layout {
                        Some(geometry_copy)
                    } else {
                        None
                    },
                    card,
                );

                let card_clone = card.clone();
                let defaults_clone = defaults_snapshot.clone();
                let result =
                    ctx.show_viewport_immediate(viewport_id, builder, move |child_ctx, class| {
                        Self::render_countdown_settings_ui(
                            child_ctx,
                            class,
                            &card_clone,
                            &defaults_clone,
                        )
                    });

                let viewport_info = viewport_info(ctx, viewport_id);
                let mut should_close = viewport_info
                    .as_ref()
                    .map(|info| info.close_requested())
                    .unwrap_or(false);

                for command in result.commands {
                    if self.apply_countdown_settings_command(command) {
                        should_close = true;
                    }
                }

                if result.close_requested {
                    should_close = true;
                }

                if let Some(info) = viewport_info.as_ref() {
                    if viewport_title_matches(info, &settings_title) {
                        if let Some(geometry) = geometry_from_viewport_info(info) {
                            if let Some(entry) = self.countdown_settings_geometry.get_mut(&id) {
                                *entry = geometry;
                            }
                        }
                    }
                }

                if should_close {
                    dialogs_to_close.push(id);
                }
            } else {
                dialogs_to_close.push(id);
            }
        }

        for id in dialogs_to_close {
            self.open_countdown_settings.remove(&id);
            self.countdown_settings_geometry.remove(&id);
            self.countdown_settings_needs_layout.remove(&id);
        }
    }

    fn apply_countdown_settings_command(&mut self, command: CountdownSettingsCommand) -> bool {
        match command {
            CountdownSettingsCommand::SetTitleOverride(id, title) => {
                self.countdown_service.set_title_override(id, title);
                false
            }
            CountdownSettingsCommand::SetComment(id, comment) => {
                self.countdown_service.set_comment(id, comment);
                false
            }
            CountdownSettingsCommand::SetAlwaysOnTop(id, value) => {
                self.countdown_service.set_always_on_top(id, value);
                false
            }
            CountdownSettingsCommand::SetCompactMode(id, value) => {
                self.countdown_service.set_compact_mode(id, value);
                false
            }
            CountdownSettingsCommand::SetDaysFontSize(id, size) => {
                self.countdown_service.set_days_font_size(id, size);
                false
            }
            CountdownSettingsCommand::SetTitleBgColor(id, color) => {
                self.countdown_service.set_title_bg_color(id, color);
                false
            }
            CountdownSettingsCommand::SetTitleFgColor(id, color) => {
                self.countdown_service.set_title_fg_color(id, color);
                false
            }
            CountdownSettingsCommand::SetBodyBgColor(id, color) => {
                self.countdown_service.set_body_bg_color(id, color);
                false
            }
            CountdownSettingsCommand::SetDaysFgColor(id, color) => {
                self.countdown_service.set_days_fg_color(id, color);
                false
            }
            CountdownSettingsCommand::ApplyVisualDefaults(id) => {
                self.countdown_service.apply_visual_defaults(id);
                false
            }
            CountdownSettingsCommand::DeleteCard(id) => {
                self.countdown_service.remove_card(id);
                self.open_countdown_settings.remove(&id);
                self.countdown_settings_geometry.remove(&id);
                self.countdown_settings_needs_layout.remove(&id);
                self.clear_geometry_wait_state(&id);
                true
            }
            CountdownSettingsCommand::SetStartAt(id, start_at) => {
                self.countdown_service.set_start_at(id, start_at);
                false
            }
            CountdownSettingsCommand::SetDefaultTitleBgColor(color) => {
                self.countdown_service.set_default_title_bg_color(color);
                false
            }
            CountdownSettingsCommand::ResetDefaultTitleBgColor => {
                self.countdown_service.reset_default_title_bg_color();
                false
            }
            CountdownSettingsCommand::SetDefaultTitleFgColor(color) => {
                self.countdown_service.set_default_title_fg_color(color);
                false
            }
            CountdownSettingsCommand::ResetDefaultTitleFgColor => {
                self.countdown_service.reset_default_title_fg_color();
                false
            }
            CountdownSettingsCommand::SetDefaultBodyBgColor(color) => {
                self.countdown_service.set_default_body_bg_color(color);
                false
            }
            CountdownSettingsCommand::ResetDefaultBodyBgColor => {
                self.countdown_service.reset_default_body_bg_color();
                false
            }
            CountdownSettingsCommand::SetDefaultDaysFgColor(color) => {
                self.countdown_service.set_default_days_fg_color(color);
                false
            }
            CountdownSettingsCommand::ResetDefaultDaysFgColor => {
                self.countdown_service.reset_default_days_fg_color();
                false
            }
            CountdownSettingsCommand::SetDefaultDaysFontSize(size) => {
                self.countdown_service.set_default_days_font_size(size);
                false
            }
            CountdownSettingsCommand::ResetDefaultDaysFontSize => {
                self.countdown_service.reset_default_days_font_size();
                false
            }
        }
    }

    fn viewport_builder_for_card(
        &self,
        card: &CountdownCardState,
        start_hidden: bool,
    ) -> egui::ViewportBuilder {
        let mut builder = egui::ViewportBuilder::default()
            .with_title(card.effective_title().to_owned())
            .with_position(egui::pos2(card.geometry.x, card.geometry.y))
            .with_inner_size(egui::vec2(
                card.geometry.width.max(120.0),
                card.geometry.height.max(90.0),
            ))
            .with_resizable(true)
            .with_transparent(false);

        if card.visuals.always_on_top {
            builder = builder.with_always_on_top();
        }

        if start_hidden {
            builder = builder.with_visible(false);
        }

        builder
    }

    fn default_settings_geometry_for(&self, card: &CountdownCardState) -> CountdownCardGeometry {
        CountdownCardGeometry {
            x: card.geometry.x + card.geometry.width + 16.0,
            y: card.geometry.y,
            width: 280.0,
            height: 560.0,
        }
    }

    fn viewport_builder_for_settings(
        &self,
        geometry: Option<CountdownCardGeometry>,
        card: &CountdownCardState,
    ) -> egui::ViewportBuilder {
        let mut builder = egui::ViewportBuilder::default()
            .with_title(format!("Settings: {}", card.effective_title()))
            .with_resizable(false);

        if let Some(geometry) = geometry {
            builder = builder
                .with_position(egui::pos2(geometry.x, geometry.y))
                .with_inner_size(egui::vec2(
                    geometry.width.max(260.0),
                    geometry.height.max(540.0),
                ));
        }

        builder
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

    fn render_countdown_card_ui(
        ctx: &egui::Context,
        class: egui::ViewportClass,
        viewport_id: egui::ViewportId,
        card: &CountdownCardState,
        now: chrono::DateTime<chrono::Local>,
        waiting_on_geometry: bool,
    ) -> CountdownCardUiAction {
        ctx.request_repaint_after(StdDuration::from_secs(1));

        if !waiting_on_geometry {
            ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(true));
        }

        let title_bg = rgba_to_color32(card.visuals.title_bg_color);
        let title_fg = rgba_to_color32(card.visuals.title_fg_color);
        let body_bg = rgba_to_color32(card.visuals.body_bg_color);
        let days_fg = rgba_to_color32(card.visuals.days_fg_color);
        let font_size = card.visuals.days_font_size.max(32.0);

        let mut geometry_settled = false;
        if waiting_on_geometry {
            let target_position = egui::pos2(card.geometry.x, card.geometry.y);
            let target_size = egui::vec2(card.geometry.width, card.geometry.height);
            ctx.send_viewport_cmd_to(
                viewport_id,
                egui::ViewportCommand::OuterPosition(target_position),
            );
            ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::InnerSize(target_size));
            log::debug!(
                "card {:?} forcing position {:?} and size {:?}",
                card.id,
                target_position,
                target_size
            );

            geometry_settled = ctx.input(|input| {
                let info = input.viewport();
                geometry_from_viewport_info(info)
                    .map(|current| !geometry_changed(card.geometry, current))
                    .unwrap_or(false)
            });

            ctx.input(|input| {
                if let Some(current) = geometry_from_viewport_info(input.viewport()) {
                    log::debug!(
                        "card {:?} current viewport geometry: {:?}",
                        card.id,
                        current
                    );
                }
            });

            if geometry_settled {
                log::debug!("card {:?} geometry settled", card.id);
                ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(true));
            } else {
                ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(false));
            }
        }

        let render = |ui: &mut egui::Ui| {
            let mut action = CountdownCardUiAction::None;
            let frame = egui::Frame::none()
                .fill(body_bg)
                .rounding(egui::Rounding::from(8.0))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(40)));

            let inner = frame.show(ui, |ui| {
                ui.vertical(|ui| {
                    egui::Frame::none()
                        .fill(title_bg)
                        .rounding(egui::Rounding::from(8.0))
                        .show(ui, |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    egui::RichText::new(card.effective_title())
                                        .color(title_fg)
                                        .strong(),
                                );
                            });
                        });

                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        let days_remaining = (card.start_at.date_naive() - now.date_naive())
                            .num_days()
                            .max(0);
                        ui.label(
                            egui::RichText::new(days_remaining.to_string())
                                .size(font_size)
                                .color(days_fg),
                        );
                    });
                    ui.add_space(4.0);
                    if let Some(comment) = card.comment.as_ref().and_then(|text| {
                        let trimmed = text.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_owned())
                        }
                    }) {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(comment)
                                    .color(days_fg)
                                    .size(font_size.min(32.0)),
                            )
                            .wrap(),
                        );
                        ui.add_space(4.0);
                    }
                });
            });

            let response = ui.interact(
                inner.response.rect,
                ui.make_persistent_id(("countdown_card_surface", card.id.0)),
                egui::Sense::click(),
            );
            response.context_menu(|ui| {
                if ui.button("Card settings...").clicked() {
                    action = CountdownCardUiAction::OpenSettings;
                    ui.close_menu();
                }
                if ui.button("Close countdown").clicked() {
                    action = CountdownCardUiAction::Close;
                    ui.close_menu();
                }
            });

            action
        };

        match class {
            egui::ViewportClass::Embedded => {
                let mut action = CountdownCardUiAction::None;
                egui::Window::new(card.effective_title())
                    .collapsible(false)
                    .resizable(true)
                    .show(ctx, |ui| {
                        action = render(ui);
                    });
                if geometry_settled {
                    CountdownCardUiAction::GeometrySettled
                } else {
                    action
                }
            }
            _ => {
                let mut action = CountdownCardUiAction::None;
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(body_bg))
                    .show(ctx, |ui| {
                        action = render(ui);
                    });
                if geometry_settled {
                    CountdownCardUiAction::GeometrySettled
                } else {
                    action
                }
            }
        }
    }

    fn render_countdown_settings_ui(
        ctx: &egui::Context,
        _class: egui::ViewportClass,
        card: &CountdownCardState,
        defaults: &CountdownCardVisuals,
    ) -> CountdownSettingsUiResult {
        let mut result = CountdownSettingsUiResult::new();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_width(320.0);
            ui.heading(card.effective_title());
            ui.add_space(8.0);

            ui.label("Title");
            let mut title_text = card
                .title_override
                .clone()
                .unwrap_or_else(|| card.event_title.clone());
            if ui
                .add(
                    egui::TextEdit::singleline(&mut title_text)
                        .desired_width(f32::INFINITY)
                        .hint_text("Countdown title"),
                )
                .changed()
            {
                let trimmed = title_text.trim();
                let payload = if trimmed.is_empty() || trimmed == card.event_title {
                    None
                } else {
                    Some(trimmed.to_owned())
                };
                result
                    .commands
                    .push(CountdownSettingsCommand::SetTitleOverride(card.id, payload));
            }
            if ui.button("Reset to event name").clicked() {
                result
                    .commands
                    .push(CountdownSettingsCommand::SetTitleOverride(card.id, None));
            }

            ui.separator();
            ui.label("Date");
            let mut target_date = card.start_at.date_naive();
            let date_picker_id = format!("countdown_date_{}", card.id.0);
            if ui
                .add(DatePickerButton::new(&mut target_date).id_source(date_picker_id.as_str()))
                .changed()
            {
                let new_dt = combine_date_with_time(target_date, card.start_at.time());
                result
                    .commands
                    .push(CountdownSettingsCommand::SetStartAt(card.id, new_dt));
            }

            ui.separator();
            ui.heading("Layout");
            let mut font_size = card.visuals.days_font_size;
            if ui
                .add(egui::Slider::new(&mut font_size, 32.0..=220.0).text("Size"))
                .changed()
            {
                result
                    .commands
                    .push(CountdownSettingsCommand::SetDaysFontSize(
                        card.id, font_size,
                    ));
            }

            let mut always_on_top = card.visuals.always_on_top;
            if ui.checkbox(&mut always_on_top, "Always on top").changed() {
                result
                    .commands
                    .push(CountdownSettingsCommand::SetAlwaysOnTop(
                        card.id,
                        always_on_top,
                    ));
            }
            let mut compact_mode = card.visuals.compact_mode;
            if ui.checkbox(&mut compact_mode, "Compact mode").changed() {
                result
                    .commands
                    .push(CountdownSettingsCommand::SetCompactMode(
                        card.id,
                        compact_mode,
                    ));
            }

            let mut font_default = (font_size - defaults.days_font_size).abs() < 0.5;
            if ui
                .checkbox(&mut font_default, "Default Font Size")
                .changed()
            {
                if font_default {
                    result
                        .commands
                        .push(CountdownSettingsCommand::SetDefaultDaysFontSize(font_size));
                } else {
                    result
                        .commands
                        .push(CountdownSettingsCommand::ResetDefaultDaysFontSize);
                }
            }

            ui.separator();
            ui.heading("Comment");
            let mut comment_text = card.comment.clone().unwrap_or_default();
            if ui
                .add(
                    egui::TextEdit::multiline(&mut comment_text)
                        .desired_rows(4)
                        .hint_text("Add notes for this countdown"),
                )
                .changed()
            {
                let payload = if comment_text.trim().is_empty() {
                    None
                } else {
                    Some(comment_text.clone())
                };
                result
                    .commands
                    .push(CountdownSettingsCommand::SetComment(card.id, payload));
            }

            ui.separator();
            ui.heading("Colors");
            Self::render_color_setting(
                ui,
                "Title Background",
                card.visuals.title_bg_color,
                defaults.title_bg_color,
                |color| CountdownSettingsCommand::SetTitleBgColor(card.id, color),
                |color| CountdownSettingsCommand::SetDefaultTitleBgColor(color),
                CountdownSettingsCommand::ResetDefaultTitleBgColor,
                &mut result,
            );
            Self::render_color_setting(
                ui,
                "Title Text",
                card.visuals.title_fg_color,
                defaults.title_fg_color,
                |color| CountdownSettingsCommand::SetTitleFgColor(card.id, color),
                |color| CountdownSettingsCommand::SetDefaultTitleFgColor(color),
                CountdownSettingsCommand::ResetDefaultTitleFgColor,
                &mut result,
            );
            Self::render_color_setting(
                ui,
                "Card Background",
                card.visuals.body_bg_color,
                defaults.body_bg_color,
                |color| CountdownSettingsCommand::SetBodyBgColor(card.id, color),
                |color| CountdownSettingsCommand::SetDefaultBodyBgColor(color),
                CountdownSettingsCommand::ResetDefaultBodyBgColor,
                &mut result,
            );
            Self::render_color_setting(
                ui,
                "Days Text",
                card.visuals.days_fg_color,
                defaults.days_fg_color,
                |color| CountdownSettingsCommand::SetDaysFgColor(card.id, color),
                |color| CountdownSettingsCommand::SetDefaultDaysFgColor(color),
                CountdownSettingsCommand::ResetDefaultDaysFgColor,
                &mut result,
            );

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Reset").clicked() {
                    result
                        .commands
                        .push(CountdownSettingsCommand::ApplyVisualDefaults(card.id));
                }
                if ui.button("Save").clicked() {
                    result.close_requested = true;
                }
                let delete_clicked = ui
                    .add(egui::Button::new("Delete").fill(egui::Color32::from_rgb(185, 28, 28)))
                    .clicked();
                if delete_clicked {
                    result
                        .commands
                        .push(CountdownSettingsCommand::DeleteCard(card.id));
                    result.close_requested = true;
                }
                if ui.button("Cancel").clicked() {
                    result.close_requested = true;
                }
            });
        });

        result
    }

    fn render_color_setting<F, G>(
        ui: &mut egui::Ui,
        label: &str,
        color_value: RgbaColor,
        default_value: RgbaColor,
        mut on_color_change: F,
        mut on_set_default: G,
        reset_default_command: CountdownSettingsCommand,
        result: &mut CountdownSettingsUiResult,
    ) where
        F: FnMut(RgbaColor) -> CountdownSettingsCommand,
        G: FnMut(RgbaColor) -> CountdownSettingsCommand,
    {
        ui.group(|ui| {
            ui.label(label);
            let mut color = rgba_to_color32(color_value);
            let mut current_value = color_value;
            if egui::color_picker::color_edit_button_srgba(
                ui,
                &mut color,
                egui::color_picker::Alpha::Opaque,
            )
            .changed()
            {
                let rgba = color32_to_rgba(color);
                current_value = rgba;
                result.commands.push(on_color_change(rgba));
            }
            let mut is_default = current_value == default_value;
            if ui
                .checkbox(&mut is_default, format!("Default {}", label))
                .changed()
            {
                if is_default {
                    result.commands.push(on_set_default(current_value));
                } else {
                    result.commands.push(reset_default_command);
                }
            }
        });
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

fn viewport_title_matches(info: &egui::ViewportInfo, expected: &str) -> bool {
    match info.title.as_deref() {
        Some(title) => title == expected,
        None => true,
    }
}

fn geometry_changed(a: CountdownCardGeometry, b: CountdownCardGeometry) -> bool {
    (a.x - b.x).abs() > 2.0
        || (a.y - b.y).abs() > 2.0
        || (a.width - b.width).abs() > 1.0
        || (a.height - b.height).abs() > 1.0
}

fn combine_date_with_time(date: NaiveDate, time: NaiveTime) -> chrono::DateTime<chrono::Local> {
    let mut naive = date.and_time(time);
    for _ in 0..3 {
        match Local.from_local_datetime(&naive) {
            LocalResult::Single(dt) => return dt,
            LocalResult::Ambiguous(dt, _) => return dt,
            LocalResult::None => naive += ChronoDuration::minutes(30),
        }
    }
    Local::now()
}
