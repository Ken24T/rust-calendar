//! Shared utilities and rendering logic for week-based views (Week and WorkWeek).
//!
//! This module extracts common code to reduce duplication between week_view.rs and workweek_view.rs.
//!
//! Sibling modules (re-exported for API compatibility):
//! - `event_rendering` — individual event block painting, colours, tooltips
//! - `time_grid` — interactive time-slot grid with cells, drag/drop, resize, context menus

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime};
use egui::{Align, Color32, CursorIcon, Rect, Sense, Vec2};
use std::collections::HashSet;

use super::{
    countdown_menu_state, is_synced_event,
    AutoFocusRequest, CountdownMenuState, CountdownRequest,
};
use crate::models::event::Event;
use crate::services::database::Database;
use crate::ui_egui::resize::{HandleRects, ResizeContext, ResizeManager, ResizeView, draw_handles};

// Re-export items from extracted modules so existing consumers compile unchanged.
pub use super::event_rendering::{format_event_tooltip, parse_color};
pub use super::time_grid::render_time_grid;
pub use super::time_grid_cell::TimeCellConfig;

/// Constants for time grid rendering
pub const SLOT_INTERVAL: i64 = 15;
pub const TIME_LABEL_WIDTH: f32 = 50.0;
pub const COLUMN_SPACING: f32 = 1.0;
pub const SLOT_HEIGHT: f32 = 30.0;

/// Request for delete confirmation (event_id, event_title, is_occurrence_only)
#[derive(Clone)]
pub struct DeleteConfirmRequest {
    pub event_id: i64,
    pub event_title: String,
    /// If true, only delete this occurrence (for recurring events)
    pub occurrence_only: bool,
    /// The occurrence date (needed for occurrence-only deletion)
    pub occurrence_date: Option<chrono::DateTime<chrono::Local>>,
}

/// Result of event interactions in views (context menus, clicks, etc.)
#[derive(Default)]
pub struct EventInteractionResult {
    /// Event that was clicked for editing
    pub event_to_edit: Option<Event>,
    /// IDs of events that were deleted (need countdown card cleanup)
    pub deleted_event_ids: Vec<i64>,
    /// Events that were moved via drag-and-drop (need countdown card sync)
    pub moved_events: Vec<Event>,
    /// Request to show delete confirmation dialog
    pub delete_confirm_request: Option<DeleteConfirmRequest>,
    /// Request to create event from template (template_id, date, optional time)
    pub template_selection: Option<(i64, NaiveDate, Option<NaiveTime>)>,
    /// Undo requests: (old_event, new_event) pairs for drag/resize operations
    pub undo_requests: Vec<(Event, Event)>,
}

impl EventInteractionResult {
    pub fn merge(&mut self, other: EventInteractionResult) {
        if other.event_to_edit.is_some() {
            self.event_to_edit = other.event_to_edit;
        }
        self.deleted_event_ids.extend(other.deleted_event_ids);
        self.moved_events.extend(other.moved_events);
        if other.delete_confirm_request.is_some() {
            self.delete_confirm_request = other.delete_confirm_request;
        }
        if other.template_selection.is_some() {
            self.template_selection = other.template_selection;
        }
        self.undo_requests.extend(other.undo_requests);
    }
}

/// Scroll to focus a specific time slot if it matches the focus request.
pub fn maybe_focus_slot(
    ui: &mut egui::Ui,
    rect: Rect,
    date: NaiveDate,
    slot_start: NaiveTime,
    slot_end: NaiveTime,
    focus_request: &mut Option<AutoFocusRequest>,
) {
    if let Some(target) = focus_request.as_ref() {
        if target.matches_slot(date, slot_start, slot_end) {
            ui.scroll_to_rect(rect.expand2(Vec2::new(0.0, 20.0)), Some(Align::Center));
            *focus_request = None;
        }
    }
}

/// Render an event in the all-day ribbon area.
/// Returns the interaction result and the event rect (for resize handle tracking).
/// 
/// Parameters:
/// - `show_left_handle`: True if this is the first day of the event (can resize start date)
/// - `show_right_handle`: True if this is the last day of the event (can resize end date)
/// - `current_date`: The date of the column being rendered (for resize tracking)
pub fn render_ribbon_event(
    ui: &mut egui::Ui,
    event: &Event,
    countdown_requests: &mut Vec<CountdownRequest>,
    active_countdown_events: &HashSet<i64>,
    _database: &'static Database,
    synced_event_ids: &HashSet<i64>,
) -> EventInteractionResult {
    render_ribbon_event_with_handles(
        ui, event, countdown_requests, active_countdown_events, _database, synced_event_ids,
        true, true, None,
    ).0
}

