//! Resize handle drawing and preview rendering.
//!
//! Visual feedback for the event resize system: handle circles/bars drawn
//! on hovered events and translucent preview silhouettes during active drags.

use chrono::NaiveDate;
use chrono::NaiveTime;
use egui::{Pos2, Rect, Vec2};

use super::resize::{HandleRects, ResizeContext, ResizeHandle, HANDLE_VISUAL_SIZE};

/// Draw resize handles on an event.
pub fn draw_handles(
    ui: &mut egui::Ui,
    handles: &HandleRects,
    hovered_handle: Option<ResizeHandle>,
    color: egui::Color32,
) {
    let draw_handle = |rect: Rect, handle_type: ResizeHandle, is_hovered: bool| {
        // Position the visual elements at the edge
        let is_vertical_handle = matches!(handle_type, ResizeHandle::Top | ResizeHandle::Bottom);

        let (center, bar_start, bar_end) = if is_vertical_handle {
            // Top/Bottom handles - horizontal bar
            let center_x = rect.center().x;
            let bar_y = match handle_type {
                ResizeHandle::Top => rect.top() + 4.0,
                ResizeHandle::Bottom => rect.bottom() - 4.0,
                _ => rect.center().y,
            };
            let center_y = match handle_type {
                ResizeHandle::Top => rect.top() + HANDLE_VISUAL_SIZE / 2.0 + 4.0,
                ResizeHandle::Bottom => rect.bottom() - HANDLE_VISUAL_SIZE / 2.0 - 4.0,
                _ => rect.center().y,
            };
            let bar_width = rect.width().min(40.0);
            (
                Pos2::new(center_x, center_y),
                Pos2::new(center_x - bar_width / 2.0, bar_y),
                Pos2::new(center_x + bar_width / 2.0, bar_y),
            )
        } else {
            // Left/Right handles - vertical bar
            let center_y = rect.center().y;
            let bar_x = match handle_type {
                ResizeHandle::Left => rect.left() + 4.0,
                ResizeHandle::Right => rect.right() - 4.0,
                _ => rect.center().x,
            };
            let center_x = match handle_type {
                ResizeHandle::Left => rect.left() + HANDLE_VISUAL_SIZE / 2.0 + 2.0,
                ResizeHandle::Right => rect.right() - HANDLE_VISUAL_SIZE / 2.0 - 2.0,
                _ => rect.center().x,
            };
            let bar_height = rect.height().min(20.0);
            (
                Pos2::new(center_x, center_y),
                Pos2::new(bar_x, center_y - bar_height / 2.0),
                Pos2::new(bar_x, center_y + bar_height / 2.0),
            )
        };

        let radius = if is_hovered {
            HANDLE_VISUAL_SIZE / 2.0 + 2.0
        } else {
            HANDLE_VISUAL_SIZE / 2.0
        };

        // Draw a subtle bar indicator across the hit zone
        ui.painter().line_segment(
            [bar_start, bar_end],
            egui::Stroke::new(
                if is_hovered { 3.0 } else { 2.0 },
                if is_hovered {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180)
                },
            ),
        );

        // Draw circle handle (more prominent)
        ui.painter().circle_filled(
            center,
            radius,
            if is_hovered {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)
            },
        );
        ui.painter().circle_stroke(
            center,
            radius,
            egui::Stroke::new(
                if is_hovered { 2.0 } else { 1.5 },
                color.linear_multiply(0.8),
            ),
        );
    };

    if let Some(rect) = handles.top {
        draw_handle(
            rect,
            ResizeHandle::Top,
            hovered_handle == Some(ResizeHandle::Top),
        );
    }
    if let Some(rect) = handles.bottom {
        draw_handle(
            rect,
            ResizeHandle::Bottom,
            hovered_handle == Some(ResizeHandle::Bottom),
        );
    }
    if let Some(rect) = handles.left {
        draw_handle(
            rect,
            ResizeHandle::Left,
            hovered_handle == Some(ResizeHandle::Left),
        );
    }
    if let Some(rect) = handles.right {
        draw_handle(
            rect,
            ResizeHandle::Right,
            hovered_handle == Some(ResizeHandle::Right),
        );
    }
}

/// Draw a resize preview silhouette showing where the event will end up.
///
/// Parameters:
/// - `ui`: The egui Ui context
/// - `resize_ctx`: The active resize context with drag state
/// - `slot_rect`: The rect of the current time slot being rendered
/// - `slot_date`: The date of the current slot (to check if preview should appear in this column)
/// - `slot_time`: The start time of the current slot
/// - `slot_end_time`: The end time of the current slot
/// - `event_color`: The color of the event (will be made translucent)
/// - `left_margin`: Left margin for the event rect within the slot
#[allow(clippy::too_many_arguments)]
pub fn draw_resize_preview(
    ui: &mut egui::Ui,
    resize_ctx: &ResizeContext,
    slot_rect: Rect,
    slot_date: NaiveDate,
    slot_time: NaiveTime,
    slot_end_time: NaiveTime,
    event_color: egui::Color32,
    left_margin: f32,
) {
    // Get the new start/end times based on current drag position
    let (preview_start, preview_end) = match resize_ctx.hovered_times() {
        Some(times) => times,
        None => return, // No valid preview yet
    };

    // Only draw preview in the correct column (same date as the event)
    // For vertical resize, the date doesn't change
    let event_date = resize_ctx.original_start.date_naive();
    if slot_date != event_date {
        return;
    }

    let preview_start_time = preview_start.time();
    let preview_end_time = preview_end.time();

    // Check if this slot overlaps with the preview time range
    let slot_overlaps = preview_start_time < slot_end_time && preview_end_time > slot_time;

    if !slot_overlaps {
        return;
    }

    // Calculate the visible portion of the preview in this slot
    let visible_start = preview_start_time.max(slot_time);
    let visible_end = preview_end_time.min(slot_end_time);

    // Calculate Y positions within the slot
    let slot_duration = (slot_end_time - slot_time).num_minutes() as f32;
    let start_offset = (visible_start - slot_time).num_minutes() as f32 / slot_duration;
    let end_offset = (visible_end - slot_time).num_minutes() as f32 / slot_duration;

    let top_y = slot_rect.top() + slot_rect.height() * start_offset;
    let bottom_y = slot_rect.top() + slot_rect.height() * end_offset;

    // Create the preview rect (same layout as events)
    let preview_rect = Rect::from_min_max(
        Pos2::new(slot_rect.left() + left_margin, top_y + 2.0),
        Pos2::new(slot_rect.right() - 5.0, bottom_y - 2.0),
    );

    // Draw with a pale, translucent version of the event color
    let preview_color = egui::Color32::from_rgba_unmultiplied(
        event_color.r(),
        event_color.g(),
        event_color.b(),
        60, // Very translucent
    );

    // Fill
    ui.painter().rect_filled(preview_rect, 3.0, preview_color);

    // Dashed border effect - draw as a stroke with the event color
    let border_color = egui::Color32::from_rgba_unmultiplied(
        event_color.r(),
        event_color.g(),
        event_color.b(),
        140,
    );
    ui.painter().rect_stroke(
        preview_rect,
        3.0,
        egui::Stroke::new(2.0, border_color),
    );

    // Draw accent bar on left side
    let bar_rect = Rect::from_min_size(
        Pos2::new(preview_rect.left(), preview_rect.top()),
        Vec2::new(4.0, preview_rect.height()),
    );
    ui.painter().rect_filled(
        bar_rect,
        2.0,
        egui::Color32::from_rgba_unmultiplied(
            event_color.r(),
            event_color.g(),
            event_color.b(),
            100,
        ),
    );
}
