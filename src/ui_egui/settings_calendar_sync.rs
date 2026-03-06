//! Calendar sync settings section for the settings dialog.
//!
//! Handles Google Calendar ICS source management: adding, editing,
//! deleting sources and triggering manual sync operations.

use crate::models::calendar_source::SYNC_CAPABILITY_READ_ONLY;
use crate::models::calendar_source::{CalendarSource, GOOGLE_ICS_SOURCE_TYPE};
use crate::models::google_account::GoogleAccount;
use crate::models::settings::Settings;
use crate::services::calendar_sync::engine::{CalendarSyncEngine, SyncRunResult};
use crate::services::calendar_sync::CalendarSourceService;
use crate::services::database::Database;
use crate::services::google_account::GoogleAccountService;
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
    sync_past_days: i64,
    sync_future_days: i64,
    enabled: bool,
}

#[derive(Clone, Copy)]
enum SyncJobKind {
    Preview,
    Apply,
}

#[derive(Clone, Copy)]
enum OAuthJobKind {
    Connect,
    Refresh,
}

type SyncWorkerMessage = Result<(String, SyncJobKind, SyncRunResult), String>;
type OAuthWorkerMessage = Result<(OAuthJobKind, GoogleAccount), String>;

/// Mutable state for the calendar sync section of the settings dialog.
#[derive(Default)]
pub struct CalendarSyncState {
    source_drafts: BTreeMap<i64, CalendarSourceDraft>,
    new_source_name: String,
    new_source_url: String,
    new_source_poll_interval: i64,
    new_source_sync_past_days: i64,
    new_source_sync_future_days: i64,
    source_status_message: Option<String>,
    source_error_message: Option<String>,
    source_sync_in_progress_id: Option<i64>,
    source_sync_result_rx: Option<Receiver<SyncWorkerMessage>>,
    oauth_client_id: String,
    oauth_client_id_loaded: bool,
    oauth_status_message: Option<String>,
    oauth_error_message: Option<String>,
    oauth_job_in_progress: Option<OAuthJobKind>,
    oauth_result_rx: Option<Receiver<OAuthWorkerMessage>>,
}

