//! Context menu rendering for month view day cells.
//!
//! Extracts the popup context menu logic from `render_day_cell` to reduce
//! the size of the main `month_view` module.

use chrono::{Local, NaiveDate};
use egui::{Color32, Pos2, Rect, Vec2};
use std::collections::HashSet;

use super::week_shared::DeleteConfirmRequest;
use super::{countdown_menu_state, is_synced_event, CountdownMenuState, CountdownRequest};
use crate::models::event::Event;
use crate::models::template::EventTemplate;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::services::template::TemplateService;

use super::month_view::MonthViewAction;

/// Result of context menu interactions within a month day cell.
pub struct MonthContextMenuResult {
    /// Delete confirmation request, if triggered
    pub delete_confirm_request: Option<DeleteConfirmRequest>,
    /// Template-based event creation, if triggered
    pub template_action: Option<MonthViewAction>,
}

/// Build a `CountdownRequest` for a month event, resolving the canonical event
/// from the database for non-recurring events.
fn countdown_request_for_month_event(
    event: &Event,
    database: &'static Database,
) -> CountdownRequest {
    let canonical_event = event
        .id
        .and_then(|event_id| {
            let has_recurrence = event
                .recurrence_rule
                .as_deref()
                .map(str::trim)
                .is_some_and(|rule| !rule.is_empty() && rule != "None");

            if has_recurrence {
                None
            } else {
                EventService::new(database.connection())
                    .get(event_id)
                    .ok()
                    .flatten()
            }
        })
        .unwrap_or_else(|| event.clone());

    CountdownRequest::from_event(&canonical_event)
}

