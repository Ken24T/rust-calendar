use chrono::{Local, NaiveDate, NaiveTime, TimeZone, Timelike};
use egui::{Color32, Margin, Stroke};
use std::collections::HashSet;

use super::palette::{DayStripPalette, TimeGridPalette};
use super::week_shared::EventInteractionResult;
use super::{AutoFocusRequest, CountdownRequest};
use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::theme::CalendarTheme;

use super::{
    filter_events_by_category, filter_events_by_sync_scope,
    load_synced_event_ids,
};

pub struct DayView;

impl DayView {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        database: &'static Database,
        settings: &Settings,
        theme: &CalendarTheme,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        focus_request: &mut Option<AutoFocusRequest>,
        category_filter: Option<&str>,
        synced_only: bool,
        synced_source_id: Option<i64>,
    ) -> EventInteractionResult {
        let mut result = EventInteractionResult::default();
        let today = Local::now().date_naive();
        let is_today = *current_date == today;
        let day_strip_palette = DayStripPalette::from_theme(theme);
        let time_palette = TimeGridPalette::from_theme(theme);

        // Get events for this day
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_day(&event_service, *current_date);
        let events = filter_events_by_category(events, category_filter);
        let events = filter_events_by_sync_scope(events, database, synced_only, synced_source_id);
        let synced_event_ids = load_synced_event_ids(database, None);

        // Day header
        let day_name = current_date.format("%A").to_string();
        let date_label = current_date.format("%B %d, %Y").to_string();
        let header_frame = egui::Frame::none()
            .fill(day_strip_palette.header_bg)
            .rounding(egui::Rounding::same(12.0))
            .stroke(Stroke::new(1.0, day_strip_palette.strip_border))
            .inner_margin(Margin::symmetric(16.0, 12.0));

        let header_response = header_frame.show(ui, |strip_ui| {
            strip_ui.horizontal(|row_ui| {
                row_ui.vertical(|text_ui| {
                    let heading_color = if is_today {
                        day_strip_palette.today_text
                    } else {
                        day_strip_palette.header_text
                    };
                    let date_color = if is_today {
                        day_strip_palette.today_date_text
                    } else {
                        day_strip_palette.header_text
                    };

                    text_ui.label(
                        egui::RichText::new(&day_name)
                            .size(24.0)
                            .color(heading_color)
                            .strong(),
                    );
                    text_ui.label(
                        egui::RichText::new(&date_label)
                            .size(14.0)
                            .color(date_color),
                    );
                });

                let remaining_width = row_ui.available_width();
                row_ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |today_ui| {
                        today_ui.set_width(remaining_width);
                        if is_today {
                            egui::Frame::none()
                                .fill(day_strip_palette.badge_bg)
                                .rounding(egui::Rounding::same(10.0))
                                .inner_margin(Margin::symmetric(12.0, 6.0))
                                .show(today_ui, |badge_ui| {
                                    badge_ui.label(
                                        egui::RichText::new("Today")
                                            .color(day_strip_palette.badge_text)
                                            .size(12.0)
                                            .strong(),
                                    );
                                });
                        }
                    },
                );
            });
        });

        let header_rect = header_response.response.rect;
        ui.painter().hline(
            header_rect.x_range(),
            header_rect.bottom(),
            Stroke::new(1.0, day_strip_palette.accent_line),
        );

        ui.add_space(8.0);

        // Scrollable time slots
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let slot_result = Self::render_time_slots(
                    ui,
                    *current_date,
                    &events,
                    &synced_event_ids,
                    settings,
                    database,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_time,
                    event_dialog_recurrence,
                    countdown_requests,
                    active_countdown_events,
                    &time_palette,
                    focus_request,
                );
                result.merge(slot_result);
            });

        result
    }

    #[allow(clippy::too_many_arguments)]
    fn render_time_slots(
        ui: &mut egui::Ui,
        date: NaiveDate,
        events: &[Event],
        synced_event_ids: &HashSet<i64>,
        _settings: &Settings,
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
        // Always render 15-minute intervals (4 slots per hour)
        const SLOT_INTERVAL: i64 = 15;

        // Remove vertical spacing between slots so time calculations are accurate
        ui.spacing_mut().item_spacing.y = 0.0;

        // Draw 24 hours with 4 slots each
        for hour in 0..24 {
            for slot in 0..4 {
                let minute = slot * SLOT_INTERVAL;
                let time = NaiveTime::from_hms_opt(hour as u32, minute as u32, 0).unwrap();
                let is_hour_start = slot == 0;

                // Calculate slot time range
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

                // Categorize events for this slot:
                // 1. Events that START in this slot (render full details)
                // 2. Events that are CONTINUING through this slot (render colored block only)
                let mut starting_events: Vec<&Event> = Vec::new();
                let mut continuing_events: Vec<&Event> = Vec::new();

                for event in events.iter() {
                    let event_start = event.start.time();
                    let event_end = event.end.time();

                    // Check if event overlaps with this slot
                    if event_start < slot_end && event_end > slot_start {
                        // Does it start in this slot?
                        if event_start >= slot_start && event_start < slot_end {
                            starting_events.push(event);
                        } else if event_start < slot_start {
                            // It started earlier and is continuing through this slot
                            continuing_events.push(event);
                        }
                    }
                }

                let slot_result = Self::render_time_slot(
                    ui,
                    date,
                    time,
                    hour,
                    slot_end,
                    is_hour_start,
                    &starting_events,
                    &continuing_events,
                    synced_event_ids,
                    database,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_time,
                    event_dialog_recurrence,
                    countdown_requests,
                    active_countdown_events,
                    palette,
                    focus_request,
                );
                result.merge(slot_result);
            }
        }

        // Draw current time indicator if viewing today
        let now = Local::now();
        let now_date = now.date_naive();
        let now_time = now.time();

        if date == now_date {
            // Calculate Y position based on time
            // Each hour has 4 slots (15 minutes each), each slot is 40 pixels high
            const SLOT_HEIGHT: f32 = 40.0;
            const SLOTS_PER_HOUR: f32 = 4.0;
            
            let hours_since_midnight = now_time.hour() as f32 + (now_time.minute() as f32 / 60.0);
            let relative_y = hours_since_midnight * SLOTS_PER_HOUR * SLOT_HEIGHT;

            // Get the UI's current position to calculate absolute coordinates
            let ui_top = ui.min_rect().top();
            let y_position = ui_top + relative_y;

            // Calculate X position across the full width
            let ui_left = ui.min_rect().left();
            let ui_right = ui.min_rect().right();
            let x_start = ui_left + 50.0; // After time labels
            let x_end = ui_right;

            // Draw the indicator line
            let painter = ui.painter();
            let line_color = Color32::from_rgb(255, 100, 100); // Red indicator
            let circle_center = egui::pos2(ui_left + 46.0, y_position);

            // Draw a small circle at the start
            painter.circle_filled(circle_center, 3.0, line_color);

            // Draw the horizontal line
            painter.line_segment(
                [
                    egui::pos2(x_start, y_position),
                    egui::pos2(x_end, y_position),
                ],
                egui::Stroke::new(2.0, line_color),
            );
        }

        result
    }

    fn get_events_for_day(event_service: &EventService, date: NaiveDate) -> Vec<Event> {
        let start = Local
            .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let end = Local
            .from_local_datetime(&date.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();

        event_service
            .expand_recurring_events(start, end)
            .unwrap_or_default()
            .into_iter()
            .filter(|e| !e.all_day)
            .collect()
    }
}