/// Extended ribbon event renderer with resize handle support.
/// Returns (EventInteractionResult, Option<Rect>) where Rect is the event's bounding box.
#[allow(clippy::too_many_arguments)]
pub fn render_ribbon_event_with_handles(
    ui: &mut egui::Ui,
    event: &Event,
    countdown_requests: &mut Vec<CountdownRequest>,
    active_countdown_events: &HashSet<i64>,
    _database: &'static Database,
    synced_event_ids: &HashSet<i64>,
    show_left_handle: bool,
    show_right_handle: bool,
    _current_date: Option<NaiveDate>,
) -> (EventInteractionResult, Option<Rect>) {
    let mut result = EventInteractionResult::default();
    let is_synced = is_synced_event(event.id, synced_event_ids);
    
    let now = Local::now();
    let is_past = event.end < now;
    
    let base_color = event
        .color
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color32::from_rgb(100, 150, 200));
    
    // Dim past events with stronger dimming for visibility
    let event_color = if is_past {
        Color32::from_rgba_unmultiplied(
            (base_color.r() as f32 * 0.4) as u8,
            (base_color.g() as f32 * 0.4) as u8,
            (base_color.b() as f32 * 0.4) as u8,
            140,
        )
    } else {
        base_color
    };
    
    // Text color for past events
    let text_color = if is_past {
        Color32::from_rgba_unmultiplied(255, 255, 255, 150)
    } else {
        Color32::WHITE
    };

    let available_width = ui.available_width();

    // Use directional rounding to visually distinguish multi-day spans from
    // separate single-day events in adjacent columns.
    //   - show_left_handle=true  → this is the first (or only) day: round left corners
    //   - show_right_handle=true → this is the last (or only) day: round right corners
    //   - Middle days of multi-day events: no rounding (flat sides = continuous bar)
    let corner_radius = 4.0;
    let rounding = egui::Rounding {
        nw: if show_left_handle { corner_radius } else { 0.0 },
        sw: if show_left_handle { corner_radius } else { 0.0 },
        ne: if show_right_handle { corner_radius } else { 0.0 },
        se: if show_right_handle { corner_radius } else { 0.0 },
    };

    // Adjust horizontal inner margin so multi-day ribbons extend to column
    // edges, creating a visually continuous bar across adjacent day columns.
    //   - First day: left padding for text inset, zero right padding (extends to edge)
    //   - Last day: zero left padding (extends from edge), right padding for text inset
    //   - Middle day: zero both sides (full bleed)
    //   - Single day (both handles): normal padding on both sides
    let h_pad = 6.0;
    let left_pad = if show_left_handle { h_pad } else { 0.0 };
    let right_pad = if show_right_handle { h_pad } else { 0.0 };
    let frame_margin = left_pad + right_pad;

    let event_frame = egui::Frame::none()
        .fill(event_color)
        .rounding(rounding)
        .inner_margin(egui::Margin {
            left: left_pad,
            right: right_pad,
            top: 1.0,
            bottom: 1.0,
        });

    let response = event_frame
        .show(ui, |ui| {
            // Account for frame margin so total width fits within available_width
            ui.set_min_width(available_width - frame_margin);
            ui.set_max_width(available_width - frame_margin);
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                // Show countdown indicator if this event has a card
                let has_countdown = event
                    .id
                    .map(|id| active_countdown_events.contains(&id))
                    .unwrap_or(false);
                if has_countdown {
                    ui.label(
                        egui::RichText::new("⏱")
                            .color(text_color)
                            .size(10.0),
                    );
                }
                
                // Show location icon if event has a location
                if event.location.as_ref().map(|l| !l.is_empty()).unwrap_or(false) {
                    ui.label(
                        egui::RichText::new("📍")
                            .color(text_color)
                            .size(10.0),
                    );
                }
                
                let display_title = if is_synced {
                    format!("🔒 {}", event.title)
                } else {
                    event.title.clone()
                };

                let title_width = ui.available_width().max(0.0);
                ui.add_sized(
                    Vec2::new(title_width, 12.0),
                    egui::Label::new(egui::RichText::new(display_title).color(text_color).size(11.0))
                        .truncate(),
                );

            });
        })
        .response;

    // Add tooltip with days-until info for future events
    let now = Local::now();
    // Use click_and_drag sensing to detect resize drag start
    let interactive_response = if event.start > now {
        let days_until = (event.start.date_naive() - now.date_naive()).num_days();
        let tooltip = if days_until == 0 {
            format!("{}\nToday", event.title)
        } else if days_until == 1 {
            format!("{}\nTomorrow", event.title)
        } else {
            format!("{}\n{} days from now", event.title, days_until)
        };
        let tooltip = if is_synced {
            format!("{}\n🔒 Synced read-only event", tooltip)
        } else {
            tooltip
        };
        response.interact(Sense::click_and_drag()).on_hover_text(tooltip)
    } else {
        let response = response.interact(Sense::click_and_drag());
        if is_synced {
            response.on_hover_text("🔒 Synced read-only event")
        } else {
            response
        }
    };

    interactive_response.context_menu(|ui| {
        ui.set_min_width(150.0);
        ui.label(format!("Event: {}", event.title));
        ui.separator();

        if is_synced {
            ui.label(
                egui::RichText::new("🔒 Synced read-only event")
                    .italics()
                    .size(11.0),
            );
            ui.add_enabled(false, egui::Button::new("✏ Edit"));
        } else if ui.button("✏ Edit").clicked() {
            result.event_to_edit = Some(event.clone());
            ui.close_menu();
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
                if ui.button("⏱ Create Countdown").clicked() {
                    countdown_requests.push(CountdownRequest::from_event(event));
                    ui.close_menu();
                }
                ui.separator();
            }
        }

        if is_synced {
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
                ui.close_menu();
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
                ui.close_menu();
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
            ui.close_menu();
        }

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
            ui.close_menu();
        }
    });

    // Double-click to edit event
    if interactive_response.double_clicked() && !is_synced {
        result.event_to_edit = Some(event.clone());
    }

    // Get the event rect for resize handle tracking
    let event_rect = interactive_response.rect;
    
    // Draw resize handles if applicable (not past, not recurring, handles enabled)
    let can_resize = !is_synced
        && !is_past
        && event.recurrence_rule.is_none()
        && (show_left_handle || show_right_handle);
    
    let mut on_resize_handle = false;
    if can_resize {
        let handles = HandleRects::for_ribbon_event_in_day(event_rect, show_left_handle, show_right_handle);
        
        // Check for hover on handles
        let pointer_pos = ui.input(|i| i.pointer.hover_pos());
        let hovered_handle = pointer_pos.and_then(|pos| handles.hit_test(pos));
        on_resize_handle = hovered_handle.is_some();
        
        // Draw handles when hovering over the event
        if interactive_response.hovered() || hovered_handle.is_some() {
            let handle_color = event
                .color
                .as_deref()
                .and_then(parse_color)
                .unwrap_or(Color32::from_rgb(100, 150, 200));
            draw_handles(ui, &handles, hovered_handle, handle_color);
        }
        
        // Set cursor for resize handles
        if let Some(handle) = hovered_handle {
            ui.output_mut(|out| out.cursor_icon = handle.cursor_icon());
        }
        
        // Detect drag start on handles to initiate resize
        if interactive_response.drag_started() {
            let drag_start_pos = interactive_response.interact_pointer_pos();
            if let Some(pos) = drag_start_pos {
                if let Some(handle) = handles.hit_test(pos) {
                    // Start resize operation
                    if let Some(resize_context) = ResizeContext::from_event(
                        event,
                        handle,
                        ResizeView::Ribbon,
                    ) {
                        ResizeManager::begin(ui.ctx(), resize_context);
                        ui.output_mut(|out| out.cursor_icon = handle.cursor_icon());
                    }
                }
            }
        }
    }
    
    // Pointer cursor when hovering over ribbon event (not on resize handles)
    if interactive_response.hovered() && !on_resize_handle {
        ui.output_mut(|out| out.cursor_icon = CursorIcon::PointingHand);
    }

    (result, Some(event_rect))
}

// render_event_in_cell, render_event_continuation, format_event_tooltip
// moved to event_rendering.rs

// parse_color moved to event_rendering.rs
// draw_current_time_indicator, TimeCellConfig, render_time_cell, render_time_grid
// moved to time_grid.rs

// NOTE: All of these are re-exported at the top of this file for backwards compatibility.

/// Get the start of the week containing the given date.
pub fn get_week_start(date: NaiveDate, first_day_of_week: u8) -> NaiveDate {
    let weekday = date.weekday().num_days_from_sunday() as i64;
    let offset = (weekday - first_day_of_week as i64 + 7) % 7;
    date - Duration::days(offset)
}

/// Format a date in short form based on the date format setting.
pub fn format_short_date(date: NaiveDate, date_format: &str) -> String {
    if date_format.starts_with("DD/MM") || date_format.starts_with("dd/mm") {
        date.format("%d/%m").to_string()
    } else if date_format.starts_with("YYYY") || date_format.starts_with("yyyy") {
        date.format("%Y/%m/%d").to_string()
    } else {
        date.format("%m/%d").to_string()
    }
}
