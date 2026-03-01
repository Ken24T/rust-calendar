//! Context menu rendering for time grid cells (week/workweek views).
//!
//! Extracts the popup context menu logic from `render_time_cell` to reduce
//! the size of the main `time_grid` module.

use chrono::{Local, NaiveDate, NaiveTime};
use egui::Color32;
use std::collections::HashSet;

use super::week_shared::{DeleteConfirmRequest, EventInteractionResult};
use super::{countdown_menu_state, is_synced_event, CountdownMenuState, CountdownRequest, render_countdown_menu_items};
use crate::models::event::Event;
use crate::models::template::EventTemplate;
use crate::services::database::Database;
use crate::services::template::TemplateService;

/// Render the context menu popup for a time-grid cell.
///
/// Handles both event-specific menus (edit, delete, countdown, export)
/// and empty-slot menus (new event, recurring event, templates).
///
/// Modifies `result` in-place with any edit / delete / template selections.
#[allow(clippy::too_many_arguments)]
pub fn render_time_cell_context_menu(
    ui: &mut egui::Ui,
    response: &egui::Response,
    date: NaiveDate,
    time: NaiveTime,
    pointer_event: &Option<Event>,
    single_event_fallback: &Option<Event>,
    synced_event_ids: &HashSet<i64>,
    countdown_requests: &mut Vec<CountdownRequest>,
    active_countdown_events: &HashSet<i64>,
    database: &'static Database,
    show_event_dialog: &mut bool,
    event_dialog_date: &mut Option<NaiveDate>,
    event_dialog_time: &mut Option<NaiveTime>,
    event_dialog_recurrence: &mut Option<String>,
    result: &mut EventInteractionResult,
) {
    let mut context_clicked_event: Option<Event> = None;
    let mut context_menu_event: Option<Event> = None;
    let popup_id = response
        .id
        .with(format!("context_menu_{}_{:?}", date, time));

    let show_context_menu = response.secondary_clicked();
    if show_context_menu {
        context_menu_event = pointer_event.clone();
        ui.memory_mut(|mem| mem.open_popup(popup_id));
    }

    egui::popup::popup_above_or_below_widget(
        ui,
        popup_id,
        response,
        egui::AboveOrBelow::Below,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(150.0);
            let popup_event = context_menu_event
                .clone()
                .or_else(|| single_event_fallback.clone());

            if let Some(event) = popup_event {
                render_event_menu(
                    ui,
                    &event,
                    synced_event_ids,
                    countdown_requests,
                    active_countdown_events,
                    &mut context_clicked_event,
                    result,
                );
            } else {
                render_empty_slot_menu(
                    ui,
                    date,
                    time,
                    database,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_time,
                    event_dialog_recurrence,
                    result,
                );
            }
        },
    );

    // Copy context menu edit request to result
    if let Some(event) = context_clicked_event {
        result.event_to_edit = Some(event);
    }
}

/// Menu items shown when right-clicking an existing event.
fn render_event_menu(
    ui: &mut egui::Ui,
    event: &Event,
    synced_event_ids: &HashSet<i64>,
    countdown_requests: &mut Vec<CountdownRequest>,
    active_countdown_events: &HashSet<i64>,
    context_clicked_event: &mut Option<Event>,
    result: &mut EventInteractionResult,
) {
    let event_is_synced = is_synced_event(event.id, synced_event_ids);
    ui.label(format!("Event: {}", event.title));
    ui.separator();

    if event_is_synced {
        ui.label(
            egui::RichText::new("🔒 Synced read-only event")
                .italics()
                .size(11.0),
        );
        ui.add_enabled(false, egui::Button::new("✏ Edit"));
    } else if ui.button("✏ Edit").clicked() {
        *context_clicked_event = Some(event.clone());
        ui.memory_mut(|mem| mem.close_popup());
    }

    // Show countdown option prominently for future events
    match countdown_menu_state(event, active_countdown_events, Local::now()) {
        CountdownMenuState::Hidden => {}
        CountdownMenuState::Active => {
            ui.label(
                egui::RichText::new("⏱ Countdown active")
                    .italics()
                    .color(Color32::from_rgb(100, 200, 100))
                    .size(11.0),
            );
            ui.separator();
        }
        CountdownMenuState::Available => {
            render_countdown_menu_items(ui, event, countdown_requests);
            ui.separator();
        }
    }

    render_delete_buttons(ui, event, event_is_synced, result);
    render_export_button(ui, event);
}

