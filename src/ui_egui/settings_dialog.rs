use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::settings::SettingsService;
use egui::{Color32, RichText};

/// Render the settings dialog
pub fn render_settings_dialog(
    ctx: &egui::Context,
    settings: &mut Settings,
    database: &Database,
    show_dialog: &mut bool,
) -> bool {
    let mut saved = false;
    let mut error_message: Option<String> = None;
    
    egui::Window::new("Settings")
        .collapsible(false)
        .resizable(true)
        .default_width(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Display errors if any
                if let Some(ref error) = error_message {
                    ui.colored_label(Color32::RED, RichText::new(error).strong());
                    ui.add_space(8.0);
                }
                
                // Theme Section
                ui.heading("Appearance");
                ui.add_space(4.0);
                
                ui.horizontal(|ui| {
                    ui.label("Theme:");
                    egui::ComboBox::from_id_source("theme_combo")
                        .selected_text(&settings.theme)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.theme, "light".to_string(), "Light");
                            ui.selectable_value(&mut settings.theme, "dark".to_string(), "Dark");
                        });
                });
                
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                
                // Calendar Settings Section
                ui.heading("Calendar");
                ui.add_space(4.0);
                
                ui.horizontal(|ui| {
                    ui.label("First day of week:");
                    egui::ComboBox::from_id_source("first_day_combo")
                        .selected_text(weekday_name(settings.first_day_of_week))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.first_day_of_week, 0, "Sunday");
                            ui.selectable_value(&mut settings.first_day_of_week, 1, "Monday");
                            ui.selectable_value(&mut settings.first_day_of_week, 2, "Tuesday");
                            ui.selectable_value(&mut settings.first_day_of_week, 3, "Wednesday");
                            ui.selectable_value(&mut settings.first_day_of_week, 4, "Thursday");
                            ui.selectable_value(&mut settings.first_day_of_week, 5, "Friday");
                            ui.selectable_value(&mut settings.first_day_of_week, 6, "Saturday");
                        });
                });
                
                ui.add_space(8.0);
                
                // Work Week Settings
                ui.label(RichText::new("Work Week").strong());
                ui.add_space(4.0);
                
                ui.horizontal(|ui| {
                    ui.label("First day:");
                    egui::ComboBox::from_id_source("work_week_start_combo")
                        .selected_text(weekday_name(settings.first_day_of_work_week))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.first_day_of_work_week, 1, "Monday");
                            ui.selectable_value(&mut settings.first_day_of_work_week, 2, "Tuesday");
                            ui.selectable_value(&mut settings.first_day_of_work_week, 3, "Wednesday");
                            ui.selectable_value(&mut settings.first_day_of_work_week, 4, "Thursday");
                            ui.selectable_value(&mut settings.first_day_of_work_week, 5, "Friday");
                        });
                    
                    ui.add_space(10.0);
                    
                    ui.label("Last day:");
                    egui::ComboBox::from_id_source("work_week_end_combo")
                        .selected_text(weekday_name(settings.last_day_of_work_week))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.last_day_of_work_week, 1, "Monday");
                            ui.selectable_value(&mut settings.last_day_of_work_week, 2, "Tuesday");
                            ui.selectable_value(&mut settings.last_day_of_work_week, 3, "Wednesday");
                            ui.selectable_value(&mut settings.last_day_of_work_week, 4, "Thursday");
                            ui.selectable_value(&mut settings.last_day_of_work_week, 5, "Friday");
                        });
                });
                
                if settings.first_day_of_work_week > settings.last_day_of_work_week {
                    ui.colored_label(Color32::LIGHT_RED, "⚠ First day should be before last day");
                }
                
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                
                // Time Settings Section
                ui.heading("Time");
                ui.add_space(4.0);
                
                ui.horizontal(|ui| {
                    ui.label("Time format:");
                    egui::ComboBox::from_id_source("time_format_combo")
                        .selected_text(&settings.time_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.time_format, "12h".to_string(), "12-hour");
                            ui.selectable_value(&mut settings.time_format, "24h".to_string(), "24-hour");
                        });
                });
                
                ui.horizontal(|ui| {
                    ui.label("Date format:");
                    egui::ComboBox::from_id_source("date_format_combo")
                        .selected_text(&settings.date_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.date_format, "MM/DD/YYYY".to_string(), "MM/DD/YYYY");
                            ui.selectable_value(&mut settings.date_format, "DD/MM/YYYY".to_string(), "DD/MM/YYYY");
                            ui.selectable_value(&mut settings.date_format, "YYYY-MM-DD".to_string(), "YYYY-MM-DD");
                        });
                });
                
                ui.horizontal(|ui| {
                    ui.label("Time slot interval:");
                    egui::ComboBox::from_id_source("time_slot_combo")
                        .selected_text(format!("{} minutes", settings.time_slot_interval))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.time_slot_interval, 15, "15 minutes");
                            ui.selectable_value(&mut settings.time_slot_interval, 30, "30 minutes");
                            ui.selectable_value(&mut settings.time_slot_interval, 45, "45 minutes");
                            ui.selectable_value(&mut settings.time_slot_interval, 60, "60 minutes");
                        });
                });
                
                ui.add_space(8.0);
                
                // Default Event Times
                ui.label(RichText::new("Default Event Start Time").strong());
                ui.horizontal(|ui| {
                    ui.label("Start time (HH:MM):");
                    ui.text_edit_singleline(&mut settings.default_event_start_time);
                });
                
                if !is_valid_time_format(&settings.default_event_start_time) {
                    ui.colored_label(Color32::LIGHT_RED, "⚠ Invalid time format (use HH:MM)");
                }
                
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                
                // View Settings Section
                ui.heading("View");
                ui.add_space(4.0);
                
                ui.horizontal(|ui| {
                    ui.label("Default view:");
                    egui::ComboBox::from_id_source("default_view_combo")
                        .selected_text(&settings.current_view)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.current_view, "Day".to_string(), "Day");
                            ui.selectable_value(&mut settings.current_view, "WorkWeek".to_string(), "Work Week");
                            ui.selectable_value(&mut settings.current_view, "Week".to_string(), "Week");
                            ui.selectable_value(&mut settings.current_view, "Month".to_string(), "Month");
                            ui.selectable_value(&mut settings.current_view, "Quarter".to_string(), "Quarter");
                        });
                });
                
                ui.add_space(8.0);
                
                ui.checkbox(&mut settings.show_my_day, "Show My Day panel");
                if settings.show_my_day {
                    ui.indent("my_day_indent", |ui| {
                        ui.checkbox(&mut settings.my_day_position_right, "Position on right side");
                    });
                }
                
                ui.checkbox(&mut settings.show_ribbon, "Show ribbon");
                
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                
                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("💾 Save").clicked() {
                        // Validate settings before saving
                        if settings.first_day_of_work_week > settings.last_day_of_work_week {
                            error_message = Some("First day of work week must be before last day".to_string());
                        } else if !is_valid_time_format(&settings.default_event_start_time) {
                            error_message = Some("Invalid default start time format (use HH:MM)".to_string());
                        } else {
                            // Save settings
                            let service = SettingsService::new(database);
                            match service.update(settings) {
                                Ok(_) => {
                                    *show_dialog = false;
                                    saved = true;
                                }
                                Err(e) => {
                                    error_message = Some(format!("Failed to save settings: {}", e));
                                }
                            }
                        }
                    }
                    
                    if ui.button("✖ Cancel").clicked() {
                        *show_dialog = false;
                    }
                    
                    ui.add_space(20.0);
                    
                    if ui.button(RichText::new("↺ Reset to Defaults").color(Color32::LIGHT_BLUE)).clicked() {
                        *settings = Settings::default();
                    }
                });
            });
        });
    
    saved
}

/// Convert weekday number to name
fn weekday_name(day: u8) -> &'static str {
    match day {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}

/// Validate time format (HH:MM)
fn is_valid_time_format(time_str: &str) -> bool {
    if !time_str.contains(':') {
        return false;
    }
    
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 {
        return false;
    }
    
    if let (Ok(hour), Ok(minute)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
        hour < 24 && minute < 60
    } else {
        false
    }
}