impl CalendarSyncState {
    pub fn new() -> Self {
        Self {
            new_source_poll_interval: 15,
            new_source_sync_past_days: 90,
            new_source_sync_future_days: 365,
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
            Ok(Ok((source_name, kind, summary))) => {
                state.source_sync_result_rx = None;
                state.source_sync_in_progress_id = None;
                state.source_error_message = None;

                let action = match kind {
                    SyncJobKind::Preview => "Preview",
                    SyncJobKind::Apply => "Sync",
                };

                state.source_status_message = Some(format!(
                    "{} complete for '{}': +{} ~{} -{} ={} skipped:{} errors:{} ({} ms)",
                    action,
                    source_name,
                    summary.created,
                    summary.updated,
                    summary.deleted,
                    summary.unchanged,
                    summary.skipped_missing_uid
                        + summary.skipped_duplicate_uid
                        + summary.skipped_filtered,
                    summary.error_count,
                    summary.duration_ms,
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

    if let Some(rx) = &state.oauth_result_rx {
        match rx.try_recv() {
            Ok(Ok((kind, account))) => {
                state.oauth_result_rx = None;
                state.oauth_job_in_progress = None;
                state.oauth_error_message = None;
                state.oauth_status_message = Some(match kind {
                    OAuthJobKind::Connect => {
                        let email = account
                            .account_email
                            .as_deref()
                            .unwrap_or("unknown account");
                        format!("Connected Google account: {}", email)
                    }
                    OAuthJobKind::Refresh => {
                        let email = account
                            .account_email
                            .as_deref()
                            .unwrap_or("unknown account");
                        format!("Refreshed Google token for {}", email)
                    }
                });
            }
            Ok(Err(err)) => {
                state.oauth_result_rx = None;
                state.oauth_job_in_progress = None;
                state.oauth_status_message = None;
                state.oauth_error_message = Some(err);
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
            Err(TryRecvError::Disconnected) => {
                state.oauth_result_rx = None;
                state.oauth_job_in_progress = None;
                state.oauth_status_message = None;
                state.oauth_error_message =
                    Some("OAuth worker disconnected unexpectedly".to_string());
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
    ui.heading("Google Account (Read/Write Preview)");
    ui.add_space(4.0);

    let account_service = match GoogleAccountService::new(database.connection()) {
        Ok(service) => Some(service),
        Err(err) => {
            state.oauth_error_message = Some(format!(
                "Failed to initialize Google account service: {}",
                err
            ));
            None
        }
    };

    if let Some(service) = &account_service {
        if !state.oauth_client_id_loaded {
            match service.load() {
                Ok(account) => {
                    if state.oauth_client_id.trim().is_empty() {
                        state.oauth_client_id = account.oauth_client_id.unwrap_or_default();
                    }
                }
                Err(err) => {
                    state.oauth_error_message =
                        Some(format!("Failed to load Google account state: {}", err));
                }
            }
            state.oauth_client_id_loaded = true;
        }
    }

    if let Some(message) = &state.oauth_status_message {
        ui.colored_label(Color32::LIGHT_GREEN, message);
    }
    if let Some(message) = &state.oauth_error_message {
        ui.colored_label(Color32::LIGHT_RED, message);
    }

    let account_snapshot = account_service
        .as_ref()
        .and_then(|service| service.load().ok());

    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::Vec2::new(label_width, 20.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label("OAuth client ID:");
            },
        );
        ui.add_sized(
            [360.0, 20.0],
            egui::TextEdit::singleline(&mut state.oauth_client_id)
                .hint_text("Desktop OAuth client ID (*.apps.googleusercontent.com)"),
        );
    });

    let any_oauth_in_progress = state.oauth_job_in_progress.is_some();

    ui.horizontal(|ui| {
        ui.add_space(label_width);

        let connect_label = if matches!(state.oauth_job_in_progress, Some(OAuthJobKind::Connect)) {
            "Connecting..."
        } else {
            "Connect / Reconnect"
        };

        if ui
            .add_enabled(!any_oauth_in_progress, egui::Button::new(connect_label))
            .clicked()
        {
            state.oauth_status_message = Some(
                "Starting Google device login. Complete the browser prompt to finish linking."
                    .to_string(),
            );
            state.oauth_error_message = None;
            state.oauth_job_in_progress = Some(OAuthJobKind::Connect);

            let client_id = state.oauth_client_id.trim().to_string();
            let db_path = database.path().to_string();
            let (tx, rx) = mpsc::channel();
            state.oauth_result_rx = Some(rx);

            thread::spawn(move || {
                let result = (|| -> OAuthWorkerMessage {
                    if client_id.trim().is_empty() {
                        return Err("OAuth client ID cannot be empty".to_string());
                    }
                    let db = Database::new(&db_path).map_err(|err| err.to_string())?;
                    let service = GoogleAccountService::new(db.connection())
                        .map_err(|err| err.to_string())?;
                    let account = service
                        .connect_with_device_flow(&client_id)
                        .map_err(|err| err.to_string())?;
                    Ok((OAuthJobKind::Connect, account))
                })();

                let _ = tx.send(result);
            });
        }

        let refresh_label = if matches!(state.oauth_job_in_progress, Some(OAuthJobKind::Refresh)) {
            "Refreshing..."
        } else {
            "Refresh Token"
        };

        if ui
            .add_enabled(!any_oauth_in_progress, egui::Button::new(refresh_label))
            .clicked()
        {
            state.oauth_status_message = Some("Refreshing Google access token...".to_string());
            state.oauth_error_message = None;
            state.oauth_job_in_progress = Some(OAuthJobKind::Refresh);

            let db_path = database.path().to_string();
            let (tx, rx) = mpsc::channel();
            state.oauth_result_rx = Some(rx);

            thread::spawn(move || {
                let result = (|| -> OAuthWorkerMessage {
                    let db = Database::new(&db_path).map_err(|err| err.to_string())?;
                    let service = GoogleAccountService::new(db.connection())
                        .map_err(|err| err.to_string())?;
                    let account = service
                        .refresh_access_token()
                        .map_err(|err| err.to_string())?;
                    Ok((OAuthJobKind::Refresh, account))
                })();

                let _ = tx.send(result);
            });
        }

        if ui
            .add_enabled(!any_oauth_in_progress, egui::Button::new("Disconnect"))
            .clicked()
        {
            state.oauth_status_message = None;
            state.oauth_error_message = None;

            if let Some(service) = &account_service {
                match service.disconnect() {
                    Ok(_) => {
                        state.oauth_status_message =
                            Some("Disconnected Google account".to_string());
                    }
                    Err(err) => {
                        state.oauth_error_message =
                            Some(format!("Failed to disconnect account: {}", err));
                    }
                }
            }
        }
    });

    if let Some(account) = account_snapshot {
        if account.is_connected() {
            if let Some(email) = account.account_email.as_deref() {
                ui.label(format!("Linked account: {}", email));
            }
            if let Some(expires_at) = account.expires_at.as_deref() {
                ui.label(format!("Token expiry: {}", expires_at));
            }
        } else {
            ui.label("No Google account linked");
        }

        if let Some(last_error) = account.last_error.as_deref() {
            ui.colored_label(
                Color32::LIGHT_RED,
                format!("Last auth error: {}", last_error),
            );
        }
    }

    ui.add_space(12.0);
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
    });

    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::Vec2::new(label_width, 20.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label("Sync window:");
            },
        );
        ui.label("Past");
        ui.add(
            egui::DragValue::new(&mut state.new_source_sync_past_days)
                .range(0..=3650)
                .speed(1)
                .suffix(" d"),
        );
        ui.label("Future");
        ui.add(
            egui::DragValue::new(&mut state.new_source_sync_future_days)
                .range(1..=3650)
                .speed(1)
                .suffix(" d"),
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
                sync_past_days: state.new_source_sync_past_days,
                sync_future_days: state.new_source_sync_future_days,
                sync_capability: SYNC_CAPABILITY_READ_ONLY.to_string(),
                api_sync_token: None,
                last_push_at: None,
                last_sync_at: None,
                last_sync_status: None,
                last_error: None,
            };

            match source_service.create(new_source) {
                Ok(created) => {
                    state.new_source_name.clear();
                    state.new_source_url.clear();
                    state.new_source_poll_interval = 15;
                    state.new_source_sync_past_days = 90;
                    state.new_source_sync_future_days = 365;
                    state.source_status_message = Some(format!("Added source '{}'", created.name));
                }
                Err(err) => {
                    state.source_error_message = Some(format!("Failed to add source: {}", err));
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
                sync_past_days: source.sync_past_days,
                sync_future_days: source.sync_future_days,
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

                ui.label("Past:");
                ui.add(
                    egui::DragValue::new(&mut draft.sync_past_days)
                        .range(0..=3650)
                        .speed(1)
                        .suffix(" d"),
                );

                ui.label("Future:");
                ui.add(
                    egui::DragValue::new(&mut draft.sync_future_days)
                        .range(1..=3650)
                        .speed(1)
                        .suffix(" d"),
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
                        sync_past_days: draft.sync_past_days,
                        sync_future_days: draft.sync_future_days,
                        sync_capability: source.sync_capability.clone(),
                        api_sync_token: source.api_sync_token.clone(),
                        last_push_at: source.last_push_at.clone(),
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

                let sync_in_progress = state.source_sync_in_progress_id == Some(source_id);
                let any_sync_in_progress = state.source_sync_in_progress_id.is_some();
                let sync_button_text = if sync_in_progress {
                    "Running..."
                } else {
                    "Sync Now"
                };

                if ui
                    .add_enabled(!any_sync_in_progress, egui::Button::new("Preview Sync"))
                    .clicked()
                {
                    state.source_error_message = None;
                    state.source_sync_in_progress_id = Some(source_id);

                    let source_name = draft.name.clone();
                    state.source_status_message = Some(format!("Previewing '{}'...", source_name));

                    let db_path = database.path().to_string();
                    let (tx, rx) = mpsc::channel();
                    state.source_sync_result_rx = Some(rx);

                    thread::spawn(move || {
                        let result = (|| -> SyncWorkerMessage {
                            let db = Database::new(&db_path).map_err(|err| err.to_string())?;
                            let engine = CalendarSyncEngine::new(db.connection())
                                .map_err(|err| err.to_string())?;
                            let summary = engine
                                .preview_source(source_id)
                                .map_err(|err| err.to_string())?;
                            Ok((source_name, SyncJobKind::Preview, summary))
                        })();

                        let _ = tx.send(result);
                    });
                }

                if ui
                    .add_enabled(!any_sync_in_progress, egui::Button::new(sync_button_text))
                    .clicked()
                {
                    state.source_error_message = None;
                    state.source_sync_in_progress_id = Some(source_id);

                    let source_name = draft.name.clone();
                    state.source_status_message = Some(format!("Syncing '{}'...", source_name));

                    let db_path = database.path().to_string();
                    let (tx, rx) = mpsc::channel();
                    state.source_sync_result_rx = Some(rx);

                    thread::spawn(move || {
                        let result = (|| -> SyncWorkerMessage {
                            let db = Database::new(&db_path).map_err(|err| err.to_string())?;
                            let engine = CalendarSyncEngine::new(db.connection())
                                .map_err(|err| err.to_string())?;
                            let summary = engine
                                .sync_source(source_id)
                                .map_err(|err| err.to_string())?;
                            Ok((source_name, SyncJobKind::Apply, summary))
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
                            state.source_status_message = Some("Source deleted".to_string());
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
            ui.label(format!("Capability: {}", source.sync_capability));
            if let Some(last_sync_at) = &source.last_sync_at {
                ui.label(format!("Last sync: {}", last_sync_at));
            }
            if let Some(last_push_at) = &source.last_push_at {
                ui.label(format!("Last push: {}", last_push_at));
            }
            if let Some(last_error) = &source.last_error {
                ui.colored_label(Color32::LIGHT_RED, format!("Last error: {}", last_error));
            }

            if let Ok(Some(run)) = source_service.latest_sync_run(source_id) {
                ui.label(format!("Last duration: {} ms", run.duration_ms));
                ui.label(format!(
                    "Last run summary: +{} ~{} -{} ={} skipped:{} errors:{}",
                    run.created_count,
                    run.updated_count,
                    run.deleted_count,
                    run.unchanged_count,
                    run.skipped_count,
                    run.error_count,
                ));
            }
        });

        ui.add_space(6.0);
    }

    // Clean up drafts for deleted sources
    for source_id in deleted_source_ids {
        state.source_drafts.remove(&source_id);
    }
}
