use super::CalendarApp;
use crate::ui_egui::event_dialog::EventDialogState;
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
            // --- Import ---
            if ui.button("📥 Import Events...").clicked() {
                self.import_events_ics();
                ui.close_menu();
            }
            if ui.button("📥 Import Countdown Layout...").clicked() {
                self.import_countdown_layout();
                ui.close_menu();
            }

            ui.separator();

            // --- Export ---
            ui.menu_button("📤 Export Events", |ui| {
                if let Some(category) = &self.active_category_filter.clone() {
                    let label = format!("Export '{}' Events...", category);
                    if ui.button(&label).clicked() {
                        self.export_filtered_events_ics();
                        ui.close_menu();
                    }
                    if ui.button("Export All Events...").clicked() {
                        self.export_all_events_ics();
                        ui.close_menu();
                    }
                } else if ui.button("Export All Events...").clicked() {
                    self.export_all_events_ics();
                    ui.close_menu();
                }
                if ui.button("Export Date Range...").clicked() {
                    self.state.show_export_range_dialog = true;
                    ui.close_menu();
                }
            });

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

            if ui.button("📤 Export Countdown Layout...").clicked() {
                self.export_countdown_layout();
                ui.close_menu();
            }

            ui.separator();

            // --- Backups ---
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
                self.exit_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
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
            
            ui.separator();
            
            if ui.button("🎨 Manage Themes...").clicked() {
                self.state.theme_dialog_state.open(&self.settings.theme);
                ui.close_menu();
            }
            if ui.button("📂 Manage Categories...").clicked() {
                self.state.category_manager_state.open();
                ui.close_menu();
            }
            if ui.button("📦 Manage Containers...").clicked() {
                self.state.countdown_category_manager_state.open();
                ui.close_menu();
            }
            if ui.button("🎨 Manage Card Templates...").clicked() {
                self.state.countdown_template_manager_state.open();
                ui.close_menu();
            }
            if ui.button("📋 Manage Event Templates...").clicked() {
                self.state.template_manager_state.open(self.context.database());
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
                
                if ui
                    .selectable_label(
                        current_mode == CountdownDisplayMode::CategoryContainers,
                        "Category Containers",
                    )
                    .clicked()
                {
                    self.countdown_ui.reset_container_state();
                    self.context
                        .countdown_service_mut()
                        .set_display_mode(CountdownDisplayMode::CategoryContainers);
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
                    // Full reset of all UI state to prevent flashing from stale state
                    self.countdown_ui.reset_all_ui_state();
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
        });
    }

    fn render_templates_submenu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("📋 Templates", |ui| {
            // Load templates
            let service = TemplateService::new(self.context.database().connection());
            let templates = service.list_all().unwrap_or_default();

            if templates.is_empty() {
                ui.label(egui::RichText::new("No templates yet").weak().italics());
                ui.label(egui::RichText::new("Use Edit > Manage Event Templates").weak().small());
            } else {
                // Show each template as a button
                for template in &templates {
                    let label = format!("{} > {}", template.name, template.title);
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
}