/// Render the context menu popup for a month-view day cell.
///
/// Handles both event-specific menus (edit, delete, countdown) and
/// empty-cell menus (new event, recurring event, templates).
///
/// Returns a `MonthContextMenuResult` containing any pending delete or
/// template actions that need to bubble up to the caller.
#[allow(clippy::too_many_arguments)]
pub fn render_cell_context_menu(
    ui: &mut egui::Ui,
    response: &egui::Response,
    rect: Rect,
    date: NaiveDate,
    events: &[&Event],
    pointer_event: &Option<Event>,
    single_event_fallback: &Option<Event>,
    pointer_hit: &Option<(Rect, Event)>,
    synced_event_ids: &HashSet<i64>,
    countdown_requests: &mut Vec<CountdownRequest>,
    active_countdown_events: &HashSet<i64>,
    database: &'static Database,
    show_event_dialog: &mut bool,
    event_dialog_date: &mut Option<NaiveDate>,
    event_dialog_recurrence: &mut Option<String>,
    event_to_edit: &mut Option<i64>,
) -> MonthContextMenuResult {
    let popup_id = response.id.with(format!("month_context_menu_{}", date));
    let popup_event_id_key = popup_id.with("context_event_id");
    let mut popup_anchor_response = response.clone();
    popup_anchor_response.rect = Rect::from_min_size(
        Pos2::new(rect.left() + 5.0, rect.top()),
        Vec2::new(200.0, 30.0),
    );

    let mut delete_confirm_request: Option<DeleteConfirmRequest> = None;

    // Check for pending delete request from previous frame
    let pending_delete_id = ui.ctx().memory_mut(|mem| {
        mem.data
            .remove_temp::<(i64, String)>(popup_id.with("pending_delete"))
    });
    if let Some((event_id, event_title)) = pending_delete_id {
        delete_confirm_request = Some(DeleteConfirmRequest {
            event_id,
            event_title,
            occurrence_only: false,
            occurrence_date: None,
        });
    }

    // Check for pending template selection from previous frame
    let pending_template = ui
        .ctx()
        .memory_mut(|mem| mem.data.remove_temp::<i64>(popup_id.with("pending_template")));

    if response.secondary_clicked() {
        if let Some(event_id) = pointer_event
            .as_ref()
            .and_then(|event| event.id)
            .or_else(|| single_event_fallback.as_ref().and_then(|event| event.id))
        {
            ui.ctx().memory_mut(|mem| {
                mem.data.insert_temp(popup_event_id_key, event_id);
            });
        } else {
            ui.ctx().memory_mut(|mem| {
                mem.data.remove_temp::<i64>(popup_event_id_key);
            });
        }

        if let Some((hit_rect, _)) = pointer_hit {
            popup_anchor_response.rect = *hit_rect;
        }

        ui.memory_mut(|mem| mem.open_popup(popup_id));
    }

    // Load templates for context menu
    let templates: Vec<EventTemplate> = TemplateService::new(database.connection())
        .list_all()
        .unwrap_or_default();

    egui::popup::popup_above_or_below_widget(
        ui,
        popup_id,
        &popup_anchor_response,
        egui::AboveOrBelow::Below,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_width(190.0);

            let popup_event_id =
                ui.ctx()
                    .memory(|mem| mem.data.get_temp::<i64>(popup_event_id_key));
            let popup_event = popup_event_id
                .and_then(|selected_id| {
                    events
                        .iter()
                        .find(|event| event.id == Some(selected_id))
                        .map(|event| (*event).clone())
                })
                .or_else(|| pointer_event.clone())
                .or_else(|| single_event_fallback.clone());

            if let Some(event) = popup_event {
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
                    if let Some(id) = event.id {
                        *event_to_edit = Some(id);
                        *show_event_dialog = true;
                        *event_dialog_date = Some(date);
                    }
                    ui.ctx().memory_mut(|mem| {
                        mem.data.remove_temp::<i64>(popup_event_id_key);
                    });
                    ui.memory_mut(|mem| mem.close_popup());
                }

                // Show countdown option prominently for future events
                match countdown_menu_state(&event, active_countdown_events, Local::now()) {
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
                        if ui.button("⏱ Create Countdown").clicked() {
                            countdown_requests.push(countdown_request_for_month_event(
                                &event, database,
                            ));
                            ui.ctx().memory_mut(|mem| {
                                mem.data.remove_temp::<i64>(popup_event_id_key);
                            });
                            ui.memory_mut(|mem| mem.close_popup());
                        }
                        ui.separator();
                    }
                }

                if event_is_synced {
                    ui.add_enabled(false, egui::Button::new("🗑 Delete"));
                } else if ui.button("🗑 Delete").clicked() {
                    if let Some(id) = event.id {
                        // Store delete request in temp memory for next frame
                        ui.ctx().memory_mut(|mem| {
                            mem.data.insert_temp(
                                popup_id.with("pending_delete"),
                                (id, event.title.clone()),
                            );
                            mem.data.remove_temp::<i64>(popup_event_id_key);
                        });
                    }
                    ui.memory_mut(|mem| mem.close_popup());
                }
            } else {
                ui.label("Create event");
                ui.separator();

                if ui.button("📅 New Event").clicked() {
                    *show_event_dialog = true;
                    *event_dialog_date = Some(date);
                    *event_dialog_recurrence = None;
                    ui.memory_mut(|mem| mem.close_popup());
                }

                if ui.button("🔄 New Recurring Event").clicked() {
                    *show_event_dialog = true;
                    *event_dialog_date = Some(date);
                    *event_dialog_recurrence = Some("FREQ=MONTHLY".to_string());
                    ui.memory_mut(|mem| mem.close_popup());
                }

                // Template submenu
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
                                    ui.ctx().memory_mut(|mem| {
                                        mem.data.insert_temp(
                                            popup_id.with("pending_template"),
                                            id,
                                        );
                                    });
                                }
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        }
                    });
                }
            }
        },
    );

    let popup_open = ui.memory(|mem| mem.is_popup_open(popup_id));
    if !popup_open {
        ui.ctx().memory_mut(|mem| {
            mem.data.remove_temp::<i64>(popup_event_id_key);
        });
    }

    // Build template action if one was selected
    let template_action = pending_template
        .map(|template_id| MonthViewAction::CreateFromTemplate(template_id, date));

    MonthContextMenuResult {
        delete_confirm_request,
        template_action,
    }
}
