//! Event rendering helpers for week-based views.
//!
//! Contains the low-level painting functions that draw individual event blocks,
//! continuation bars, tooltips, and colour parsing for the time grid.

use chrono::Local;
use egui::{Color32, Pos2, Rect, Vec2};

use crate::models::event::Event;

/// Parse a hex color string to Color32.
pub fn parse_color(hex: &str) -> Option<Color32> {
    if hex.is_empty() {
        return None;
    }

    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color32::from_rgb(r, g, b))
}

/// Render an event bar inside a time cell (for events starting in this slot).
/// If `continues_to_next_slot` is true, the bottom edge extends to connect
/// with continuation blocks in subsequent slots.
pub fn render_event_in_cell(
    ui: &mut egui::Ui, 
    cell_rect: Rect, 
    event: &Event,
    has_countdown: bool,
    is_synced: bool,
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

    if is_synced {
        title_text.push_str("🔒 ");
    }
    
    // Add location icon if event has a location
    if event.location.as_ref().map(|l| !l.is_empty()).unwrap_or(false) {
        title_text.push('📍');
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

/// Generate a rich tooltip string for an event.
/// Shows title, time range, location, and description preview.
pub fn format_event_tooltip(event: &Event, is_synced: bool) -> String {
    let mut lines = Vec::new();
    
    // Title (bold via unicode)
    lines.push(format!("📌 {}", event.title));
    
    // Time
    if event.all_day {
        let date_str = event.start.format("%A, %B %d, %Y").to_string();
        lines.push(format!("🕐 All day - {}", date_str));
    } else {
        let start_str = event.start.format("%H:%M").to_string();
        let end_str = event.end.format("%H:%M").to_string();
        let date_str = event.start.format("%A, %B %d").to_string();
        lines.push(format!("🕐 {} - {} ({})", start_str, end_str, date_str));
    }
    
    // Location
    if let Some(ref location) = event.location {
        if !location.is_empty() {
            lines.push(format!("📍 {}", location));
        }
    }
    
    // Category
    if let Some(ref category) = event.category {
        if !category.is_empty() {
            lines.push(format!("🏷️ {}", category));
        }
    }
    
    // Recurring indicator
    if event.recurrence_rule.is_some() {
        lines.push("🔄 Recurring event".to_string());
    }

    if is_synced {
        lines.push("🔒 Synced read-only event".to_string());
    }
    
    // Description preview (truncated)
    if let Some(ref description) = event.description {
        if !description.is_empty() {
            let preview = if description.len() > 100 {
                format!("{}...", &description[..100])
            } else {
                description.clone()
            };
            lines.push(format!("\n📝 {}", preview));
        }
    }
    
    // Add interaction hint
    if is_synced {
        lines.push("\n💡 Right-click for details and export".to_string());
    } else {
        lines.push("\n💡 Double-click to edit, right-click for more options".to_string());
    }
    
    lines.join("\n")
}
