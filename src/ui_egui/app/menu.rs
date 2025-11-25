use super::CalendarApp;
use crate::ui_egui::event_dialog::EventDialogState;
use egui::Context;

impl CalendarApp {
    pub(super) fn render_menu_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.render_file_menu(ui, ctx);
                self.render_edit_menu(ui);
                self.render_view_menu(ui);
                self.render_theme_menu(ui);
                self.render_events_menu(ui);
            });
        });
    }

    fn render_file_menu(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        ui.menu_button("File", |ui| {
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

    fn render_edit_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Edit", |ui| {
            if ui.button("Settings    Ctrl+S").clicked() {
                self.show_settings_dialog = true;
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Import Event...").clicked() {
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
        });
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
    }

    fn render_theme_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Theme", |ui| {
            if ui.button("Themes...").clicked() {
                self.state.theme_dialog_state.open(&self.settings.theme);
                ui.close_menu();
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
        });
    }
}
