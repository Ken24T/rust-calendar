use crate::models::settings::Settings;
use crate::services::countdown::{CountdownCardGeometry, CountdownService};
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
use chrono::{Local, NaiveDate};
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    Day,
    Week,
    WorkWeek,
    Month,
    Quarter,
}

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

        // Dialogs (to be implemented)
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
        for request in requests {
            let card_id = self.countdown_service.create_card(
                request.event_id,
                request.title,
                request.start_at,
            );
            log::info!("created countdown card {:?}", card_id);
        }
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
            let builder = self.viewport_builder_for_card(&card);

            let card_clone = card.clone();
            let close_via_ui = ctx.show_viewport_immediate(viewport_id, builder, move |child_ctx, class| {
                Self::render_countdown_card_ui(child_ctx, class, &card_clone, now)
            });

            let close_via_window = ctx.input(|input| {
                input
                    .raw
                    .viewports
                    .get(&viewport_id)
                    .map(|info| info.close_requested())
                    .unwrap_or(false)
            });

            if close_via_ui || close_via_window {
                removals.push(card.id);
                continue;
            }

            if let Some((outer_left, outer_top, inner_rect)) = ctx.input(|input| {
                input
                    .raw
                    .viewports
                    .get(&viewport_id)
                    .and_then(|info| {
                        info.inner_rect.map(|inner| {
                            let (outer_left, outer_top) = info
                                .outer_rect
                                .map(|outer| (outer.left(), outer.top()))
                                .unwrap_or((inner.left(), inner.top()));
                            (outer_left, outer_top, inner)
                        })
                    })
            }) {
                let new_geometry = CountdownCardGeometry {
                    x: outer_left,
                    y: outer_top,
                    width: inner_rect.width(),
                    height: inner_rect.height(),
                };

                if geometry_changed(card.geometry, new_geometry) {
                    self
                        .countdown_service
                        .queue_geometry_update(card.id, new_geometry);
                }
            }
        }

        for id in removals {
            self.countdown_service.remove_card(id);
        }

        self.countdown_service.flush_geometry_updates();

        fn geometry_changed(a: CountdownCardGeometry, b: CountdownCardGeometry) -> bool {
            (a.x - b.x).abs() > 2.0
                || (a.y - b.y).abs() > 2.0
                || (a.width - b.width).abs() > 1.0
                || (a.height - b.height).abs() > 1.0
        }
    }

    fn viewport_builder_for_card(&self, card: &crate::services::countdown::CountdownCardState) -> egui::ViewportBuilder {
        let mut builder = egui::ViewportBuilder::default()
            .with_title(card.effective_title().to_owned())
            .with_position(egui::pos2(card.geometry.x, card.geometry.y))
            .with_inner_size(egui::vec2(card.geometry.width.max(120.0), card.geometry.height.max(90.0)))
            .with_resizable(true)
            .with_transparent(false);

        if card.visuals.always_on_top {
            builder = builder.with_always_on_top();
        }

        builder
    }

    fn render_countdown_card_ui(
        ctx: &egui::Context,
        class: egui::ViewportClass,
        card: &crate::services::countdown::CountdownCardState,
        now: chrono::DateTime<chrono::Local>,
    ) -> bool {
        let mut close_requested = false;

        let mut render_contents = |ui: &mut egui::Ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.heading(card.effective_title());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            close_requested = true;
                        }
                    });
                });
                ui.separator();
                let days = card.compute_days_remaining(now);
                let suffix = if days == 1 { "" } else { "s" };
                ui.label(format!("{} day{} remaining", days, suffix));
                ui.label(card.start_at.format("Starts %b %d, %Y %H:%M").to_string());
                ui.add_space(4.0);
                ui.label("Countdown card UI coming soon...");
            });
        };

        match class {
            egui::ViewportClass::Embedded => {
                egui::Window::new(card.effective_title())
                    .collapsible(false)
                    .resizable(true)
                    .show(ctx, |ui| render_contents(ui)); // placeholder to fix compile
            }
            _ => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(ctx.style().visuals.window_fill()))
                    .show(ctx, |ui| {
                        render_contents(ui);
                    });
            }
        }

        close_requested
    }
}
