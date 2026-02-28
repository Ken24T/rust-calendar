//! Calendar sync settings section for the settings dialog.
//!
//! Handles Google Calendar ICS source management: adding, editing,
//! deleting sources and triggering manual sync operations.

use crate::models::calendar_source::{CalendarSource, GOOGLE_ICS_SOURCE_TYPE};
use crate::models::settings::Settings;
use crate::services::calendar_sync::engine::{CalendarSyncEngine, SyncRunResult};
use crate::services::calendar_sync::CalendarSourceService;
use crate::services::database::Database;
use egui::{Color32, RichText};
use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

/// Draft state for editing an existing calendar source inline.
#[derive(Default)]
struct CalendarSourceDraft {
    name: String,
    ics_url: String,
    poll_interval_minutes: i64,
    enabled: bool,
}

/// Mutable state for the calendar sync section of the settings dialog.
#[derive(Default)]
pub struct CalendarSyncState {
    source_drafts: BTreeMap<i64, CalendarSourceDraft>,
    new_source_name: String,
    new_source_url: String,
    new_source_poll_interval: i64,
    source_status_message: Option<String>,
    source_error_message: Option<String>,
    source_sync_in_progress_id: Option<i64>,
    source_sync_result_rx: Option<Receiver<Result<(String, SyncRunResult), String>>>,
}

impl CalendarSyncState {
    pub fn new() -> Self {
        Self {
            new_source_poll_interval: 15,
            ..Self::default()
        }
    }
}

/// Poll for completed sync results from background threads.
///
/// Should be called once per frame, before rendering.
pub fn poll_sync_result(ctx: &egui::Context, state: &mut CalendarSyncState) {
    if let Some(rx) = &state.source_sync_result_rx {
        match rx.try_recv() {
            Ok(Ok((source_name, summary))) => {
                state.source_sync_result_rx = None;
                state.source_sync_in_progress_id = None;
                state.source_error_message = None;
                state.source_status_message = Some(format!(
                    "Sync complete for '{}': +{} ~{} -{}",
                    source_name, summary.created, summary.updated, summary.deleted
                ));
            }
            Ok(Err(err)) => {
                state.source_sync_result_rx = None;
                state.source_sync_in_progress_id = None;
                state.source_status_message = None;
                state.source_error_message = Some(format!("Sync failed: {}", err));
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
            Err(TryRecvError::Disconnected) => {
                state.source_sync_result_rx = None;
                state.source_sync_in_progress_id = None;
                state.source_status_message = None;
                state.source_error_message =
                    Some("Sync worker disconnected unexpectedly".to_string());
            }
        }
    }
}

/// Render the Google Calendar Sync section of the settings dialog.
pub fn render_calendar_sync_section(
    ui: &mut egui::Ui,
    label_width: f32,
    settings: &mut Settings,
    database: &Database,
    state: &mut CalendarSyncState,
) {
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

    if let Some(message) = &state.source_status_message {
        ui.colored_label(Color32::LIGHT_GREEN, message);
    }
    if let Some(message) = &state.source_error_message {
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

    // Add new source form
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
            egui::TextEdit::singleline(&mut state.new_source_name),
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
            egui::TextEdit::singleline(&mut state.new_source_url),
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
            egui::DragValue::new(&mut state.new_source_poll_interval)
                .range(1..=1440)
                .speed(1)
                .suffix(" min"),
        );

        if ui.button("Add Source").clicked() {
            state.source_status_message = None;
            state.source_error_message = None;

            let new_source = CalendarSource {
                id: None,
                name: state.new_source_name.trim().to_string(),
                source_type: GOOGLE_ICS_SOURCE_TYPE.to_string(),
                ics_url: state.new_source_url.trim().to_string(),
                enabled: true,
                poll_interval_minutes: state.new_source_poll_interval,
                last_sync_at: None,
                last_sync_status: None,
                last_error: None,
            };

            match source_service.create(new_source) {
                Ok(created) => {
                    state.new_source_name.clear();
                    state.new_source_url.clear();
                    state.new_source_poll_interval = 15;
                    state.source_status_message =
                        Some(format!("Added source '{}'", created.name));
                }
                Err(err) => {
                    state.source_error_message =
                        Some(format!("Failed to add source: {}", err));
                }
            }
        }
    });

    ui.add_space(8.0);
    ui.label(RichText::new("Configured Sources").strong());

    // Render each configured source
    let mut deleted_source_ids: Vec<i64> = Vec::new();

    for source in &mut sources {
        let Some(source_id) = source.id else {
            continue;
        };

        let draft = state
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
                ui.add_sized(
                    [140.0, 20.0],
                    egui::TextEdit::singleline(&mut draft.name),
                );
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
                    state.source_status_message = None;
                    state.source_error_message = None;

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
                            state.source_status_message =
                                Some(format!("Updated source '{}'", updated.name));
                        }
                        Err(err) => {
                            state.source_error_message =
                                Some(format!("Failed to update source: {}", err));
                        }
                    }
                }

                let sync_in_progress =
                    state.source_sync_in_progress_id == Some(source_id);
                let any_sync_in_progress = state.source_sync_in_progress_id.is_some();
                let sync_button_text = if sync_in_progress {
                    "Syncing..."
                } else {
                    "Sync Now"
                };

                if ui
                    .add_enabled(!any_sync_in_progress, egui::Button::new(sync_button_text))
                    .clicked()
                {
                    state.source_error_message = None;
                    state.source_sync_in_progress_id = Some(source_id);

                    let source_name = draft.name.clone();
                    state.source_status_message =
                        Some(format!("Syncing '{}'...", source_name));

                    let db_path = database.path().to_string();
                    let (tx, rx) = mpsc::channel();
                    state.source_sync_result_rx = Some(rx);

                    thread::spawn(move || {
                        let result = (|| -> Result<(String, SyncRunResult), String> {
                            let db =
                                Database::new(&db_path).map_err(|err| err.to_string())?;
                            let engine = CalendarSyncEngine::new(db.connection())
                                .map_err(|err| err.to_string())?;
                            let summary = engine
                                .sync_source(source_id)
                                .map_err(|err| err.to_string())?;
                            Ok((source_name, summary))
                        })();

                        let _ = tx.send(result);
                    });
                }

                if ui.button("Delete").clicked() {
                    state.source_status_message = None;
                    state.source_error_message = None;

                    match source_service.delete(source_id) {
                        Ok(_) => {
                            deleted_source_ids.push(source_id);
                            state.source_status_message =
                                Some("Source deleted".to_string());
                        }
                        Err(err) => {
                            state.source_error_message =
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
                ui.colored_label(
                    Color32::LIGHT_RED,
                    format!("Last error: {}", last_error),
                );
            }
        });

        ui.add_space(6.0);
    }

    // Clean up drafts for deleted sources
    for source_id in deleted_source_ids {
        state.source_drafts.remove(&source_id);
    }
}
