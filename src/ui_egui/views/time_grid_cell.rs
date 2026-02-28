//! Single time cell rendering for the week/workweek time grid.
//!
//! Contains the `render_time_cell` function and the `TimeCellConfig` struct,
//! handling: background painting, event drawing, drag-and-drop, resize handles,
//! context menus, tooltips, and double-click behaviour for individual 15-minute
//! time slots.

use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use egui::{Color32, CursorIcon, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashSet;

use super::event_rendering::{
    format_event_tooltip, parse_color, render_event_continuation, render_event_in_cell,
};
use super::palette::TimeGridPalette;
use super::time_grid_context_menu::render_time_cell_context_menu;
use super::week_shared::{
    maybe_focus_slot, EventInteractionResult, SLOT_HEIGHT,
};
use super::{
    event_time_segment_for_date, is_synced_event, AutoFocusRequest,
    CountdownRequest,
};
use crate::models::event::Event;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::drag::{DragContext, DragManager, DragView};
use crate::ui_egui::resize::{
    HandleRects, ResizeContext, ResizeHandle, ResizeManager, ResizeView, draw_handles,
    draw_resize_preview,
};

/// Configuration for rendering a time cell, allowing view-specific behavior.
pub struct TimeCellConfig {
    pub drag_view: DragView,
    pub resize_view: ResizeView,
    pub check_weekend: bool,
}

/// Render a single time cell in the grid.
#[allow(clippy::too_many_arguments)]
pub fn render_time_cell(
    ui: &mut egui::Ui,
    col_width: f32,
    date: NaiveDate,
    time: NaiveTime,
    slot_end: NaiveTime,
    is_hour_start: bool,
    starting_events: &[&Event],
    continuing_events: &[&Event],
    database: &'static Database,
    show_event_dialog: &mut bool,
    event_dialog_date: &mut Option<NaiveDate>,
    event_dialog_time: &mut Option<NaiveTime>,
    event_dialog_recurrence: &mut Option<String>,
    countdown_requests: &mut Vec<CountdownRequest>,
    active_countdown_events: &HashSet<i64>,
    synced_event_ids: &HashSet<i64>,
    palette: &TimeGridPalette,
    focus_request: &mut Option<AutoFocusRequest>,
    config: &TimeCellConfig,
) -> EventInteractionResult {
    let mut result = EventInteractionResult::default();
    let today = Local::now().date_naive();
    let is_today = date == today;
    let is_weekend = config.check_weekend
        && (date.weekday().num_days_from_sunday() == 0
            || date.weekday().num_days_from_sunday() == 6);

    let desired_size = Vec2::new(col_width, SLOT_HEIGHT);
    let drag_sense = Sense::click_and_drag().union(Sense::hover());
    let (rect, response) = ui.allocate_exact_size(desired_size, drag_sense);

    maybe_focus_slot(ui, rect, date, time, slot_end, focus_request);

    // Check if this slot contains the current time (only for today)
    let now = Local::now().time();
    let is_current_time_slot = is_today && now >= time && now < slot_end;

    // Background color selection with current time highlight
    let base_bg = if is_today {
        palette.today_bg
    } else if is_weekend {
        palette.weekend_bg
    } else {
        palette.regular_bg
    };
    
    let bg_color = if is_current_time_slot {
        // Blend with a soft highlight color (light blue/cyan tint)
        Color32::from_rgb(
            ((base_bg.r() as u16 * 230 + 100 * 25) / 255) as u8,
            ((base_bg.g() as u16 * 230 + 180 * 25) / 255) as u8,
            ((base_bg.b() as u16 * 230 + 255 * 25) / 255) as u8,
        )
    } else {
        base_bg
    };
    ui.painter().rect_filled(rect, 0.0, bg_color);

    // Horizontal grid line
    let line_color = if is_hour_start {
        palette.hour_line
    } else {
        palette.slot_line
    };
    ui.painter().line_segment(
        [
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.right(), rect.top()),
        ],
        Stroke::new(1.0, line_color),
    );

    // Vertical grid line
    ui.painter().line_segment(
        [
            Pos2::new(rect.right(), rect.top()),
            Pos2::new(rect.right(), rect.bottom()),
        ],
        Stroke::new(1.0, palette.divider),
    );

    // Hover effect with cursor change
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, palette.hover_overlay);
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    // Track (rect, event, is_starting, is_ending) for resize handle visibility
    let mut event_hitboxes: Vec<(Rect, Event, bool, bool)> = Vec::new();
    let slot_start_dt = date.and_time(time);
    let slot_end_dt = date.and_time(slot_end);

    // Draw continuing events first
    for event in continuing_events {
        // Check if event ends in this slot
        let segment_end = event_time_segment_for_date(event, date)
            .map(|(_, end)| end)
            .unwrap_or_else(|| event.end.naive_local());
        let is_ending = segment_end > slot_start_dt && segment_end <= slot_end_dt;
        let continues_to_next_slot = segment_end > slot_end_dt;
        
        let event_rect = render_event_continuation(ui, rect, event, continues_to_next_slot);
        // Continuing events never show top handle (they started earlier)
        event_hitboxes.push((event_rect, (*event).clone(), false, is_ending));
    }

    // Draw starting events
    for event in starting_events {
        let has_countdown = event
            .id
            .map(|id| active_countdown_events.contains(&id))
            .unwrap_or(false);
        let event_is_synced = is_synced_event(event.id, synced_event_ids);
        
        // Check if event ends in this slot
        let segment_end = event_time_segment_for_date(event, date)
            .map(|(_, end)| end)
            .unwrap_or_else(|| event.end.naive_local());
        let is_ending = segment_end > slot_start_dt && segment_end <= slot_end_dt;
        let continues_to_next_slot = segment_end > slot_end_dt;
        
        let event_rect =
            render_event_in_cell(ui, rect, event, has_countdown, event_is_synced, continues_to_next_slot);
        // Starting events always show top handle
        event_hitboxes.push((event_rect, (*event).clone(), true, is_ending));
    }

    // Build resize handle info for each event
    let event_handles: Vec<(Rect, Event, HandleRects)> = event_hitboxes
        .iter()
        .map(|(r, e, is_starting, is_ending)| {
            (*r, e.clone(), HandleRects::for_timed_event_in_slot(*r, *is_starting, *is_ending))
        })
        .collect();

    // Pointer hit detection
    let pointer_pos = response
        .interact_pointer_pos()
        .or_else(|| ui.input(|i| i.pointer.hover_pos()));
    // Include is_starting and is_ending flags for proper handle visibility
    let pointer_hit = pointer_pos.and_then(|pos| {
        event_hitboxes
            .iter()
            .rev()
            .find(|(hit_rect, _, _, _)| hit_rect.contains(pos))
            .map(|(hit_rect, event, is_starting, is_ending)| {
                (*hit_rect, event.clone(), *is_starting, *is_ending)
            })
    });
    let pointer_event = pointer_hit.as_ref().map(|(_, event, _, _)| event.clone());
    let single_event_fallback = if event_hitboxes.len() == 1 {
        Some(event_hitboxes[0].1.clone())
    } else {
        None
    };

    // Check if pointer is on a resize handle
    let now = Local::now();
    let hovered_handle: Option<(ResizeHandle, Rect, Event)> = pointer_pos.and_then(|pos| {
        event_handles
            .iter()
            .rev()
            .find_map(|(event_rect, event, handles)| {
                // Only allow resize for non-recurring events and non-past events
                if event.recurrence_rule.is_some() {
                    return None;
                }
                // Don't allow resize for past events
                if event.end < now {
                    return None;
                }
                if is_synced_event(event.id, synced_event_ids) {
                    return None;
                }
                handles.hit_test(pos).map(|h| (h, *event_rect, event.clone()))
            })
    });

    // Draw resize handles on hovered event (when not dragging/resizing)
    let is_dragging = DragManager::is_active_for_view(ui.ctx(), config.drag_view);
    let is_resizing = ResizeManager::is_active_for_view(ui.ctx(), config.resize_view);
    
    // Draw resize preview silhouette when actively resizing
    if is_resizing {
        if let Some(resize_ctx) = ResizeManager::active_for_view(ui.ctx(), config.resize_view) {
            // Find the event color for the preview
            let event_color = event_hitboxes
                .iter()
                .find(|(_, e, _, _)| e.id == Some(resize_ctx.event_id))
                .map(|(_, e, _, _)| {
                    e.color
                        .as_deref()
                        .and_then(parse_color)
                        .unwrap_or(Color32::from_rgb(100, 150, 200))
                })
                .unwrap_or(Color32::from_rgb(100, 150, 200));
            
            // Draw the preview for this slot
            draw_resize_preview(
                ui,
                &resize_ctx,
                rect,
                date,
                time,
                slot_end,
                event_color,
                4.0, // Smaller left margin for week view cells
            );
        }
    }
    
    if !is_dragging && !is_resizing {
        if let Some((hit_rect, hovered_event, is_starting, is_ending)) = &pointer_hit {
            // Only show handles for non-recurring events and non-past events
            let is_past_event = hovered_event.end < now;
            if hovered_event.recurrence_rule.is_none()
                && !is_past_event
                && !is_synced_event(hovered_event.id, synced_event_ids)
            {
                // Use slot-aware handles: only show handles that are active in this slot
                let handles = HandleRects::for_timed_event_in_slot(*hit_rect, *is_starting, *is_ending);
                let hovered_h = hovered_handle.as_ref().map(|(h, _, _)| *h);
                let event_color = hovered_event
                    .color
                    .as_deref()
                    .and_then(parse_color)
                    .unwrap_or(Color32::from_rgb(100, 150, 200));
                draw_handles(ui, &handles, hovered_h, event_color);
            }
        }
    }

    // Set cursor for resize handles
    if let Some((handle, _, _)) = &hovered_handle {
        if !is_dragging && !is_resizing {
            ui.output_mut(|out| out.cursor_icon = handle.cursor_icon());
        }
    } else if pointer_hit.is_some() && !is_dragging && !is_resizing {
        // Pointer cursor when hovering over an event (indicates it's interactive)
        ui.output_mut(|out| out.cursor_icon = CursorIcon::PointingHand);
    }

    // Show tooltip when hovering over an event (but not on resize handles)
    if let Some((hit_rect, hovered_event, _, _)) = &pointer_hit {
        if response.hovered() 
            && hit_rect.contains(pointer_pos.unwrap_or_default())
            && hovered_handle.is_none()
        {
            let tooltip_text =
                format_event_tooltip(hovered_event, is_synced_event(hovered_event.id, synced_event_ids));
            response.clone().on_hover_ui_at_pointer(|ui| {
                ui.label(tooltip_text);
            });
        }
    }

    // Drag/Resize hover tracking
    let pointer_for_hover = ui
        .ctx()
        .pointer_interact_pos()
        .or_else(|| ui.input(|i| i.pointer.hover_pos()));
    if let Some(pointer) = pointer_for_hover {
        if rect.contains(pointer) {
            DragManager::update_hover(ui.ctx(), date, time, rect, pointer);
            
            // Update resize hover when resizing is active
            if ResizeManager::is_active_for_view(ui.ctx(), config.resize_view) {
                ResizeManager::update_hover(ui.ctx(), date, time, slot_end, pointer);
            }
            
            if DragManager::is_active_for_view(ui.ctx(), config.drag_view) {
                ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                ui.ctx().request_repaint();
            }
            if let Some(resize_ctx) = ResizeManager::active_for_view(ui.ctx(), config.resize_view) {
                ui.output_mut(|out| out.cursor_icon = resize_ctx.handle.cursor_icon());
                ui.ctx().request_repaint();
            }
        }
    }

    // Drag highlight
    if let Some(drag_state) = DragManager::active_for_view(ui.ctx(), config.drag_view) {
        if drag_state.hovered_date == Some(date) && drag_state.hovered_time == Some(time) {
            let highlight = rect.shrink2(Vec2::new(3.0, 2.0));
            ui.painter().rect_filled(
                highlight,
                2.0,
                Color32::from_rgba_unmultiplied(120, 200, 120, 35),
            );
            ui.painter().rect_stroke(
                highlight,
                2.0,
                Stroke::new(1.5, Color32::from_rgb(120, 200, 120)),
            );
        }
    }

    // Context menu
    render_time_cell_context_menu(
        ui,
        &response,
        date,
        time,
        &pointer_event,
        &single_event_fallback,
        synced_event_ids,
        countdown_requests,
        active_countdown_events,
        database,
        show_event_dialog,
        event_dialog_date,
        event_dialog_time,
        event_dialog_recurrence,
        &mut result,
    );

    // Drag/Resize handling
    if response.drag_started() {
        // Use interact_pointer_pos for the drag start position
        let drag_start_pos = response.interact_pointer_pos();
        
        // Recalculate which handle was clicked using the drag start position
        let drag_handle: Option<(ResizeHandle, Rect, Event)> = drag_start_pos.and_then(|pos| {
            event_handles
                .iter()
                .rev()
                .find_map(|(event_rect, event, handles)| {
                    // Don't allow resize for recurring or past events
                    if event.recurrence_rule.is_some() {
                        return None;
                    }
                    if event.end < now {
                        return None;
                    }
                    handles.hit_test(pos).map(|h| (h, *event_rect, event.clone()))
                })
        });
        
        // First check if we're starting a resize operation
        if let Some((handle, _hit_rect, event)) = drag_handle {
            // Start resize instead of drag
            if !is_synced_event(event.id, synced_event_ids) {
                if let Some(resize_context) =
                    ResizeContext::from_event(&event, handle, config.resize_view)
                {
                    ResizeManager::begin(ui.ctx(), resize_context);
                    ui.output_mut(|out| out.cursor_icon = handle.cursor_icon());
                }
            }
        } else if let Some((hit_rect, event, _, _)) = pointer_hit.clone() {
            // Otherwise start a drag operation
            if event.recurrence_rule.is_none() && !is_synced_event(event.id, synced_event_ids) {
                if let Some(drag_context) = DragContext::from_event(
                    &event,
                    pointer_pos
                        .map(|pos| pos - hit_rect.min)
                        .unwrap_or(Vec2::ZERO),
                    config.drag_view,
                ) {
                    DragManager::begin(ui.ctx(), drag_context);
                    ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                }
            }
        }
    }

    // Double-click on event opens edit dialog, on empty space creates new event
    if response.double_clicked() {
        if let Some(event) = pointer_event {
            // Double-click on event - edit it
            if result.event_to_edit.is_none() && !is_synced_event(event.id, synced_event_ids) {
                result.event_to_edit = Some(event);
            }
        } else {
            // Double-click on empty space - create new event at this time
            *show_event_dialog = true;
            *event_dialog_date = Some(date);
            *event_dialog_time = Some(time);
            *event_dialog_recurrence = None;
        }
    }

    if response.dragged() {
        if DragManager::is_active_for_view(ui.ctx(), config.drag_view) {
            ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
        } else if let Some(resize_ctx) = ResizeManager::active_for_view(ui.ctx(), config.resize_view) {
            ui.output_mut(|out| out.cursor_icon = resize_ctx.handle.cursor_icon());
        }
    }

    // Check for global mouse release to complete resize operations
    let primary_released = ui.input(|i| i.pointer.primary_released());
    if primary_released && ResizeManager::is_active_for_view(ui.ctx(), config.resize_view) {
        if let Some(resize_ctx) = ResizeManager::finish_for_view(ui.ctx(), config.resize_view) {
            if is_synced_event(Some(resize_ctx.event_id), synced_event_ids) {
                return result;
            }
            if let Some((new_start, new_end)) = resize_ctx.hovered_times() {
                let event_service = EventService::new(database.connection());
                if let Ok(Some(event)) = event_service.get(resize_ctx.event_id) {
                    // Capture old event for undo before modifying
                    let old_event = event.clone();
                    let mut new_event = event;
                    new_event.start = new_start;
                    new_event.end = new_end;
                    
                    // Validate the new event times
                    if new_event.validate().is_err() {
                        log::warn!("Resize would create invalid event, ignoring");
                    } else if let Err(err) = event_service.update(&new_event) {
                        log::error!(
                            "Failed to resize event {}: {}",
                            resize_ctx.event_id, err
                        );
                    } else {
                        // Track for undo and countdown card sync
                        result.undo_requests.push((old_event, new_event.clone()));
                        result.moved_events.push(new_event);
                    }
                }
            }
        }
    }

    if response.drag_stopped() {
        if let Some(drag_context) = DragManager::finish_for_view(ui.ctx(), config.drag_view) {
            if is_synced_event(Some(drag_context.event_id), synced_event_ids) {
                return result;
            }
            if let Some(target_start) = drag_context
                .hovered_start()
                .or_else(|| date.and_time(time).and_local_timezone(Local).single())
            {
                let new_end = target_start + drag_context.duration;
                let event_service = EventService::new(database.connection());
                if let Ok(Some(event)) = event_service.get(drag_context.event_id) {
                    // Capture old event for undo before modifying
                    let old_event = event.clone();
                    let mut new_event = event;
                    new_event.start = target_start;
                    new_event.end = new_end;
                    
                    if let Err(err) = event_service.update(&new_event) {
                        log::error!("Failed to move event {}: {}", drag_context.event_id, err);
                    } else {
                        // Track for undo and countdown card sync
                        result.undo_requests.push((old_event, new_event.clone()));
                        result.moved_events.push(new_event);
                    }
                }
            }
        }
    }

    result
}
