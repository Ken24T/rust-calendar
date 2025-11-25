//! Shared utilities and rendering logic for week-based views (Week and WorkWeek).
//!
//! This module extracts common code to reduce duplication between week_view.rs and workweek_view.rs.

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime, Timelike};
use egui::{Align, Color32, CursorIcon, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashSet;

use super::palette::TimeGridPalette;
use super::{event_time_segment_for_date, AutoFocusRequest, CountdownRequest};
use crate::models::event::Event;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::drag::{DragContext, DragManager, DragView};

/// Constants for time grid rendering
pub const SLOT_INTERVAL: i64 = 15;
pub const TIME_LABEL_WIDTH: f32 = 50.0;
pub const COLUMN_SPACING: f32 = 2.0;
pub const SLOT_HEIGHT: f32 = 30.0;

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
pub fn render_ribbon_event(
    ui: &mut egui::Ui,
    event: &Event,
    countdown_requests: &mut Vec<CountdownRequest>,
    active_countdown_events: &HashSet<i64>,
    database: &'static Database,
) -> Option<Event> {
    let event_color = event
        .color
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color32::from_rgb(100, 150, 200));

    let available_width = ui.available_width();

    let event_frame = egui::Frame::none()
        .fill(event_color)
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(6.0, 3.0));

    let response = event_frame
        .show(ui, |ui| {
            ui.set_min_width(available_width - 8.0);
            ui.horizontal(|ui| {
                // Show countdown indicator if this event has a card
                let has_countdown = event
                    .id
                    .map(|id| active_countdown_events.contains(&id))
                    .unwrap_or(false);
                if has_countdown {
                    ui.label(
                        egui::RichText::new("⏱")
                            .color(Color32::WHITE)
                            .size(11.0),
                    );
                }
                
                ui.label(
                    egui::RichText::new(&event.title)
                        .color(Color32::WHITE)
                        .size(12.0),
                );

                if event.start.date_naive() != event.end.date_naive() {
                    ui.label(
                        egui::RichText::new(format!(
                            "({} - {})",
                            event.start.format("%b %d"),
                            event.end.format("%b %d")
                        ))
                        .color(Color32::from_gray(220))
                        .size(10.0),
                    );
                }
            });
        })
        .response;

    // Add tooltip with days-until info for future events
    let now = Local::now();
    let interactive_response = if event.start > now {
        let days_until = (event.start.date_naive() - now.date_naive()).num_days();
        let tooltip = if days_until == 0 {
            format!("{}\nToday", event.title)
        } else if days_until == 1 {
            format!("{}\nTomorrow", event.title)
        } else {
            format!("{}\n{} days from now", event.title, days_until)
        };
        response.interact(Sense::click()).on_hover_text(tooltip)
    } else {
        response.interact(Sense::click())
    };
    let mut clicked_event = None;

    interactive_response.context_menu(|ui| {
        ui.set_min_width(150.0);
        ui.label(format!("Event: {}", event.title));
        ui.separator();

        if ui.button("✏ Edit").clicked() {
            clicked_event = Some(event.clone());
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
                    let service = EventService::new(database.connection());
                    let _ = service.delete_occurrence(id, event.start);
                }
                ui.close_menu();
            }
            if ui.button("🗑 Delete All Occurrences").clicked() {
                if let Some(id) = event.id {
                    let service = EventService::new(database.connection());
                    let _ = service.delete(id);
                }
                ui.close_menu();
            }
        } else if ui.button("🗑 Delete").clicked() {
            if let Some(id) = event.id {
                let service = EventService::new(database.connection());
                let _ = service.delete(id);
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

    clicked_event
}

/// Render an event bar inside a time cell (for events starting in this slot).
pub fn render_event_in_cell(
    ui: &mut egui::Ui, 
    cell_rect: Rect, 
    event: &Event,
    has_countdown: bool,
) -> Rect {
    let now = Local::now();
    let is_past = event.end < now;
    
    let base_color = event
        .color
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color32::from_rgb(100, 150, 200));
    
    // Dim past events by reducing opacity
    let event_color = if is_past {
        base_color.linear_multiply(0.5)
    } else {
        base_color
    };

    let bar_rect = Rect::from_min_size(
        Pos2::new(cell_rect.left() + 2.0, cell_rect.top() + 2.0),
        Vec2::new(cell_rect.width() - 4.0, cell_rect.height() - 4.0),
    );
    ui.painter().rect_filled(bar_rect, 2.0, event_color);

    let font_id = egui::FontId::proportional(10.0);
    let available_width = cell_rect.width() - 10.0;

    // Build title with countdown indicator if applicable
    let title_text = if has_countdown {
        format!("⏱ {}", event.title)
    } else {
        event.title.clone()
    };

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
pub fn render_event_continuation(ui: &mut egui::Ui, cell_rect: Rect, event: &Event) -> Rect {
    let now = Local::now();
    let is_past = event.end < now;
    
    let base_color = event
        .color
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(Color32::from_rgb(100, 150, 200));

    // Dim past events further
    let event_color = if is_past {
        base_color.linear_multiply(0.3)
    } else {
        base_color.linear_multiply(0.5)
    };

    let bg_rect = Rect::from_min_size(
        Pos2::new(cell_rect.left() + 2.0, cell_rect.top() + 2.0),
        Vec2::new(cell_rect.width() - 4.0, cell_rect.height() - 4.0),
    );
    ui.painter()
        .rect_filled(bg_rect, 2.0, event_color);

    bg_rect
}

/// Generate a rich tooltip string for an event.
/// Shows title, time range, location, and description preview.
pub fn format_event_tooltip(event: &Event) -> String {
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
    
    lines.join("\n")
}

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
) -> Option<Event> {
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

    let mut event_hitboxes: Vec<(Rect, Event)> = Vec::new();

    // Draw continuing events first
    for event in continuing_events {
        let event_rect = render_event_continuation(ui, rect, event);
        event_hitboxes.push((event_rect, (*event).clone()));
    }

    // Draw starting events
    for event in starting_events {
        let has_countdown = event
            .id
            .map(|id| active_countdown_events.contains(&id))
            .unwrap_or(false);
        let event_rect = render_event_in_cell(ui, rect, event, has_countdown);
        event_hitboxes.push((event_rect, (*event).clone()));
    }

    // Pointer hit detection
    let pointer_pos = response
        .interact_pointer_pos()
        .or_else(|| ui.input(|i| i.pointer.hover_pos()));
    let pointer_hit = pointer_pos.and_then(|pos| {
        event_hitboxes
            .iter()
            .rev()
            .find(|(hit_rect, _)| hit_rect.contains(pos))
            .map(|(hit_rect, event)| (*hit_rect, event.clone()))
    });
    let pointer_event = pointer_hit.as_ref().map(|(_, event)| event.clone());
    let single_event_fallback = if event_hitboxes.len() == 1 {
        Some(event_hitboxes[0].1.clone())
    } else {
        None
    };

    // Show tooltip when hovering over an event
    if let Some((hit_rect, hovered_event)) = &pointer_hit {
        if response.hovered() && hit_rect.contains(pointer_pos.unwrap_or_default()) {
            let tooltip_text = format_event_tooltip(hovered_event);
            response.clone().on_hover_ui_at_pointer(|ui| {
                ui.label(tooltip_text);
            });
        }
    }

    // Drag hover tracking
    let pointer_for_hover = ui
        .ctx()
        .pointer_interact_pos()
        .or_else(|| ui.input(|i| i.pointer.hover_pos()));
    if let Some(pointer) = pointer_for_hover {
        if rect.contains(pointer) {
            DragManager::update_hover(ui.ctx(), date, time, rect, pointer);
            if DragManager::is_active_for_view(ui.ctx(), config.drag_view) {
                ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
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
                            let service = EventService::new(database.connection());
                            let _ = service.delete_occurrence(id, event.start);
                        }
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                    if ui.button("🗑 Delete All Occurrences").clicked() {
                        if let Some(id) = event.id {
                            let service = EventService::new(database.connection());
                            let _ = service.delete(id);
                        }
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                } else if ui.button("🗑 Delete").clicked() {
                    if let Some(id) = event.id {
                        let service = EventService::new(database.connection());
                        let _ = service.delete(id);
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
            }
        },
    );

    let mut clicked_event: Option<Event> = context_clicked_event;

    // Drag handling
    if response.drag_started() {
        let view_name = match config.drag_view {
            DragView::Week => "week_view",
            DragView::WorkWeek => "workweek_view",
            _ => "view",
        };
        eprintln!(
            "[{}] drag_started detected, pointer_hit: {:?}",
            view_name,
            pointer_hit.as_ref().map(|(_, e)| &e.title)
        );
        if let Some((hit_rect, event)) = pointer_hit.clone() {
            eprintln!("[{}] event under pointer: {}", view_name, event.title);
            if event.recurrence_rule.is_none() {
                if let Some(drag_context) = DragContext::from_event(
                    &event,
                    pointer_pos
                        .map(|pos| pos - hit_rect.min)
                        .unwrap_or(Vec2::ZERO),
                    config.drag_view,
                ) {
                    eprintln!("[{}] starting drag for event: {}", view_name, event.title);
                    DragManager::begin(ui.ctx(), drag_context);
                    ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                }
            } else {
                eprintln!(
                    "[{}] cannot drag recurring event: {}",
                    view_name, event.title
                );
            }
        } else {
            eprintln!("[{}] drag_started but no event under pointer", view_name);
        }
    } else if clicked_event.is_none() && response.clicked() {
        if let Some(event) = pointer_event {
            clicked_event = Some(event);
        } else {
            *show_event_dialog = true;
            *event_dialog_date = Some(date);
            *event_dialog_time = Some(time);
            *event_dialog_recurrence = None;
        }
    }

    if response.double_clicked() {
        *show_event_dialog = true;
        *event_dialog_date = Some(date);
        *event_dialog_time = Some(time);
        *event_dialog_recurrence = Some("FREQ=WEEKLY".to_string());
    }

    if response.dragged() {
        ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
    }

    if response.drag_stopped() {
        if let Some(drag_context) = DragManager::finish_for_view(ui.ctx(), config.drag_view) {
            if let Some(target_start) = drag_context
                .hovered_start()
                .or_else(|| date.and_time(time).and_local_timezone(Local).single())
            {
                let new_end = target_start + drag_context.duration;
                let event_service = EventService::new(database.connection());
                if let Ok(Some(mut event)) = event_service.get(drag_context.event_id) {
                    event.start = target_start;
                    event.end = new_end;
                    if let Err(err) = event_service.update(&event) {
                        eprintln!("Failed to move event {}: {}", drag_context.event_id, err);
                    }
                }
            }
        }
    }

    clicked_event
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
) -> Option<Event> {
    let mut clicked_event: Option<Event> = None;

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

                    if let Some(event) = render_time_cell(
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
                    ) {
                        clicked_event = Some(event);
                    }

                    if day_idx < dates.len() - 1 {
                        ui.add_space(COLUMN_SPACING);
                    }
                }
            });
        }
    }

    // Draw current time indicator
    draw_current_time_indicator(ui, dates, col_width, TIME_LABEL_WIDTH, COLUMN_SPACING);

    clicked_event
}
