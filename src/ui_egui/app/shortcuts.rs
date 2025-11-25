use super::CalendarApp;
use super::state::ViewType;

impl CalendarApp {
    pub(super) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
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
                } else if self.state.date_picker_state.is_open {
                    self.state.date_picker_state.close();
                }
            }

            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) && !self.show_event_dialog {
                self.show_event_dialog = true;
                self.event_dialog_date = Some(self.current_date);
                self.event_dialog_time = None;
                self.event_dialog_recurrence = None;
                self.event_to_edit = None;
            }

            if i.modifiers.ctrl && i.key_pressed(egui::Key::T) {
                self.jump_to_today();
            }

            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                self.show_settings_dialog = true;
            }

            if i.modifiers.ctrl && i.key_pressed(egui::Key::B) {
                if let Err(e) = self.state.backup_manager_state.create_backup() {
                    log::error!("Failed to create backup: {}", e);
                }
            }

            // Ctrl+\ to toggle sidebar
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Backslash) {
                self.toggle_sidebar();
            }

            // View type shortcuts (only when no dialog is open)
            if !self.show_event_dialog
                && !self.show_settings_dialog
                && !self.state.theme_dialog_state.is_open
                && !self.state.date_picker_state.is_open
            {
                // D for Day view
                if i.key_pressed(egui::Key::D) && !i.modifiers.ctrl {
                    self.current_view = ViewType::Day;
                    self.focus_on_current_time_if_visible();
                }
                // W for Week view
                if i.key_pressed(egui::Key::W) && !i.modifiers.ctrl {
                    self.current_view = ViewType::Week;
                    self.focus_on_current_time_if_visible();
                }
                // K for Work Week view (W is taken)
                if i.key_pressed(egui::Key::K) && !i.modifiers.ctrl {
                    self.current_view = ViewType::WorkWeek;
                    self.focus_on_current_time_if_visible();
                }
                // M for Month view
                if i.key_pressed(egui::Key::M) && !i.modifiers.ctrl {
                    self.current_view = ViewType::Month;
                }

                // Arrow key navigation
                if i.key_pressed(egui::Key::ArrowLeft) {
                    self.navigate_previous();
                }
                if i.key_pressed(egui::Key::ArrowRight) {
                    self.navigate_next();
                }

                if i.key_pressed(egui::Key::ArrowUp) {
                    self.navigate_up();
                }
                if i.key_pressed(egui::Key::ArrowDown) {
                    self.navigate_down();
                }
            }
        });
    }
}