/// Delete buttons for the event context menu (handles recurring vs single events).
fn render_delete_buttons(
    ui: &mut egui::Ui,
    event: &Event,
    event_is_synced: bool,
    result: &mut EventInteractionResult,
) {
    if event_is_synced {
        if event.recurrence_rule.is_some() {
            ui.add_enabled(false, egui::Button::new("🗑 Delete This Occurrence"));
            ui.add_enabled(false, egui::Button::new("🗑 Delete All Occurrences"));
        } else {
            ui.add_enabled(false, egui::Button::new("🗑 Delete"));
        }
    } else if event.recurrence_rule.is_some() {
        if ui.button("🗑 Delete This Occurrence").clicked() {
            if let Some(id) = event.id {
                result.delete_confirm_request = Some(DeleteConfirmRequest {
                    event_id: id,
                    event_title: event.title.clone(),
                    occurrence_only: true,
                    occurrence_date: Some(event.start),
                });
            }
            ui.memory_mut(|mem| mem.close_popup());
        }
        if ui.button("🗑 Delete All Occurrences").clicked() {
            if let Some(id) = event.id {
                result.delete_confirm_request = Some(DeleteConfirmRequest {
                    event_id: id,
                    event_title: event.title.clone(),
                    occurrence_only: false,
                    occurrence_date: None,
                });
            }
            ui.memory_mut(|mem| mem.close_popup());
        }
    } else if ui.button("🗑 Delete").clicked() {
        if let Some(id) = event.id {
            result.delete_confirm_request = Some(DeleteConfirmRequest {
                event_id: id,
                event_title: event.title.clone(),
                occurrence_only: false,
                occurrence_date: None,
            });
        }
        ui.memory_mut(|mem| mem.close_popup());
    }
}

/// Export button for the event context menu.
fn render_export_button(ui: &mut egui::Ui, event: &Event) {
    if ui.button("📤 Export this event").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("{}.ics", event.title.replace(' ', "_")))
            .add_filter("iCalendar", &["ics"])
            .save_file()
        {
            use crate::services::icalendar::export;
            match export::single(event) {
                Ok(ics_content) => {
                    if let Err(e) = std::fs::write(&path, ics_content) {
                        log::error!("Failed to write ICS file: {}", e);
                    } else {
                        log::info!("Exported event to {:?}", path);
                    }
                }
                Err(e) => {
                    log::error!("Failed to export event: {}", e);
                }
            }
        }
        ui.memory_mut(|mem| mem.close_popup());
    }
}

/// Menu items shown when right-clicking an empty time slot.
#[allow(clippy::too_many_arguments)]
fn render_empty_slot_menu(
    ui: &mut egui::Ui,
    date: NaiveDate,
    time: NaiveTime,
    database: &'static Database,
    show_event_dialog: &mut bool,
    event_dialog_date: &mut Option<NaiveDate>,
    event_dialog_time: &mut Option<NaiveTime>,
    event_dialog_recurrence: &mut Option<String>,
    result: &mut EventInteractionResult,
) {
    ui.label("Create event");
    ui.separator();

    if ui.button("📅 New Event").clicked() {
        *show_event_dialog = true;
        *event_dialog_date = Some(date);
        *event_dialog_time = Some(time);
        *event_dialog_recurrence = None;
        ui.memory_mut(|mem| mem.close_popup());
    }

    if ui.button("🔄 New Recurring Event").clicked() {
        *show_event_dialog = true;
        *event_dialog_date = Some(date);
        *event_dialog_time = Some(time);
        *event_dialog_recurrence = Some("FREQ=WEEKLY".to_string());
        ui.memory_mut(|mem| mem.close_popup());
    }

    // Template submenu
    let templates: Vec<EventTemplate> = TemplateService::new(database.connection())
        .list_all()
        .unwrap_or_default();

    if !templates.is_empty() {
        ui.separator();
        ui.menu_button("📋 From Template", |ui| {
            for template in &templates {
                let label = template.name.to_string();
                if ui
                    .button(&label)
                    .on_hover_text(format!(
                        "Create '{}' event\nDuration: {}",
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
                    ))
                    .clicked()
                {
                    if let Some(id) = template.id {
                        result.template_selection = Some((id, date, Some(time)));
                    }
                    ui.memory_mut(|mem| mem.close_popup());
                }
            }
        });
    }
}
