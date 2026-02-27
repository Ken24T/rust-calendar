use crate::models::settings::Settings;
use crate::models::{calendar_source::GOOGLE_ICS_SOURCE_TYPE, calendar_source::CalendarSource};
use crate::services::calendar_sync::engine::{CalendarSyncEngine, SyncRunResult};
use crate::services::calendar_sync::CalendarSourceService;
use crate::services::database::Database;
use crate::services::settings::SettingsService;
use egui::{Color32, RichText};
use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

const MIN_CARD_DIMENSION: f32 = 20.0;
const MAX_CARD_DIMENSION: f32 = 600.0;

pub struct SettingsDialogResponse {
    pub saved: bool,
    pub show_ribbon_changed: bool,
}

#[derive(Default)]
struct CalendarSourceDraft {
    name: String,
    ics_url: String,
    poll_interval_minutes: i64,
    enabled: bool,
}

#[derive(Default)]
pub struct SettingsDialogState {
    source_drafts: BTreeMap<i64, CalendarSourceDraft>,
    new_source_name: String,
    new_source_url: String,
    new_source_poll_interval: i64,
    source_status_message: Option<String>,
    source_error_message: Option<String>,
    source_sync_in_progress_id: Option<i64>,
    source_sync_result_rx: Option<Receiver<Result<(String, SyncRunResult), String>>>,
}

