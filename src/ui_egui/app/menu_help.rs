use super::CalendarApp;
use egui::Context;

/// Help menu and About dialog.
impl CalendarApp {
    pub(super) fn render_help_menu(&mut self, ui: &mut egui::Ui) {
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
}
