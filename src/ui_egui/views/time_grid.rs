//! Time grid rendering for week-based calendar views.
//!
//! Contains the outer grid loop that iterates over hours × day columns,
//! the current time indicator, and delegates cell rendering to `time_grid_cell`.

use chrono::{Local, NaiveDate, NaiveTime, Timelike};
use egui::{Color32, Vec2};
use std::collections::HashSet;

use super::palette::TimeGridPalette;
use super::time_grid_cell::{render_time_cell, TimeCellConfig};
use super::week_shared::{
    EventInteractionResult, COLUMN_SPACING, SLOT_HEIGHT,
    SLOT_INTERVAL, TIME_LABEL_WIDTH,
};
use super::{event_time_segment_for_date, AutoFocusRequest, CountdownRequest};
use crate::models::event::Event;
use crate::services::database::Database;

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
    synced_event_ids: &HashSet<i64>,
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
                        synced_event_ids,
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
