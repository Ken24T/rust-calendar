use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::settings::SettingsService;
use egui::{Color32, RichText};

const MIN_CARD_DIMENSION: f32 = 20.0;
const MAX_CARD_DIMENSION: f32 = 600.0;

pub struct SettingsDialogResponse {
    pub saved: bool,
    pub show_ribbon_changed: bool,
}

impl SettingsDialogResponse {
    fn new(saved: bool, show_ribbon_changed: bool) -> Self {
        Self {
            saved,
            show_ribbon_changed,
        }
    }
}

/// Render the settings dialog
pub fn render_settings_dialog(
    ctx: &egui::Context,
    settings: &mut Settings,
    database: &Database,
    show_dialog: &mut bool,
) -> SettingsDialogResponse {
    let mut saved = false;
    let mut error_message: Option<String> = None;
    let mut show_ribbon_changed = false;

    let mut dialog_open = *show_dialog;

    egui::Window::new("Settings")
        .open(&mut dialog_open)
        .collapsible(false)
        .resizable(true)
        .default_width(550.0)
        .default_height(640.0)
        .min_height(600.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Display errors if any
                if let Some(ref error) = error_message {
                    ui.colored_label(Color32::RED, RichText::new(error).strong());
                    ui.add_space(8.0);
                }

                // Two-column layout with right-aligned labels and left-aligned values
                let label_width = 180.0;

                // Calendar Settings Section
                ui.heading("Calendar");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("First day of week:");
                        },
                    );
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
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("First day:");
                        },
                    );
                    egui::ComboBox::from_id_source("work_week_start_combo")
                        .selected_text(weekday_name(settings.first_day_of_work_week))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.first_day_of_work_week, 1, "Monday");
                            ui.selectable_value(&mut settings.first_day_of_work_week, 2, "Tuesday");
                            ui.selectable_value(
                                &mut settings.first_day_of_work_week,
                                3,
                                "Wednesday",
                            );
                            ui.selectable_value(
                                &mut settings.first_day_of_work_week,
                                4,
                                "Thursday",
                            );
                            ui.selectable_value(&mut settings.first_day_of_work_week, 5, "Friday");
                        });
                });

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Last day:");
                        },
                    );
                    egui::ComboBox::from_id_source("work_week_end_combo")
                        .selected_text(weekday_name(settings.last_day_of_work_week))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut settings.last_day_of_work_week, 1, "Monday");
                            ui.selectable_value(&mut settings.last_day_of_work_week, 2, "Tuesday");
                            ui.selectable_value(
                                &mut settings.last_day_of_work_week,
                                3,
                                "Wednesday",
                            );
                            ui.selectable_value(&mut settings.last_day_of_work_week, 4, "Thursday");
                            ui.selectable_value(&mut settings.last_day_of_work_week, 5, "Friday");
                        });
                });

                if settings.first_day_of_work_week > settings.last_day_of_work_week {
                    ui.horizontal(|ui| {
                        ui.add_space(label_width);
                        ui.colored_label(
                            Color32::LIGHT_RED,
                            "⚠ First day should be before last day",
                        );
                    });
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Time Settings Section
                ui.heading("Time");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Time format:");
                        },
                    );
                    egui::ComboBox::from_id_source("time_format_combo")
                        .selected_text(&settings.time_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut settings.time_format,
                                "12h".to_string(),
                                "12-hour",
                            );
                            ui.selectable_value(
                                &mut settings.time_format,
                                "24h".to_string(),
                                "24-hour",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Date format:");
                        },
                    );
                    egui::ComboBox::from_id_source("date_format_combo")
                        .selected_text(&settings.date_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut settings.date_format,
                                "DD/MM/YYYY".to_string(),
                                "DD/MM/YYYY",
                            );
                            ui.selectable_value(
                                &mut settings.date_format,
                                "MM/DD/YYYY".to_string(),
                                "MM/DD/YYYY",
                            );
                            ui.selectable_value(
                                &mut settings.date_format,
                                "YYYY-MM-DD".to_string(),
                                "YYYY-MM-DD",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Default Event Duration:");
                        },
                    );
                    egui::ComboBox::from_id_source("default_event_duration_combo")
                        .selected_text(format!("{} minutes", settings.default_event_duration))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut settings.default_event_duration,
                                15,
                                "15 minutes",
                            );
                            ui.selectable_value(
                                &mut settings.default_event_duration,
                                30,
                                "30 minutes",
                            );
                            ui.selectable_value(
                                &mut settings.default_event_duration,
                                45,
                                "45 minutes",
                            );
                            ui.selectable_value(
                                &mut settings.default_event_duration,
                                60,
                                "60 minutes (1 hour)",
                            );
                            ui.selectable_value(
                                &mut settings.default_event_duration,
                                90,
                                "90 minutes (1.5 hours)",
                            );
                            ui.selectable_value(
                                &mut settings.default_event_duration,
                                120,
                                "120 minutes (2 hours)",
                            );
                        });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Default Event Start Time:");
                        },
                    );
                    ui.add_sized(
                        [80.0, 20.0],
                        egui::TextEdit::singleline(&mut settings.default_event_start_time),
                    );
                    ui.label("(HH:MM)");
                });

                if !is_valid_time_format(&settings.default_event_start_time) {
                    ui.horizontal(|ui| {
                        ui.add_space(label_width);
                        ui.colored_label(Color32::LIGHT_RED, "⚠ Invalid time format");
                    });
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // View Settings Section
                ui.heading("View");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Default view:");
                        },
                    );
                    egui::ComboBox::from_id_source("default_view_combo")
                        .selected_text(&settings.current_view)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut settings.current_view,
                                "Day".to_string(),
                                "Day",
                            );
                            ui.selectable_value(
                                &mut settings.current_view,
                                "WorkWeek".to_string(),
                                "Work Week",
                            );
                            ui.selectable_value(
                                &mut settings.current_view,
                                "Week".to_string(),
                                "Week",
                            );
                            ui.selectable_value(
                                &mut settings.current_view,
                                "Month".to_string(),
                                "Month",
                            );
                        });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_space(label_width);
                    ui.checkbox(&mut settings.show_sidebar, "Show Sidebar");
                });

                if settings.show_sidebar {
                    ui.horizontal(|ui| {
                        ui.add_space(label_width + 20.0);
                        ui.checkbox(
                            &mut settings.my_day_position_right,
                            "Position on right side",
                        );
                    });
                }

                ui.horizontal(|ui| {
                    ui.add_space(label_width);
                    ui.checkbox(&mut settings.use_system_theme, "Use system theme")
                        .on_hover_text("Automatically switch between Light and Dark based on your system settings");
                });

                let ribbon_response = ui
                    .horizontal(|ui| {
                        ui.add_space(label_width);
                        ui.checkbox(&mut settings.show_ribbon, "Show ribbon")
                    })
                    .inner;

                if ribbon_response.changed() {
                    show_ribbon_changed = true;
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                ui.heading("Card");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Default card width:");
                        },
                    );
                    ui.add(
                        egui::DragValue::new(&mut settings.default_card_width)
                            .range(MIN_CARD_DIMENSION..=MAX_CARD_DIMENSION)
                            .speed(1.0)
                            .suffix(" px"),
                    );
                });

                if !is_valid_card_dimension(settings.default_card_width) {
                    ui.horizontal(|ui| {
                        ui.add_space(label_width);
                        ui.colored_label(
                            Color32::LIGHT_RED,
                            format!(
                                "⚠ Width must be between {:.0} and {:.0} px",
                                MIN_CARD_DIMENSION, MAX_CARD_DIMENSION
                            ),
                        );
                    });
                }

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Default card height:");
                        },
                    );
                    ui.add(
                        egui::DragValue::new(&mut settings.default_card_height)
                            .range(MIN_CARD_DIMENSION..=MAX_CARD_DIMENSION)
                            .speed(1.0)
                            .suffix(" px"),
                    );
                });

                if !is_valid_card_dimension(settings.default_card_height) {
                    ui.horizontal(|ui| {
                        ui.add_space(label_width);
                        ui.colored_label(
                            Color32::LIGHT_RED,
                            format!(
                                "⚠ Height must be between {:.0} and {:.0} px",
                                MIN_CARD_DIMENSION, MAX_CARD_DIMENSION
                            ),
                        );
                    });
                }

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_space(label_width);
                    ui.checkbox(
                        &mut settings.auto_create_countdown_on_import,
                        "Auto-create countdown cards on ICS import",
                    );
                });

                ui.horizontal(|ui| {
                    ui.add_space(label_width);
                    ui.checkbox(
                        &mut settings.edit_before_import,
                        "Open event dialog when importing/dragging ICS files",
                    );
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("💾 Save").clicked() {
                        // Validate settings before saving
                        if settings.first_day_of_work_week > settings.last_day_of_work_week {
                            error_message =
                                Some("First day of work week must be before last day".to_string());
                        } else if !is_valid_time_format(&settings.default_event_start_time) {
                            error_message =
                                Some("Invalid default start time format (use HH:MM)".to_string());
                        } else if !is_valid_card_dimension(settings.default_card_width) {
                            error_message = Some(format!(
                                "Default card width must be between {:.0} and {:.0} px",
                                MIN_CARD_DIMENSION, MAX_CARD_DIMENSION
                            ));
                        } else if !is_valid_card_dimension(settings.default_card_height) {
                            error_message = Some(format!(
                                "Default card height must be between {:.0} and {:.0} px",
                                MIN_CARD_DIMENSION, MAX_CARD_DIMENSION
                            ));
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

                    if ui
                        .button(RichText::new("↺ Reset to Defaults").color(Color32::LIGHT_BLUE))
                        .clicked()
                    {
                        let previous = settings.show_ribbon;
                        *settings = Settings::default();
                        if settings.show_ribbon != previous {
                            show_ribbon_changed = true;
                        }
                    }
                });
            });
        });

    if !dialog_open {
        *show_dialog = false;
    }

    SettingsDialogResponse::new(saved, show_ribbon_changed)
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

fn is_valid_card_dimension(value: f32) -> bool {
    value.is_finite() && (MIN_CARD_DIMENSION..=MAX_CARD_DIMENSION).contains(&value)
}
