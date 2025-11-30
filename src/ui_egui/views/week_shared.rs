//! Shared utilities and rendering logic for week-based views (Week and WorkWeek).
//!
//! This module extracts common code to reduce duplication between week_view.rs and workweek_view.rs.

use chrono::{Datelike, Local, NaiveDate, NaiveTime, Timelike};
use egui::{Align, Color32, CursorIcon, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashSet;

use super::palette::TimeGridPalette;
use super::{event_time_segment_for_date, AutoFocusRequest, CountdownRequest};

// Re-export utility functions for backward compatibility
pub use super::utils::{format_event_tooltip, format_short_date, get_week_start, parse_color};
use crate::models::event::Event;
use crate::models::template::EventTemplate;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::services::template::TemplateService;
use crate::ui_egui::drag::{DragContext, DragManager, DragView};
use crate::ui_egui::resize::{HandleRects, ResizeContext, ResizeHandle, ResizeManager, ResizeView, draw_handles, draw_resize_preview};

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
) -> EventInteractionResult {
    render_ribbon_event_with_handles(
        ui, event, countdown_requests, active_countdown_events, _database,
        false, false, None,
    ).0
}

/// Extended ribbon event renderer with resize handle support.
/// Returns (EventInteractionResult, Option<Rect>) where Rect is the event's bounding box.
pub fn render_ribbon_event_with_handles(
    ui: &mut egui::Ui,
    event: &Event,
    countdown_requests: &mut Vec<CountdownRequest>,
    active_countdown_events: &HashSet<i64>,
    _database: &'static Database,
    show_left_handle: bool,
    show_right_handle: bool,
    _current_date: Option<NaiveDate>,
) -> (EventInteractionResult, Option<Rect>) {
    let mut result = EventInteractionResult::default();
    
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

    // Frame has 6.0 horizontal margin on each side (12.0 total)
    let frame_margin = 12.0;
    let event_frame = egui::Frame::none()
        .fill(event_color)
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(6.0, 1.0));

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
                
                ui.label(
                    egui::RichText::new(&event.title)
                        .color(text_color)
                        .size(11.0),
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
        response.interact(Sense::click_and_drag()).on_hover_text(tooltip)
    } else {
        response.interact(Sense::click_and_drag())
    };

    interactive_response.context_menu(|ui| {
        ui.set_min_width(150.0);
        ui.label(format!("Event: {}", event.title));
        ui.separator();

        if ui.button("✏ Edit").clicked() {
            result.event_to_edit = Some(event.clone());
            ui.close_menu();
        }

        // Show countdown option prominently for future events
        if event.start > Local::now() {
            let timer_exists = event
                .id
                .map(|id| active_countdown_events.contains(&id))
                .unwrap_or(false);
            if timer_exists {
                ui.label(
                    egui::RichText::new("⏱ Countdown active")
                        .italics()
                        .color(Color32::from_rgb(100, 200, 100))
                        .size(11.0),
                );
            } else if ui.button("⏱ Create Countdown").clicked() {
                countdown_requests.push(CountdownRequest::from_event(event));
                ui.close_menu();
            }
            ui.separator();
        }

        if event.recurrence_rule.is_some() {
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
                .set_file_name(&format!("{}.ics", event.title.replace(' ', "_")))
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
    if interactive_response.double_clicked() {
        result.event_to_edit = Some(event.clone());
    }

    // Get the event rect for resize handle tracking
    let event_rect = interactive_response.rect;
    
    // Draw resize handles if applicable (not past, not recurring, handles enabled)
    let can_resize = !is_past && event.recurrence_rule.is_none() && (show_left_handle || show_right_handle);
    
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

/// Render an event bar inside a time cell (for events starting in this slot).
/// If `continues_to_next_slot` is true, the bottom edge extends to connect
/// with continuation blocks in subsequent slots.
pub fn render_event_in_cell(
    ui: &mut egui::Ui, 
    cell_rect: Rect, 
    event: &Event,
    has_countdown: bool,
    continues_to_next_slot: bool,
) -> Rect {
    let now = Local::now();
    let is_past = event.end < now;
    
    let base_color = event
        .color
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color32::from_rgb(100, 150, 200));
    
    // Dim past events by reducing both color intensity and alpha
    // Using a stronger dimming factor (0.4) and ensuring consistent opacity
    let event_color = if is_past {
        // Multiply RGB by 0.4 and reduce alpha to 140 for visible dimming
        Color32::from_rgba_unmultiplied(
            (base_color.r() as f32 * 0.4) as u8,
            (base_color.g() as f32 * 0.4) as u8,
            (base_color.b() as f32 * 0.4) as u8,
            140,
        )
    } else {
        base_color
    };

    // If event continues to next slot, extend to bottom of cell (no bottom margin)
    let bottom_margin = if continues_to_next_slot { 0.0 } else { 2.0 };
    let bar_rect = Rect::from_min_size(
        Pos2::new(cell_rect.left() + 1.0, cell_rect.top() + 2.0),
        Vec2::new(cell_rect.width() - 2.0, cell_rect.height() - 2.0 - bottom_margin),
    );
    // Use rounded corners only at top if continuing, full rounding otherwise
    let rounding = if continues_to_next_slot {
        egui::Rounding { nw: 2.0, ne: 2.0, sw: 0.0, se: 0.0 }
    } else {
        egui::Rounding::same(2.0)
    };
    ui.painter().rect_filled(bar_rect, rounding, event_color);

    let font_id = egui::FontId::proportional(10.0);
    let available_width = cell_rect.width() - 10.0;

    // Build title with countdown indicator, location icon, and category if applicable
    let mut title_text = String::new();
    
    // Add countdown indicator
    if has_countdown {
        title_text.push_str("⏱ ");
    }
    
    // Add location icon if event has a location
    if event.location.as_ref().map(|l| !l.is_empty()).unwrap_or(false) {
        title_text.push_str("📍");
    }
    
    title_text.push_str(&event.title);
    
    // Add category badge if present
    if let Some(category) = &event.category {
        title_text.push_str(&format!(" [{}]", category));
    }

    // Dim text for past events
    let text_color = if is_past {
        Color32::from_rgba_unmultiplied(255, 255, 255, 180)
    } else {
        Color32::WHITE
    };

    let layout_job = egui::text::LayoutJob::simple(
        title_text,
        font_id,
        text_color,
        available_width,
    );

    let galley = ui.fonts(|f| f.layout_job(layout_job));

    ui.painter().galley(
        Pos2::new(cell_rect.left() + 5.0, cell_rect.top() + 5.0),
        galley,
        text_color,
    );

    bar_rect
}

/// Render a continuation block for events spanning multiple time slots.
/// This extends upward to cover the grid line at the top of the cell,
/// making multi-slot events appear as one contiguous block.
/// If `continues_to_next_slot` is true, the bottom edge also extends to connect
/// with the next continuation block.
pub fn render_event_continuation(
    ui: &mut egui::Ui, 
    cell_rect: Rect, 
    event: &Event,
    continues_to_next_slot: bool,
) -> Rect {
    let now = Local::now();
    let is_past = event.end < now;
    
    let base_color = event
        .color
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color32::from_rgb(100, 150, 200));

    // Dim past events further (continuation blocks are already dimmer)
    let event_color = if is_past {
        // Stronger dimming for past continuation blocks
        Color32::from_rgba_unmultiplied(
            (base_color.r() as f32 * 0.25) as u8,
            (base_color.g() as f32 * 0.25) as u8,
            (base_color.b() as f32 * 0.25) as u8,
            120,
        )
    } else {
        base_color.linear_multiply(0.5)
    };

    // Extend upward to cover the grid line (start at cell top, not top + 2)
    // This makes the continuation seamlessly connect with the previous slot
    // If continuing to next slot, extend to bottom too
    let bottom_margin = if continues_to_next_slot { 0.0 } else { 2.0 };
    let bg_rect = Rect::from_min_size(
        Pos2::new(cell_rect.left() + 1.0, cell_rect.top()),
        Vec2::new(cell_rect.width() - 2.0, cell_rect.height() - bottom_margin),
    );
    
    // Only round bottom corners if this is the last slot of the event
    let rounding = if continues_to_next_slot {
        egui::Rounding::ZERO
    } else {
        egui::Rounding { nw: 0.0, ne: 0.0, sw: 2.0, se: 2.0 }
    };
    ui.painter().rect_filled(bg_rect, rounding, event_color);

    bg_rect
}

/// Draw the current time indicator line across a day column.
pub fn draw_current_time_indicator(
    ui: &mut egui::Ui,
    dates: &[NaiveDate],
    col_width: f32,
    time_label_width: f32,
    spacing: f32,
) {
    let now = Local::now();
    let now_date = now.date_naive();
    let now_time = now.time();

    if let Some(day_index) = dates.iter().position(|d| *d == now_date) {
        // Calculate Y position based on time
        // Each hour has 4 slots (15 minutes each), each slot is SLOT_HEIGHT pixels
        const SLOTS_PER_HOUR: f32 = 4.0;
        
        let hours_since_midnight = now_time.hour() as f32 + (now_time.minute() as f32 / 60.0);
        let relative_y = hours_since_midnight * SLOTS_PER_HOUR * SLOT_HEIGHT;

        let ui_top = ui.min_rect().top();
        let y_position = ui_top + relative_y;

        let ui_left = ui.min_rect().left();
        let x_start =
            ui_left + time_label_width + spacing + (day_index as f32 * (col_width + spacing));
        let x_end = x_start + col_width;

        let painter = ui.painter();
        let line_color = Color32::from_rgb(255, 100, 100);
        let circle_center = egui::pos2(x_start - 4.0, y_position);

        painter.circle_filled(circle_center, 3.0, line_color);
        painter.line_segment(
            [
                egui::pos2(x_start, y_position),
                egui::pos2(x_end, y_position),
            ],
            egui::Stroke::new(2.0, line_color),
        );
    }
}

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
        
        // Check if event ends in this slot
        let segment_end = event_time_segment_for_date(event, date)
            .map(|(_, end)| end)
            .unwrap_or_else(|| event.end.naive_local());
        let is_ending = segment_end > slot_start_dt && segment_end <= slot_end_dt;
        let continues_to_next_slot = segment_end > slot_end_dt;
        
        let event_rect = render_event_in_cell(ui, rect, event, has_countdown, continues_to_next_slot);
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
            if hovered_event.recurrence_rule.is_none() && !is_past_event {
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
            let tooltip_text = format_event_tooltip(hovered_event);
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

    // Context menu handling
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
        &response,
        egui::AboveOrBelow::Below,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(150.0);
            let popup_event = context_menu_event
                .clone()
                .or_else(|| single_event_fallback.clone());

            if let Some(event) = popup_event {
                ui.label(format!("Event: {}", event.title));
                ui.separator();

                if ui.button("✏ Edit").clicked() {
                    context_clicked_event = Some(event.clone());
                    ui.memory_mut(|mem| mem.close_popup());
                }

                // Show countdown option prominently for future events
                if event.start > Local::now() {
                    let timer_exists = event
                        .id
                        .map(|id| active_countdown_events.contains(&id))
                        .unwrap_or(false);
                    if timer_exists {
                        ui.label(
                            egui::RichText::new("⏱ Countdown active")
                                .italics()
                                .color(Color32::from_rgb(100, 200, 100))
                                .size(11.0),
                        );
                    } else if ui.button("⏱ Create Countdown").clicked() {
                        countdown_requests.push(CountdownRequest::from_event(&event));
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                    ui.separator();
                }

                if event.recurrence_rule.is_some() {
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

                if ui.button("📤 Export this event").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(&format!("{}.ics", event.title.replace(' ', "_")))
                        .add_filter("iCalendar", &["ics"])
                        .save_file()
                    {
                        use crate::services::icalendar::export;
                        match export::single(&event) {
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
            } else {
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
                            let label = format!("{}", template.name);
                            if ui.button(&label).on_hover_text(format!(
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
                            )).clicked() {
                                if let Some(id) = template.id {
                                    result.template_selection = Some((id, date, Some(time)));
                                }
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        }
                    });
                }
            }
        },
    );

    // Copy context menu edit request to result
    if let Some(event) = context_clicked_event {
        result.event_to_edit = Some(event);
    }

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
            if let Some(resize_context) = ResizeContext::from_event(
                &event,
                handle,
                config.resize_view,
            ) {
                ResizeManager::begin(ui.ctx(), resize_context);
                ui.output_mut(|out| out.cursor_icon = handle.cursor_icon());
            }
        } else if let Some((hit_rect, event, _, _)) = pointer_hit.clone() {
            // Otherwise start a drag operation
            if event.recurrence_rule.is_none() {
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
            if result.event_to_edit.is_none() {
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

/// Render the full time grid for a set of dates.
#[allow(clippy::too_many_arguments)]
pub fn render_time_grid(
    ui: &mut egui::Ui,
    col_width: f32,
    dates: &[NaiveDate],
    events: &[Event],
    database: &'static Database,
    show_event_dialog: &mut bool,
    event_dialog_date: &mut Option<NaiveDate>,
    event_dialog_time: &mut Option<NaiveTime>,
    event_dialog_recurrence: &mut Option<String>,
    countdown_requests: &mut Vec<CountdownRequest>,
    active_countdown_events: &HashSet<i64>,
    palette: &TimeGridPalette,
    focus_request: &mut Option<AutoFocusRequest>,
    config: &TimeCellConfig,
) -> EventInteractionResult {
    let mut result = EventInteractionResult::default();

    // Remove vertical spacing between slots so time calculations are accurate
    ui.spacing_mut().item_spacing.y = 0.0;

    // Draw 24 hours with 4 slots each
    for hour in 0..24 {
        for slot in 0..4 {
            let minute = slot * SLOT_INTERVAL;
            let time = NaiveTime::from_hms_opt(hour as u32, minute as u32, 0).unwrap();
            let is_hour_start = slot == 0;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                // Time label
                ui.allocate_ui_with_layout(
                    Vec2::new(TIME_LABEL_WIDTH, SLOT_HEIGHT),
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

                ui.add_space(COLUMN_SPACING);

                // Day columns
                for (day_idx, date) in dates.iter().enumerate() {
                    let slot_start = time;
                    let slot_end = {
                        let total_minutes = hour * 60 + (minute + SLOT_INTERVAL);
                        let end_hour = (total_minutes / 60) as u32;
                        let end_minute = (total_minutes % 60) as u32;
                        if end_hour >= 24 {
                            NaiveTime::from_hms_opt(23, 59, 59).unwrap()
                        } else {
                            NaiveTime::from_hms_opt(end_hour, end_minute, 0).unwrap()
                        }
                    };

                    // Categorize events for this slot
                    let mut starting_events: Vec<&Event> = Vec::new();
                    let mut continuing_events: Vec<&Event> = Vec::new();

                    let slot_start_dt = date.and_time(slot_start);
                    let slot_end_dt = date.and_time(slot_end);

                    for event in events.iter() {
                        let Some((segment_start, segment_end)) =
                            event_time_segment_for_date(event, *date)
                        else {
                            continue;
                        };

                        if segment_start >= slot_end_dt || segment_end <= slot_start_dt {
                            continue;
                        }

                        if segment_start >= slot_start_dt && segment_start < slot_end_dt {
                            starting_events.push(event);
                        } else if segment_start < slot_start_dt {
                            continuing_events.push(event);
                        }
                    }

                    let cell_result = render_time_cell(
                        ui,
                        col_width,
                        *date,
                        time,
                        slot_end,
                        is_hour_start,
                        &starting_events,
                        &continuing_events,
                        database,
                        show_event_dialog,
                        event_dialog_date,
                        event_dialog_time,
                        event_dialog_recurrence,
                        countdown_requests,
                        active_countdown_events,
                        palette,
                        focus_request,
                        config,
                    );
                    result.merge(cell_result);

                    if day_idx < dates.len() - 1 {
                        ui.add_space(COLUMN_SPACING);
                    }
                }
            });
        }
    }

    // Draw current time indicator
    draw_current_time_indicator(ui, dates, col_width, TIME_LABEL_WIDTH, COLUMN_SPACING);

    result
}
