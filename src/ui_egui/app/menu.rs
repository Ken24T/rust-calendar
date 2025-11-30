use super::CalendarApp;
use chrono::Datelike;
use crate::ui_egui::event_dialog::EventDialogState;
use crate::services::pdf::{PdfExportService, service::PdfExportOptions};
use crate::services::event::EventService;
use crate::services::template::TemplateService;
use crate::services::countdown::CountdownDisplayMode;
use egui::Context;

impl CalendarApp {
    pub(super) fn render_menu_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.render_file_menu(ui, ctx);
                self.render_edit_menu(ui);
                self.render_view_menu(ui);
                self.render_events_menu(ui);
                self.render_help_menu(ui);
            });
        });
    }

    fn render_file_menu(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        ui.menu_button("File", |ui| {
            ui.menu_button("📄 Export to PDF", |ui| {
                if ui.button("Export Month View...").clicked() {
                    self.export_month_to_pdf();
                    ui.close_menu();
                }
                if ui.button("Export Week View...").clicked() {
                    self.export_week_to_pdf();
                    ui.close_menu();
                }
                if ui.button("Export All Events...").clicked() {
                    self.export_events_to_pdf();
                    ui.close_menu();
                }
            });
            ui.separator();
            if ui.button("💾 Backup Database...    Ctrl+B").clicked() {
                if let Err(e) = self.state.backup_manager_state.create_backup() {
                    log::error!("Failed to create backup: {}", e);
                }
                ui.close_menu();
            }
            if ui.button("📂 Manage Backups...").clicked() {
                self.state.backup_manager_state.open();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Exit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }

    fn export_month_to_pdf(&self) {
        let date = self.current_date;
        let month_name = date.format("%B_%Y").to_string();
        
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export Month View to PDF")
            .set_file_name(&format!("calendar_{}.pdf", month_name))
            .add_filter("PDF files", &["pdf"])
            .save_file()
        {
            let event_service = EventService::new(self.context.database().connection());
            let options = PdfExportOptions {
                title: format!("Calendar - {}", date.format("%B %Y")),
                ..Default::default()
            };
            
            if let Err(e) = PdfExportService::export_month(
                &event_service,
                date,
                &path,
                &options,
                self.settings.first_day_of_week,
            ) {
                log::error!("Failed to export PDF: {}", e);
            } else {
                log::info!("Successfully exported month view to {:?}", path);
            }
        }
    }

    fn export_week_to_pdf(&self) {
        let date = self.current_date;
        let week_num = date.iso_week().week();
        
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export Week View to PDF")
            .set_file_name(&format!("calendar_week_{}.pdf", week_num))
            .add_filter("PDF files", &["pdf"])
            .save_file()
        {
            let event_service = EventService::new(self.context.database().connection());
            let options = PdfExportOptions {
                title: format!("Calendar - Week {}", week_num),
                ..Default::default()
            };
            
            if let Err(e) = PdfExportService::export_week(
                &event_service,
                date,
                &path,
                &options,
                self.settings.first_day_of_week,
            ) {
                log::error!("Failed to export PDF: {}", e);
            } else {
                log::info!("Successfully exported week view to {:?}", path);
            }
        }
    }

    fn export_events_to_pdf(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export All Events to PDF")
            .set_file_name("calendar_events.pdf")
            .add_filter("PDF files", &["pdf"])
            .save_file()
        {
            let event_service = EventService::new(self.context.database().connection());
            let events = event_service.list_all().unwrap_or_default();
            let options = PdfExportOptions {
                title: "Calendar Events".to_string(),
                ..Default::default()
            };
            
            if let Err(e) = PdfExportService::export_event_list(&events, &path, &options) {
                log::error!("Failed to export PDF: {}", e);
            } else {
                log::info!("Successfully exported {} events to {:?}", events.len(), path);
            }
        }
    }

    fn render_edit_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Edit", |ui| {
            // Undo/Redo section
            let can_undo = self.undo_manager.can_undo();
            let can_redo = self.undo_manager.can_redo();
            
            let undo_label = if let Some(desc) = self.undo_manager.undo_description() {
                format!("↶ Undo {}    Ctrl+Z", desc)
            } else {
                "↶ Undo    Ctrl+Z".to_string()
            };
            
            let redo_label = if let Some(desc) = self.undo_manager.redo_description() {
                format!("↷ Redo {}    Ctrl+Y", desc)
            } else {
                "↷ Redo    Ctrl+Y".to_string()
            };
            
            ui.add_enabled_ui(can_undo, |ui| {
                if ui.button(&undo_label).clicked() {
                    self.perform_undo();
                    ui.close_menu();
                }
            });
            
            ui.add_enabled_ui(can_redo, |ui| {
                if ui.button(&redo_label).clicked() {
                    self.perform_redo();
                    ui.close_menu();
                }
            });
            
            ui.separator();
            
            if ui.button("⚙ Settings    Ctrl+S").clicked() {
                self.show_settings_dialog = true;
                ui.close_menu();
            }
            if ui.button("🎨 Manage Themes...").clicked() {
                self.state.theme_dialog_state.open(&self.settings.theme);
                ui.close_menu();
            }
            if ui.button("📂 Manage Categories...").clicked() {
                self.state.category_manager_state.open();
                ui.close_menu();
            }
        });
    }
    
    /// Perform undo operation
    pub(super) fn perform_undo(&mut self) {
        let event_service = self.context.event_service();
        match self.undo_manager.undo(&event_service) {
            Ok(Some(desc)) => {
                self.toast_manager.info(format!("Undone: {}", desc));
            }
            Ok(None) => {
                // Nothing to undo
            }
            Err(e) => {
                log::error!("Undo failed: {}", e);
                self.toast_manager.error(format!("Undo failed: {}", e));
            }
        }
    }
    
    /// Perform redo operation
    pub(super) fn perform_redo(&mut self) {
        let event_service = self.context.event_service();
        match self.undo_manager.redo(&event_service) {
            Ok(Some(desc)) => {
                self.toast_manager.info(format!("Redone: {}", desc));
            }
            Ok(None) => {
                // Nothing to redo
            }
            Err(e) => {
                log::error!("Redo failed: {}", e);
                self.toast_manager.error(format!("Redo failed: {}", e));
            }
        }
    }

    fn render_view_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("View", |ui| {
            // Sidebar toggle
            let mut show_sidebar = self.settings.show_sidebar;
            if ui
                .checkbox(&mut show_sidebar, "Show Sidebar    Ctrl+\\")
                .clicked()
            {
                self.toggle_sidebar();
                ui.close_menu();
            }

            if ui
                .checkbox(&mut self.show_ribbon, "Show All-Day Events Ribbon")
                .clicked()
            {
                self.settings.show_ribbon = self.show_ribbon;
                let settings_service = self.context.settings_service();
                if let Err(err) = settings_service.update(&self.settings) {
                    log::error!("Failed to update settings: {}", err);
                }
                ui.close_menu();
            }

            ui.separator();

            // Calendar Views submenu
            ui.menu_button("📅 Calendar Views", |ui| {
                if ui
                    .selectable_label(self.current_view == super::state::ViewType::Day, "Day")
                    .clicked()
                {
                    self.current_view = super::state::ViewType::Day;
                    self.focus_on_current_time_if_visible();
                    ui.close_menu();
                }
                if ui
                    .selectable_label(self.current_view == super::state::ViewType::Week, "Week")
                    .clicked()
                {
                    self.current_view = super::state::ViewType::Week;
                    self.focus_on_current_time_if_visible();
                    ui.close_menu();
                }
                if ui
                    .selectable_label(
                        self.current_view == super::state::ViewType::WorkWeek,
                        "Work Week",
                    )
                    .clicked()
                {
                    self.current_view = super::state::ViewType::WorkWeek;
                    self.focus_on_current_time_if_visible();
                    ui.close_menu();
                }
                if ui
                    .selectable_label(self.current_view == super::state::ViewType::Month, "Month")
                    .clicked()
                {
                    self.current_view = super::state::ViewType::Month;
                    ui.close_menu();
                }
            });

            // Themes submenu
            ui.menu_button("🎨 Themes", |ui| {
                let theme_service = self.context.theme_service();
                let available_themes = theme_service.list_themes().unwrap_or_default();
                let current_theme = self.settings.theme.clone();

                for theme_name in &available_themes {
                    let is_selected = theme_name == &current_theme;
                    if ui.selectable_label(is_selected, theme_name).clicked() {
                        if theme_service.get_theme(theme_name).is_ok() {
                            self.settings.theme = theme_name.clone();
                            let settings_service = self.context.settings_service();
                            if let Err(err) = settings_service.update(&self.settings) {
                                log::error!("Failed to update settings: {}", err);
                            }
                            // Flag to apply theme on next frame (we don't have ctx here)
                            self.state.pending_theme_apply = true;
                        }
                        ui.close_menu();
                    }
                }

                ui.separator();

                if ui.button("🎨 Manage Themes...").clicked() {
                    self.state.theme_dialog_state.open(&self.settings.theme);
                    ui.close_menu();
                }
            });

            ui.separator();

            // Category filter submenu
            self.render_category_filter_submenu(ui);

            ui.separator();

            // Countdown Cards submenu
            ui.menu_button("⏱ Countdown Cards", |ui| {
                let current_mode = self.context.countdown_service().display_mode();
                
                if ui
                    .selectable_label(
                        current_mode == CountdownDisplayMode::IndividualWindows,
                        "Individual Windows",
                    )
                    .clicked()
                {
                    self.context
                        .countdown_service_mut()
                        .set_display_mode(CountdownDisplayMode::IndividualWindows);
                    // Reset container state so it re-initializes when switching back
                    self.countdown_ui.reset_container_state();
                    ui.close_menu();
                }
                
                if ui
                    .selectable_label(
                        current_mode == CountdownDisplayMode::Container,
                        "Container (All in one window)",
                    )
                    .clicked()
                {
                    // Reset container state so it re-applies stored geometry
                    self.countdown_ui.reset_container_state();
                    self.context
                        .countdown_service_mut()
                        .set_display_mode(CountdownDisplayMode::Container);
                    ui.close_menu();
                }
                
                ui.separator();
                
                // Reset positions option - helpful when cards get lost on disconnected monitors
                let card_count = self.context.countdown_service().cards().len();
                let reset_label = if card_count > 0 {
                    format!("🔄 Reset Card Positions ({})", card_count)
                } else {
                    "🔄 Reset Card Positions".to_string()
                };
                
                if ui.button(&reset_label)
                    .on_hover_text("Reset all countdown cards and container to default positions on the primary monitor")
                    .clicked()
                {
                    self.context.countdown_service_mut().reset_all_positions();
                    // Reset container UI state so it re-applies the new geometry
                    self.countdown_ui.reset_container_state();
                    self.toast_manager.info("Card positions reset to defaults");
                    ui.close_menu();
                }
            });
        });
    }

    fn render_category_filter_submenu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("📂 Filter by Category", |ui| {
            // Get all categories
            let categories = self.context.category_service().list_all().unwrap_or_default();
            
            // "All Categories" option
            let is_all_selected = self.active_category_filter.is_none();
            if ui.selectable_label(is_all_selected, "All Categories").clicked() {
                self.active_category_filter = None;
                ui.close_menu();
            }
            
            if !categories.is_empty() {
                ui.separator();
                
                for category in &categories {
                    let label = if let Some(icon) = &category.icon {
                        format!("{} {}", icon, category.name)
                    } else {
                        category.name.clone()
                    };
                    
                    let is_selected = self.active_category_filter.as_ref() == Some(&category.name);
                    if ui.selectable_label(is_selected, label).clicked() {
                        self.active_category_filter = Some(category.name.clone());
                        ui.close_menu();
                    }
                }
            }
        });
    }


    fn render_events_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Events", |ui| {
            if ui.button("New Event...    Ctrl+N").clicked() {
                self.show_event_dialog = true;
                self.event_dialog_state = Some(EventDialogState::new_event(
                    self.current_date,
                    &self.settings,
                ));
                ui.close_menu();
            }
            
            // Templates submenu
            self.render_templates_submenu(ui);
            
            ui.separator();
            if ui.button("🔍 Search Events...    Ctrl+F").clicked() {
                self.state.show_search_dialog = true;
                ui.close_menu();
            }
            if ui.button("📥 Import Event...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("iCalendar", &["ics"])
                    .pick_file()
                {
                    match std::fs::read_to_string(&path) {
                        Ok(ics_content) => {
                            use crate::services::icalendar::import;
                            match import::from_str(&ics_content) {
                                Ok(events) => {
                                    self.handle_ics_import(events, "file import dialog");
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
            
            ui.separator();
            
            ui.menu_button("📤 Export Events", |ui| {
                // Show category-specific export if filter is active
                if let Some(category) = &self.active_category_filter {
                    let label = format!("Export '{}' Events...", category);
                    if ui.button(&label).clicked() {
                        self.export_filtered_events_ics();
                        ui.close_menu();
                    }
                    if ui.button("Export All Events...").clicked() {
                        self.export_all_events_ics();
                        ui.close_menu();
                    }
                } else {
                    if ui.button("Export All Events...").clicked() {
                        self.export_all_events_ics();
                        ui.close_menu();
                    }
                }
                if ui.button("Export Date Range...").clicked() {
                    self.state.show_export_range_dialog = true;
                    ui.close_menu();
                }
            });
        });
    }

    fn render_templates_submenu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("📋 Templates", |ui| {
            // Load templates
            let service = TemplateService::new(self.context.database().connection());
            let templates = service.list_all().unwrap_or_default();

            if templates.is_empty() {
                ui.label(egui::RichText::new("No templates").weak().italics());
                ui.separator();
            } else {
                // Show each template as a button
                for template in &templates {
                    let label = format!("{} → {}", template.name, template.title);
                    if ui.button(&label).on_hover_text(format!(
                        "Create event from template: {}\nDuration: {}",
                        template.title,
                        if template.all_day {
                            "All day".to_string()
                        } else {
                            let h = template.duration_minutes / 60;
                            let m = template.duration_minutes % 60;
                            if h > 0 && m > 0 {
                                format!("{}h {}m", h, m)
                            } else if h > 0 {
                                format!("{}h", h)
                            } else {
                                format!("{}m", m)
                            }
                        }
                    )).clicked() {
                        self.create_event_from_template(template);
                        ui.close_menu();
                    }
                }
                ui.separator();
            }

            if ui.button("Manage Templates...").clicked() {
                self.state.template_manager_state.open(self.context.database());
                ui.close_menu();
            }
        });
    }

    /// Create a new event from a template
    fn create_event_from_template(&mut self, template: &crate::models::template::EventTemplate) {
        use chrono::{Duration, NaiveDateTime};
        
        let mut state = EventDialogState::new_event(self.current_date, &self.settings);
        
        // Apply template values
        state.title = template.title.clone();
        state.description = template.description.clone().unwrap_or_default();
        state.location = template.location.clone().unwrap_or_default();
        state.category = template.category.clone().unwrap_or_default();
        state.color = template.color.clone().unwrap_or_else(|| "#3B82F6".to_string());
        state.all_day = template.all_day;
        
        // Calculate end time based on duration
        if !template.all_day {
            let start_dt = NaiveDateTime::new(state.date, state.start_time);
            let end_dt = start_dt + Duration::minutes(template.duration_minutes as i64);
            state.end_time = end_dt.time();
            // If end goes past midnight, adjust end date
            if end_dt.date() > state.date {
                state.end_date = end_dt.date();
            }
        }
        
        self.event_dialog_state = Some(state);
        self.show_event_dialog = true;
    }
    
    /// Create a new event from a template by ID with a specific date and optional time
    /// Used by context menus in calendar views
    pub(super) fn create_event_from_template_with_date(
        &mut self,
        template_id: i64,
        date: chrono::NaiveDate,
        time: Option<chrono::NaiveTime>,
    ) {
        let service = TemplateService::new(self.context.database().connection());
        if let Ok(template) = service.get_by_id(template_id) {
            use chrono::{Duration, NaiveDateTime};
            
            let mut state = EventDialogState::new_event(date, &self.settings);
            
            // Apply template values
            state.title = template.title.clone();
            state.description = template.description.clone().unwrap_or_default();
            state.location = template.location.clone().unwrap_or_default();
            state.category = template.category.clone().unwrap_or_default();
            state.color = template.color.clone().unwrap_or_else(|| "#3B82F6".to_string());
            state.all_day = template.all_day;
            
            // Use the clicked time if provided, otherwise use the default start time
            if let Some(start_time) = time {
                if !template.all_day {
                    state.start_time = start_time;
                }
            }
            
            // Calculate end time based on duration
            if !template.all_day {
                let start_dt = NaiveDateTime::new(state.date, state.start_time);
                let end_dt = start_dt + Duration::minutes(template.duration_minutes as i64);
                state.end_time = end_dt.time();
                // If end goes past midnight, adjust end date
                if end_dt.date() > state.date {
                    state.end_date = end_dt.date();
                }
            }
            
            // Apply recurrence rule from template if present
            // The recurrence rule parsing is done in EventDialogState::from_event
            // For templates, we'll need to parse the RRULE string
            if template.recurrence_rule.is_some() {
                state.is_recurring = true;
                // Template recurrence will be applied when saving - let user customize
            }
            
            self.event_dialog_state = Some(state);
            self.show_event_dialog = true;
        }
    }

    fn render_help_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Help", |ui| {
            if ui.button("ℹ About...").clicked() {
                self.state.show_about_dialog = true;
                ui.close_menu();
            }
        });
    }

    pub(super) fn render_about_dialog(&mut self, ctx: &Context) {
        if !self.state.show_about_dialog {
            return;
        }

        let mut dialog_open = true;
        egui::Window::new("About Rust Calendar")
            .open(&mut dialog_open)
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(300.0);
                ui.set_max_width(400.0);
                
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(15.0, 10.0))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            // App icon/title
                            ui.heading("📅 Rust Calendar");
                            ui.add_space(5.0);
                            
                            // Version
                            ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                            ui.add_space(10.0);
                            
                            ui.separator();
                            ui.add_space(10.0);
                            
                            // Description
                            ui.label(env!("CARGO_PKG_DESCRIPTION"));
                            ui.add_space(10.0);
                            
                            // Author
                            ui.label(format!("Author: {}", env!("CARGO_PKG_AUTHORS")));
                            ui.add_space(5.0);
                            
                            // License
                            ui.label(format!("License: {}", env!("CARGO_PKG_LICENSE")));
                            ui.add_space(10.0);
                            
                            ui.separator();
                            ui.add_space(10.0);
                            
                            // System info
                            ui.label(egui::RichText::new("System Information").strong());
                            ui.add_space(5.0);
                        });
                        
                        // Grid needs to be outside vertical_centered to align properly
                        egui::Grid::new("about_system_info")
                            .num_columns(2)
                            .spacing([20.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Rust Version:");
                                ui.label(env!("CARGO_PKG_RUST_VERSION", "stable"));
                                ui.end_row();
                                
                                ui.label("Target:");
                                ui.label(std::env::consts::ARCH);
                                ui.end_row();
                                
                                ui.label("OS:");
                                ui.label(std::env::consts::OS);
                                ui.end_row();
                                
                                ui.label("GUI Framework:");
                                ui.label("egui/eframe 0.28");
                                ui.end_row();
                            });
                        
                        ui.add_space(15.0);
                        
                        ui.vertical_centered(|ui| {
                            // Repository link
                            ui.hyperlink_to(
                                "🔗 GitHub Repository",
                                env!("CARGO_PKG_REPOSITORY"),
                            );
                        });
                        
                        ui.add_space(5.0);
                    });
            });
        
        if !dialog_open {
            self.state.show_about_dialog = false;
        }
    }

    /// Export all events to an .ics file
    fn export_all_events_ics(&mut self) {
        let event_service = EventService::new(self.context.database().connection());
        let events = match event_service.list_all() {
            Ok(events) => events,
            Err(e) => {
                log::error!("Failed to load events for export: {}", e);
                self.toast_manager.error("Failed to load events");
                return;
            }
        };

        if events.is_empty() {
            self.toast_manager.warning("No events to export");
            return;
        }

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export All Events")
            .set_file_name("calendar_events.ics")
            .add_filter("iCalendar", &["ics"])
            .save_file()
        {
            use crate::services::icalendar::ICalendarService;
            let ics_service = ICalendarService::new();
            
            match ics_service.export_events_to_file(&events, &path) {
                Ok(()) => {
                    log::info!("Exported {} events to {:?}", events.len(), path);
                    self.toast_manager.success(format!("Exported {} events", events.len()));
                }
                Err(e) => {
                    log::error!("Failed to export events: {}", e);
                    self.toast_manager.error("Failed to export events");
                }
            }
        }
    }

    /// Export filtered events (by category) to an .ics file
    fn export_filtered_events_ics(&mut self) {
        let category = match &self.active_category_filter {
            Some(cat) => cat.clone(),
            None => {
                // No filter active, fall back to export all
                self.export_all_events_ics();
                return;
            }
        };

        let event_service = EventService::new(self.context.database().connection());
        let all_events = match event_service.list_all() {
            Ok(events) => events,
            Err(e) => {
                log::error!("Failed to load events for export: {}", e);
                self.toast_manager.error("Failed to load events");
                return;
            }
        };

        // Filter events by category
        let events: Vec<_> = all_events
            .into_iter()
            .filter(|e| e.category.as_deref() == Some(&category))
            .collect();

        if events.is_empty() {
            self.toast_manager.warning(format!("No '{}' events to export", category));
            return;
        }

        let safe_category = category.replace(' ', "_").replace('/', "-");
        if let Some(path) = rfd::FileDialog::new()
            .set_title(&format!("Export '{}' Events", category))
            .set_file_name(&format!("{}_events.ics", safe_category))
            .add_filter("iCalendar", &["ics"])
            .save_file()
        {
            use crate::services::icalendar::ICalendarService;
            let ics_service = ICalendarService::new();
            
            match ics_service.export_events_to_file(&events, &path) {
                Ok(()) => {
                    log::info!("Exported {} '{}' events to {:?}", events.len(), category, path);
                    self.toast_manager.success(format!("Exported {} '{}' events", events.len(), category));
                }
                Err(e) => {
                    log::error!("Failed to export events: {}", e);
                    self.toast_manager.error("Failed to export events");
                }
            }
        }
    }

    /// Export events in a date range to an .ics file
    pub(super) fn export_events_in_range(&mut self, start: chrono::NaiveDate, end: chrono::NaiveDate) {
        use chrono::{Local, NaiveTime, TimeZone};
        
        // Convert NaiveDate to DateTime for the query
        let start_dt = Local.from_local_datetime(
            &start.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        ).unwrap();
        let end_dt = Local.from_local_datetime(
            &end.and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap())
        ).unwrap();
        
        let event_service = EventService::new(self.context.database().connection());
        let events = match event_service.find_by_date_range(start_dt, end_dt) {
            Ok(events) => events,
            Err(e) => {
                log::error!("Failed to load events for export: {}", e);
                self.toast_manager.error("Failed to load events");
                return;
            }
        };

        if events.is_empty() {
            self.toast_manager.warning("No events in selected range");
            return;
        }

        let filename = format!("calendar_{}_{}.ics", 
            start.format("%Y%m%d"), 
            end.format("%Y%m%d")
        );

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Export Events")
            .set_file_name(&filename)
            .add_filter("iCalendar", &["ics"])
            .save_file()
        {
            use crate::services::icalendar::ICalendarService;
            let ics_service = ICalendarService::new();
            
            match ics_service.export_events_to_file(&events, &path) {
                Ok(()) => {
                    log::info!("Exported {} events to {:?}", events.len(), path);
                    self.toast_manager.success(format!("Exported {} events", events.len()));
                }
                Err(e) => {
                    log::error!("Failed to export events: {}", e);
                    self.toast_manager.error("Failed to export events");
                }
            }
        }
    }
}
