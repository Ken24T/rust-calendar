//! Individual time-slot rendering for the day view.
//!
//! Extracted from `day_view.rs` — handles drawing a single 15-minute time slot
//! including event bars, drag/drop, resize handles, context menus, and tooltips.

use chrono::{Local, NaiveDate, NaiveTime};
use egui::{Color32, CursorIcon, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashSet;

use super::day_context_menu;
use super::day_event_rendering;
use super::palette::TimeGridPalette;
use super::week_shared::{maybe_focus_slot, parse_color, EventInteractionResult};
use super::{is_synced_event, AutoFocusRequest, CountdownRequest};
use crate::models::event::Event;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::drag::{DragContext, DragManager, DragView};
use crate::ui_egui::resize::{
    draw_handles, draw_resize_preview, HandleRects, ResizeContext, ResizeHandle, ResizeManager,
    ResizeView,
};

use super::day_view::DayView;

impl DayView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_time_slot(
        ui: &mut egui::Ui,
        date: NaiveDate,
        time: NaiveTime,
        hour: i64,
        slot_end: NaiveTime,
        is_hour_start: bool,
        starting_events: &[&Event],
        continuing_events: &[&Event],
        synced_event_ids: &HashSet<i64>,
        database: &'static Database,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        palette: &TimeGridPalette,
        focus_request: &mut Option<AutoFocusRequest>,
    ) -> EventInteractionResult {
        let mut result = EventInteractionResult::default();

        ui.horizontal(|ui| {
            // Time label with fixed width (only on hour starts)
            ui.allocate_ui_with_layout(
                Vec2::new(50.0, 40.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if is_hour_start {
                        let time_str = format!("{:02}:00", hour);
                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new(time_str)
                                .size(12.0)
                                .color(Color32::GRAY),
                        );
                    }
                },
            );

            // Time slot area
            let desired_size = Vec2::new(ui.available_width(), 40.0);
            let drag_sense = Sense::click_and_drag().union(Sense::hover());
            let (rect, response) = ui.allocate_exact_size(desired_size, drag_sense);

            maybe_focus_slot(ui, rect, date, time, slot_end, focus_request);

            let hour_bg = palette.hour_bg;
            let regular_bg = palette.regular_bg;
            let hour_line_color = palette.hour_line;
            let slot_line_color = palette.slot_line;
            let hover_overlay = palette.hover_overlay;

            // Check if this slot contains the current time (only for today)
            let today = Local::now().date_naive();
            let now = Local::now().time();
            let is_current_time_slot = date == today && now >= time && now < slot_end;

            // Background - highlight current time slot with a subtle tint
            let bg_color = if is_current_time_slot {
                let highlight = Color32::from_rgba_unmultiplied(100, 180, 255, 25);
                let base = if is_hour_start { hour_bg } else { regular_bg };
                Color32::from_rgb(
                    ((base.r() as u16 * 230 + highlight.r() as u16 * 25) / 255) as u8,
                    ((base.g() as u16 * 230 + highlight.g() as u16 * 25) / 255) as u8,
                    ((base.b() as u16 * 230 + highlight.b() as u16 * 25) / 255) as u8,
                )
            } else if is_hour_start {
                hour_bg
            } else {
                regular_bg
            };
            ui.painter().rect_filled(rect, 0.0, bg_color);

            // Horizontal grid line
            let line_color = if is_hour_start {
                hour_line_color
            } else {
                slot_line_color
            };
            ui.painter().line_segment(
                [
                    Pos2::new(rect.left(), rect.top()),
                    Pos2::new(rect.right(), rect.top()),
                ],
                Stroke::new(1.0, line_color),
            );

            // Hover effect with cursor change
            if response.hovered() {
                ui.painter().rect_filled(rect, 0.0, hover_overlay);
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }

            // Track (rect, event, is_starting, is_ending) for handle visibility
            let mut event_hitboxes: Vec<(Rect, Event, bool, bool)> = Vec::new();

            // Draw continuing events first (colored blocks only)
            for event in continuing_events {
                let event_rect = day_event_rendering::render_event_continuation(ui, rect, event);
                let event_end = event.end.time();
                let is_ending = event_end > time && event_end <= slot_end;
                event_hitboxes.push((event_rect, (*event).clone(), false, is_ending));
            }

            // Draw starting events (full details)
            for event in starting_events {
                let event_rect = day_event_rendering::render_event_in_slot(
                    ui,
                    rect,
                    event,
                    is_synced_event(event.id, synced_event_ids),
                );
                let event_end = event.end.time();
                let is_ending = event_end > time && event_end <= slot_end;
                event_hitboxes.push((event_rect, (*event).clone(), true, is_ending));
            }

            // Build resize handle info for each event
            let event_handles: Vec<(Rect, Event, HandleRects)> = event_hitboxes
                .iter()
                .map(|(r, e, is_starting, is_ending)| {
                    (
                        *r,
                        e.clone(),
                        HandleRects::for_timed_event_in_slot(*r, *is_starting, *is_ending),
                    )
                })
                .collect();

            // Check for pointer position
            let pointer_pos = response
                .interact_pointer_pos()
                .or_else(|| ui.input(|i| i.pointer.hover_pos()));
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
            let hovered_handle: Option<(ResizeHandle, Rect, Event)> =
                pointer_pos.and_then(|pos| {
                    event_handles
                        .iter()
                        .rev()
                        .find_map(|(event_rect, event, handles)| {
                            if event.recurrence_rule.is_some() {
                                return None;
                            }
                            if event.end < now {
                                return None;
                            }
                            if is_synced_event(event.id, synced_event_ids) {
                                return None;
                            }
                            handles
                                .hit_test(pos)
                                .map(|h| (h, *event_rect, event.clone()))
                        })
                });

            // Draw resize handles on hovered event (when not dragging/resizing)
            let is_dragging = DragManager::is_active_for_view(ui.ctx(), DragView::Day);
            let is_resizing = ResizeManager::is_active_for_view(ui.ctx(), ResizeView::Day);

            // Draw resize preview silhouette when actively resizing
            if is_resizing {
                if let Some(resize_ctx) =
                    ResizeManager::active_for_view(ui.ctx(), ResizeView::Day)
                {
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

                    draw_resize_preview(
                        ui,
                        &resize_ctx,
                        rect,
                        date,
                        time,
                        slot_end,
                        event_color,
                        55.0,
                    );
                }
            }

            if !is_dragging && !is_resizing {
                if let Some((hit_rect, hovered_event, is_starting, is_ending)) = &pointer_hit {
                    let is_past_event = hovered_event.end < now;
                    if hovered_event.recurrence_rule.is_none()
                        && !is_past_event
                        && !is_synced_event(hovered_event.id, synced_event_ids)
                    {
                        let handles = HandleRects::for_timed_event_in_slot(
                            *hit_rect,
                            *is_starting,
                            *is_ending,
                        );
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

            // Show tooltip when hovering over an event (but not on resize handles)
            if let Some((hit_rect, hovered_event, _, _)) = &pointer_hit {
                if response.hovered()
                    && hit_rect.contains(pointer_pos.unwrap_or_default())
                    && hovered_handle.is_none()
                {
                    let tooltip_text = super::week_shared::format_event_tooltip(
                        hovered_event,
                        is_synced_event(hovered_event.id, synced_event_ids),
                    );
                    response.clone().on_hover_ui_at_pointer(|ui| {
                        ui.label(tooltip_text);
                    });
                }
            }

            // Set cursor for resize handles or pointer for events
            if let Some((handle, _, _)) = &hovered_handle {
                if !is_dragging && !is_resizing {
                    ui.output_mut(|out| out.cursor_icon = handle.cursor_icon());
                }
            } else if pointer_hit.is_some() && !is_dragging && !is_resizing {
                ui.output_mut(|out| out.cursor_icon = CursorIcon::PointingHand);
            }

            let pointer_for_hover = ui
                .ctx()
                .pointer_interact_pos()
                .or_else(|| ui.input(|i| i.pointer.hover_pos()));
            if let Some(pointer) = pointer_for_hover {
                let is_resize_active =
                    ResizeManager::is_active_for_view(ui.ctx(), ResizeView::Day);

                if rect.contains(pointer) {
                    DragManager::update_hover(ui.ctx(), date, time, rect, pointer);

                    if is_resize_active {
                        ResizeManager::update_hover(ui.ctx(), date, time, slot_end, pointer);
                    }
                }

                if DragManager::is_active_for_view(ui.ctx(), DragView::Day) {
                    ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                    ui.ctx().request_repaint();
                }
                if let Some(resize_ctx) =
                    ResizeManager::active_for_view(ui.ctx(), ResizeView::Day)
                {
                    ui.output_mut(|out| out.cursor_icon = resize_ctx.handle.cursor_icon());
                    ui.ctx().request_repaint();
                }
            }

            // Check for global mouse release to complete resize operations
            let primary_released = ui.input(|i| i.pointer.primary_released());
            if primary_released && ResizeManager::is_active_for_view(ui.ctx(), ResizeView::Day) {
                if let Some(resize_ctx) =
                    ResizeManager::finish_for_view(ui.ctx(), ResizeView::Day)
                {
                    if is_synced_event(Some(resize_ctx.event_id), synced_event_ids) {
                        return;
                    }
                    log::info!(
                        "Resize finished: handle={:?}, hovered_time={:?}, original_start={}, original_end={}",
                        resize_ctx.handle,
                        resize_ctx.hovered_time,
                        resize_ctx.original_start,
                        resize_ctx.original_end
                    );
                    if let Some((new_start, new_end)) = resize_ctx.hovered_times() {
                        log::info!("New times: start={}, end={}", new_start, new_end);
                        let event_service = EventService::new(database.connection());
                        if let Ok(Some(event)) = event_service.get(resize_ctx.event_id) {
                            let old_event = event.clone();
                            let mut new_event = event;
                            new_event.start = new_start;
                            new_event.end = new_end;

                            if new_event.validate().is_err() {
                                log::warn!("Resize would create invalid event, ignoring");
                            } else if let Err(err) = event_service.update(&new_event) {
                                log::error!(
                                    "Failed to resize event {}: {}",
                                    resize_ctx.event_id, err
                                );
                            } else {
                                result.undo_requests.push((old_event, new_event.clone()));
                                result.moved_events.push(new_event);
                            }
                        }
                    } else {
                        log::warn!("hovered_times() returned None");
                    }
                }
            }

            if let Some(drag_state) = DragManager::active_for_view(ui.ctx(), DragView::Day) {
                if drag_state.hovered_date == Some(date) && drag_state.hovered_time == Some(time) {
                    let highlight = rect.shrink2(Vec2::new(5.0, 4.0));
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
            day_context_menu::render_slot_context_menu(
                ui,
                &response,
                rect,
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

            // Check drag_started BEFORE clicked to ensure drag detection works
            if response.drag_started() {
                let drag_start_pos = response.interact_pointer_pos();

                let drag_handle: Option<(ResizeHandle, Rect, Event)> =
                    drag_start_pos.and_then(|pos| {
                        event_handles
                            .iter()
                            .rev()
                            .find_map(|(event_rect, event, handles)| {
                                if event.recurrence_rule.is_some() {
                                    return None;
                                }
                                if event.end < now {
                                    return None;
                                }
                                if is_synced_event(event.id, synced_event_ids) {
                                    return None;
                                }
                                handles
                                    .hit_test(pos)
                                    .map(|h| (h, *event_rect, event.clone()))
                            })
                    });

                if let Some((handle, _hit_rect, event)) = drag_handle {
                    if !is_synced_event(event.id, synced_event_ids) {
                        if let Some(resize_context) =
                            ResizeContext::from_event(&event, handle, ResizeView::Day)
                        {
                            ResizeManager::begin(ui.ctx(), resize_context);
                            ui.output_mut(|out| out.cursor_icon = handle.cursor_icon());
                        }
                    }
                } else if let Some((hit_rect, event, _, _)) = pointer_hit.clone() {
                    if event.recurrence_rule.is_none()
                        && !is_synced_event(event.id, synced_event_ids)
                    {
                        if let Some(drag_context) = DragContext::from_event(
                            &event,
                            pointer_pos
                                .map(|pos| pos - hit_rect.min)
                                .unwrap_or(Vec2::ZERO),
                            DragView::Day,
                        ) {
                            DragManager::begin(ui.ctx(), drag_context);
                            ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                        }
                    }
                }
            }

            // Double-click on event opens edit dialog, on empty space creates new event
            if response.double_clicked() {
                if let Some(event) = pointer_event.clone() {
                    if result.event_to_edit.is_none()
                        && !is_synced_event(event.id, synced_event_ids)
                    {
                        result.event_to_edit = Some(event);
                    }
                } else {
                    *show_event_dialog = true;
                    *event_dialog_date = Some(date);
                    *event_dialog_time = Some(time);
                    *event_dialog_recurrence = None;
                }
            }

            if response.dragged() {
                if ResizeManager::is_active_for_view(ui.ctx(), ResizeView::Day) {
                    if let Some(resize_ctx) =
                        ResizeManager::active_for_view(ui.ctx(), ResizeView::Day)
                    {
                        ui.output_mut(|out| out.cursor_icon = resize_ctx.handle.cursor_icon());
                    }
                } else {
                    ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                }
            }

            if response.drag_stopped() {
                if let Some(drag_context) = DragManager::finish_for_view(ui.ctx(), DragView::Day) {
                    if is_synced_event(Some(drag_context.event_id), synced_event_ids) {
                        return;
                    }
                    if let Some(target_start) = drag_context
                        .hovered_start()
                        .or_else(|| date.and_time(time).and_local_timezone(Local).single())
                    {
                        let new_end = target_start + drag_context.duration;
                        let event_service = EventService::new(database.connection());
                        if let Ok(Some(event)) = event_service.get(drag_context.event_id) {
                            let old_event = event.clone();
                            let mut new_event = event;
                            new_event.start = target_start;
                            new_event.end = new_end;

                            if let Err(err) = event_service.update(&new_event) {
                                log::error!(
                                    "Failed to move event {}: {}",
                                    drag_context.event_id, err
                                );
                            } else {
                                result.undo_requests.push((old_event, new_event.clone()));
                                result.moved_events.push(new_event);
                            }
                        }
                    }
                }
            }
        });

        result
    }
}
