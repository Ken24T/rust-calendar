//! Event rendering helpers for the day view's time slots.
//!
//! Contains visual rendering of individual event blocks and continuation bars
//! within day view time slots.  These are the day-view–specific counterparts
//! to the shared helpers in `event_rendering.rs`.

use chrono::Local;
use egui::{Color32, Pos2, Rect, Vec2};

use super::week_shared::parse_color;
use crate::models::event::Event;

/// Render a starting event block inside a day-view time slot.
///
/// Draws a coloured background with accent bar, title text, and time range.
/// Past events are dimmed for visual distinction.
///
/// Returns the bounding rectangle of the rendered event block.
pub fn render_event_in_slot(
    ui: &mut egui::Ui,
    slot_rect: Rect,
    event: &Event,
    is_synced: bool,
) -> Rect {
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

    // Event background - fill the slot area (after the time label)
    let bg_rect = Rect::from_min_size(
        Pos2::new(slot_rect.left() + 55.0, slot_rect.top() + 2.0),
        Vec2::new(slot_rect.width() - 60.0, slot_rect.height() - 4.0),
    );
    ui.painter().rect_filled(bg_rect, 2.0, event_color);

    // Event bar (left side) - darker accent
    let bar_rect = Rect::from_min_size(
        Pos2::new(slot_rect.left() + 55.0, slot_rect.top() + 2.0),
        Vec2::new(4.0, slot_rect.height() - 4.0),
    );
    ui.painter()
        .rect_filled(bar_rect, 2.0, event_color.linear_multiply(0.7));

    // Event title
    let text_rect = Rect::from_min_size(
        Pos2::new(bar_rect.right() + 5.0, slot_rect.top() + 2.0),
        Vec2::new(slot_rect.width() - 70.0, slot_rect.height() - 4.0),
    );

    // Use egui's layout system to properly truncate text
    let font_id = egui::FontId::proportional(13.0);
    let available_width = text_rect.width();

    // Dim text for past events
    let text_color = if is_past {
        Color32::from_rgba_unmultiplied(255, 255, 255, 180)
    } else {
        Color32::WHITE
    };

    let layout_job = egui::text::LayoutJob::simple(
        if is_synced {
            format!("🔒 {}", event.title)
        } else {
            event.title.clone()
        },
        font_id.clone(),
        text_color,
        available_width,
    );

    let galley = ui.fonts(|f| f.layout_job(layout_job));

    ui.painter().galley(
        Pos2::new(
            text_rect.left(),
            text_rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        text_color,
    );

    // Time range
    let time_str = format!(
        "{} - {}",
        event.start.format("%H:%M"),
        event.end.format("%H:%M")
    );
    ui.painter().text(
        Pos2::new(text_rect.left(), text_rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        time_str,
        egui::FontId::proportional(10.0),
        text_color,
    );

    bg_rect
}

/// Render a continuation bar for an event that spans into this slot.
///
/// Shows a thin coloured bar and faint background to indicate the event
/// is still active in this time range.
///
/// Returns the bounding rectangle of the rendered continuation block.
pub fn render_event_continuation(
    ui: &mut egui::Ui,
    slot_rect: Rect,
    event: &Event,
) -> Rect {
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
            (base_color.r() as f32 * 0.25) as u8,
            (base_color.g() as f32 * 0.25) as u8,
            (base_color.b() as f32 * 0.25) as u8,
            120,
        )
    } else {
        base_color
    };

    // Just render a colored bar to show the event continues through this slot
    let bar_rect = Rect::from_min_size(
        Pos2::new(slot_rect.left() + 55.0, slot_rect.top() + 2.0),
        Vec2::new(4.0, slot_rect.height() - 4.0),
    );
    ui.painter().rect_filled(bar_rect, 2.0, event_color);

    // Optional: render a lighter background to show continuation
    let bg_rect = Rect::from_min_size(
        Pos2::new(bar_rect.right() + 5.0, slot_rect.top() + 2.0),
        Vec2::new(slot_rect.width() - 70.0, slot_rect.height() - 4.0),
    );
    ui.painter()
        .rect_filled(bg_rect, 2.0, event_color.linear_multiply(0.3));

    bg_rect
}