impl SettingsDialogState {
    pub fn new() -> Self {
        Self {
            new_source_poll_interval: 15,
            ..Self::default()
        }
    }
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
    dialog_state: &mut SettingsDialogState,
    show_dialog: &mut bool,
) -> SettingsDialogResponse {
    let mut saved = false;
    let mut error_message: Option<String> = None;
    let mut show_ribbon_changed = false;

    if let Some(rx) = &dialog_state.source_sync_result_rx {
        match rx.try_recv() {
            Ok(Ok((source_name, summary))) => {
                dialog_state.source_sync_result_rx = None;
                dialog_state.source_sync_in_progress_id = None;
                dialog_state.source_error_message = None;
                dialog_state.source_status_message = Some(format!(
                    "Sync complete for '{}': +{} ~{} -{}",
                    source_name, summary.created, summary.updated, summary.deleted
                ));
            }
            Ok(Err(err)) => {
                dialog_state.source_sync_result_rx = None;
                dialog_state.source_sync_in_progress_id = None;
                dialog_state.source_status_message = None;
                dialog_state.source_error_message = Some(format!("Sync failed: {}", err));
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
            Err(TryRecvError::Disconnected) => {
                dialog_state.source_sync_result_rx = None;
                dialog_state.source_sync_in_progress_id = None;
                dialog_state.source_status_message = None;
                dialog_state.source_error_message =
                    Some("Sync worker disconnected unexpectedly".to_string());
            }
        }
    }

    let mut dialog_open = *show_dialog;

    egui::Window::new("Settings")
        .open(&mut dialog_open)
        .collapsible(false)
        .resizable(true)
        .default_width(550.0)
        .default_height(720.0)
        .min_height(680.0)
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

                ui.horizontal(|ui| {
                    ui.add_space(label_width);
                    ui.checkbox(&mut settings.show_week_numbers, "Show week numbers")
                        .on_hover_text("Display ISO week numbers on calendar views");
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

                ui.heading("Google Calendar Sync (Read-Only)");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Startup delay:");
                        },
                    );
                    ui.add(
                        egui::DragValue::new(&mut settings.sync_startup_delay_minutes)
                            .range(0..=1440)
                            .speed(1)
                            .suffix(" min"),
                    );
                    ui.label("(applies on app launch)");
                });

                ui.add_space(6.0);

                if let Some(message) = &dialog_state.source_status_message {
                    ui.colored_label(Color32::LIGHT_GREEN, message);
                }
                if let Some(message) = &dialog_state.source_error_message {
                    ui.colored_label(Color32::LIGHT_RED, message);
                }

                let source_service = CalendarSourceService::new(database.connection());
                let mut sources = match source_service.list_all() {
                    Ok(list) => list,
                    Err(err) => {
                        ui.colored_label(
                            Color32::LIGHT_RED,
                            format!("Failed to load calendar sources: {}", err),
                        );
                        Vec::new()
                    }
                };

                ui.label(RichText::new("Add Source").strong());
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Name:");
                        },
                    );
                    ui.add_sized(
                        [180.0, 20.0],
                        egui::TextEdit::singleline(&mut dialog_state.new_source_name),
                    );
                });

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Private ICS URL:");
                        },
                    );
                    ui.add_sized(
                        [360.0, 20.0],
                        egui::TextEdit::singleline(&mut dialog_state.new_source_url),
                    );
                });

                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(label_width, 20.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label("Poll interval:");
                        },
                    );
                    ui.add(
                        egui::DragValue::new(&mut dialog_state.new_source_poll_interval)
                            .range(1..=1440)
                            .speed(1)
                            .suffix(" min"),
                    );

                    if ui.button("Add Source").clicked() {
                        dialog_state.source_status_message = None;
                        dialog_state.source_error_message = None;

                        let new_source = CalendarSource {
                            id: None,
                            name: dialog_state.new_source_name.trim().to_string(),
                            source_type: GOOGLE_ICS_SOURCE_TYPE.to_string(),
                            ics_url: dialog_state.new_source_url.trim().to_string(),
                            enabled: true,
                            poll_interval_minutes: dialog_state.new_source_poll_interval,
                            last_sync_at: None,
                            last_sync_status: None,
                            last_error: None,
                        };

                        match source_service.create(new_source) {
                            Ok(created) => {
                                dialog_state.new_source_name.clear();
                                dialog_state.new_source_url.clear();
                                dialog_state.new_source_poll_interval = 15;
                                dialog_state.source_status_message = Some(format!(
                                    "Added source '{}'",
                                    created.name
                                ));
                            }
                            Err(err) => {
                                dialog_state.source_error_message =
                                    Some(format!("Failed to add source: {}", err));
                            }
                        }
                    }
                });

                ui.add_space(8.0);
                ui.label(RichText::new("Configured Sources").strong());

                let mut deleted_source_ids: Vec<i64> = Vec::new();

                for source in &mut sources {
                    let Some(source_id) = source.id else {
                        continue;
                    };

                    let draft = dialog_state
                        .source_drafts
                        .entry(source_id)
                        .or_insert_with(|| CalendarSourceDraft {
                            name: source.name.clone(),
                            ics_url: source.ics_url.clone(),
                            poll_interval_minutes: source.poll_interval_minutes,
                            enabled: source.enabled,
                        });

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut draft.enabled, "Enabled");
                            ui.label("Name:");
                            ui.add_sized([140.0, 20.0], egui::TextEdit::singleline(&mut draft.name));
                        });

                        ui.horizontal(|ui| {
                            ui.label("ICS URL:");
                            ui.add_sized(
                                [390.0, 20.0],
                                egui::TextEdit::singleline(&mut draft.ics_url),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("Poll:");
                            ui.add(
                                egui::DragValue::new(&mut draft.poll_interval_minutes)
                                    .range(1..=1440)
                                    .speed(1)
                                    .suffix(" min"),
                            );

                            if ui.button("Update").clicked() {
                                dialog_state.source_status_message = None;
                                dialog_state.source_error_message = None;

                                let updated = CalendarSource {
                                    id: Some(source_id),
                                    name: draft.name.trim().to_string(),
                                    source_type: GOOGLE_ICS_SOURCE_TYPE.to_string(),
                                    ics_url: draft.ics_url.trim().to_string(),
                                    enabled: draft.enabled,
                                    poll_interval_minutes: draft.poll_interval_minutes,
                                    last_sync_at: source.last_sync_at.clone(),
                                    last_sync_status: source.last_sync_status.clone(),
                                    last_error: source.last_error.clone(),
                                };

                                match source_service.update(&updated) {
                                    Ok(_) => {
                                        dialog_state.source_status_message = Some(format!(
                                            "Updated source '{}'",
                                            updated.name
                                        ));
                                    }
                                    Err(err) => {
                                        dialog_state.source_error_message =
                                            Some(format!("Failed to update source: {}", err));
                                    }
                                }
                            }

                            let sync_in_progress =
                                dialog_state.source_sync_in_progress_id == Some(source_id);
                            let any_sync_in_progress = dialog_state.source_sync_in_progress_id.is_some();
                            let sync_button_text = if sync_in_progress {
                                "Syncing..."
                            } else {
                                "Sync Now"
                            };

                            if ui
                                .add_enabled(!any_sync_in_progress, egui::Button::new(sync_button_text))
                                .clicked()
                            {
                                dialog_state.source_error_message = None;
                                dialog_state.source_sync_in_progress_id = Some(source_id);

                                let source_name = draft.name.clone();
                                dialog_state.source_status_message = Some(format!(
                                    "Syncing '{}'...",
                                    source_name
                                ));

                                let db_path = database.path().to_string();
                                let (tx, rx) = mpsc::channel();
                                dialog_state.source_sync_result_rx = Some(rx);

                                thread::spawn(move || {
                                    let result = (|| -> Result<(String, SyncRunResult), String> {
                                        let db = Database::new(&db_path).map_err(|err| err.to_string())?;
                                        let engine =
                                            CalendarSyncEngine::new(db.connection()).map_err(|err| err.to_string())?;
                                        let summary = engine.sync_source(source_id).map_err(|err| err.to_string())?;
                                        Ok((source_name, summary))
                                    })();

                                    let _ = tx.send(result);
                                });
                            }

                            if ui.button("Delete").clicked() {
                                dialog_state.source_status_message = None;
                                dialog_state.source_error_message = None;

                                match source_service.delete(source_id) {
                                    Ok(_) => {
                                        deleted_source_ids.push(source_id);
                                        dialog_state.source_status_message =
                                            Some("Source deleted".to_string());
                                    }
                                    Err(err) => {
                                        dialog_state.source_error_message =
                                            Some(format!("Failed to delete source: {}", err));
                                    }
                                }
                            }
                        });

                        if let Some(status) = &source.last_sync_status {
                            ui.label(format!("Last status: {}", status));
                        }
                        if let Some(last_sync_at) = &source.last_sync_at {
                            ui.label(format!("Last sync: {}", last_sync_at));
                        }
                        if let Some(last_error) = &source.last_error {
                            ui.colored_label(Color32::LIGHT_RED, format!("Last error: {}", last_error));
                        }
                    });

                    ui.add_space(6.0);
                }

                for source_id in deleted_source_ids {
                    dialog_state.source_drafts.remove(&source_id);
                }

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
