use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::settings::SettingsService;
use crate::ui_egui::views::day_view::DayView;
use crate::ui_egui::views::week_view::WeekView;
use crate::ui_egui::views::workweek_view::WorkWeekView;
use crate::ui_egui::views::month_view::MonthView;
use crate::ui_egui::views::quarter_view::QuarterView;
use crate::ui_egui::event_dialog::{EventDialogState, render_event_dialog};
use crate::ui_egui::settings_dialog::render_settings_dialog;
use chrono::{Local, NaiveDate};

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
    show_theme_picker: bool,
    
    // Event dialog state
    event_dialog_state: Option<EventDialogState>,
    event_dialog_date: Option<NaiveDate>,
    event_dialog_recurrence: Option<String>,
    event_to_edit: Option<i64>, // Event ID to edit
}

impl CalendarApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize database and leak it for 'static lifetime
        // This is necessary because eframe requires 'static App implementations
        let database = Box::leak(Box::new(
            Database::new("calendar.db")
                .expect("Failed to initialize database")
        ));
        
        // Create temporary services to load settings
        let settings_service = SettingsService::new(database);
        
        // Load settings
        let settings = settings_service
            .get()
            .unwrap_or_else(|_| Settings::default());
        
        // Apply theme to egui
        Self::apply_theme(&cc.egui_ctx, &settings);
        
        Self {
            database,
            settings,
            current_view: ViewType::Month,
            current_date: Local::now().date_naive(),
            show_event_dialog: false,
            show_settings_dialog: false,
            show_theme_picker: false,
            event_dialog_state: None,
            event_dialog_date: None,
            event_dialog_recurrence: None,
            event_to_edit: None,
        }
    }
    
    fn apply_theme(ctx: &egui::Context, settings: &Settings) {
        let visuals = if settings.theme.to_lowercase().contains("dark") {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        
        // Customize visuals based on theme settings
        // We'll expand this as we port the theme system
        
        ctx.set_visuals(visuals);
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
                self.event_dialog_recurrence = None;
                self.event_to_edit = None;
            } else if self.show_settings_dialog {
                self.show_settings_dialog = false;
            } else if self.show_theme_picker {
                self.show_theme_picker = false;
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
                    if ui.selectable_label(self.current_view == ViewType::Day, "Day").clicked() {
                        self.current_view = ViewType::Day;
                        ui.close_menu();
                    }
                    if ui.selectable_label(self.current_view == ViewType::Week, "Week").clicked() {
                        self.current_view = ViewType::Week;
                        ui.close_menu();
                    }
                    if ui.selectable_label(self.current_view == ViewType::WorkWeek, "Work Week").clicked() {
                        self.current_view = ViewType::WorkWeek;
                        ui.close_menu();
                    }
                    if ui.selectable_label(self.current_view == ViewType::Month, "Month").clicked() {
                        self.current_view = ViewType::Month;
                        ui.close_menu();
                    }
                    if ui.selectable_label(self.current_view == ViewType::Quarter, "Quarter").clicked() {
                        self.current_view = ViewType::Quarter;
                        ui.close_menu();
                    }
                });
                
                ui.menu_button("Theme", |ui| {
                    if ui.button("Change Theme...").clicked() {
                        self.show_theme_picker = true;
                        ui.close_menu();
                    }
                });
                
                ui.menu_button("Events", |ui| {
                    if ui.button("New Event...").clicked() {
                        self.show_event_dialog = true;
                        self.event_dialog_state = Some(EventDialogState::new_event(
                            self.current_date,
                            &self.settings
                        ));
                        ui.close_menu();
                    }
                });
            });
        });
        
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
                ViewType::Day => self.render_day_view(ui),
                ViewType::Week => self.render_week_view(ui),
                ViewType::WorkWeek => self.render_workweek_view(ui),
                ViewType::Month => self.render_month_view(ui),
                ViewType::Quarter => self.render_quarter_view(ui),
            }
        });
        
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
                        self.event_dialog_state = Some(EventDialogState::from_event(
                            &event,
                            &self.settings
                        ));
                    } else {
                        // Event not found, create new one instead
                        self.event_dialog_state = Some(EventDialogState::new_event(
                            self.event_dialog_date.unwrap_or(self.current_date),
                            &self.settings
                        ));
                    }
                } else {
                    // Creating a new event
                    self.event_dialog_state = Some(EventDialogState::new_event(
                        self.event_dialog_date.unwrap_or(self.current_date),
                        &self.settings
                    ));
                    // Apply any recurrence rule from click
                    if let (Some(ref mut state), Some(ref rrule)) = 
                        (&mut self.event_dialog_state, &self.event_dialog_recurrence) {
                        // Parse and set recurrence from the click handler
                        state.is_recurring = true;
                        if rrule.contains("WEEKLY") {
                            state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Weekly;
                        } else if rrule.contains("MONTHLY") {
                            state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Monthly;
                        } else if rrule.contains("YEARLY") {
                            state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Yearly;
                        } else {
                            state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Daily;
                        }
                    }
                }
            }
            
            self.render_event_dialog(ctx);
        }
        
        if self.show_settings_dialog {
            self.render_settings_dialog(ctx);
        }
        
        if self.show_theme_picker {
            self.render_theme_picker(ctx);
        }
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
    fn render_day_view(&mut self, ui: &mut egui::Ui) {
        DayView::show(
            ui,
            &mut self.current_date,
            self.database,
            &self.settings,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_recurrence,
        );
    }
    
    fn render_week_view(&mut self, ui: &mut egui::Ui) {
        WeekView::show(
            ui,
            &mut self.current_date,
            self.database,
            &self.settings,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_recurrence,
        );
    }
    
    fn render_workweek_view(&mut self, ui: &mut egui::Ui) {
        WorkWeekView::show(
            ui,
            &mut self.current_date,
            self.database,
            &self.settings,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_recurrence,
        );
    }
    
    fn render_month_view(&mut self, ui: &mut egui::Ui) {
        MonthView::show(
            ui,
            &mut self.current_date,
            self.database,
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
            Self::apply_theme(&ctx, &self.settings);
        }
    }
    
    fn render_theme_picker(&mut self, ctx: &egui::Context) {
        egui::Window::new("Theme Picker")
            .collapsible(false)
            .resizable(false)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.label("Theme picker - To be implemented");
                
                if ui.button("Close").clicked() {
                    self.show_theme_picker = false;
                }
            });
    }
}
