use chrono::{Datelike, Local, NaiveDate};
use egui::{Color32, Margin, Sense, Stroke, Vec2};
use std::collections::HashSet;

use super::palette::{CalendarCellPalette, DayStripPalette};
use super::week_shared::DeleteConfirmRequest;
use super::{
    filter_events_by_category, filter_events_by_sync_scope, CountdownRequest,
};
use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::theme::CalendarTheme;

/// Width of the week number column
const WEEK_NUMBER_WIDTH: f32 = 35.0;

/// Result returned from month view
pub struct MonthViewResult {
    /// Action to perform
    pub action: MonthViewAction,
    /// Request to show delete confirmation dialog
    pub delete_confirm_request: Option<DeleteConfirmRequest>,
}

impl Default for MonthViewResult {
    fn default() -> Self {
        Self {
            action: MonthViewAction::None,
            delete_confirm_request: None,
        }
    }
}

/// Action returned from month view
pub enum MonthViewAction {
    /// No action
    None,
    /// Switch to day view for a specific date
    SwitchToDayView(NaiveDate),
    /// Switch to user's default view for a specific date
    SwitchToDefaultView(NaiveDate),
    /// Create event from template (template_id, date)
    CreateFromTemplate(i64, NaiveDate),
}

/// Blend header color for weekend columns (slightly darker/lighter)
fn blend_header_weekend(header_bg: Color32, is_dark: bool) -> Color32 {
    let factor = if is_dark { 1.15 } else { 0.92 };
    Color32::from_rgb(
        ((header_bg.r() as f32 * factor).min(255.0)) as u8,
        ((header_bg.g() as f32 * factor).min(255.0)) as u8,
        ((header_bg.b() as f32 * factor).min(255.0)) as u8,
    )
}

pub struct MonthView;

impl MonthView {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        database: &'static Database,
        settings: &Settings,
        theme: &CalendarTheme,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
        event_to_edit: &mut Option<i64>,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        category_filter: Option<&str>,
        synced_only: bool,
        synced_source_id: Option<i64>,
    ) -> MonthViewResult {
        let today = Local::now().date_naive();
        let mut result = MonthViewResult::default();

        // Get events for the month
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_month(&event_service, *current_date);
        let events = filter_events_by_category(events, category_filter);
        let events = filter_events_by_sync_scope(events, database, synced_only, synced_source_id);
        let synced_event_ids = super::load_synced_event_ids(database, synced_source_id);

        // Day of week headers - use Grid to match column widths below
        let day_names = Self::get_day_names(settings.first_day_of_week);
        let spacing = 2.0;
        let show_week_numbers = settings.show_week_numbers;
        let week_col_extra = if show_week_numbers { WEEK_NUMBER_WIDTH + spacing } else { 0.0 };
        let total_spacing = spacing * 6.0; // 6 gaps between 7 columns
        let col_width = (ui.available_width() - total_spacing - week_col_extra) / 7.0;

        let day_strip_palette = DayStripPalette::from_theme(theme);
        egui::Grid::new("month_header_grid")
            .spacing([spacing, spacing])
            .show(ui, |ui| {
                // Week number header (empty)
                if show_week_numbers {
                    ui.allocate_ui_with_layout(
                        Vec2::new(WEEK_NUMBER_WIDTH, 30.0),
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            egui::Frame::none()
                                .fill(day_strip_palette.header_bg)
                                .rounding(egui::Rounding::same(6.0))
                                .stroke(Stroke::new(1.0, day_strip_palette.strip_border))
                                .inner_margin(Margin::symmetric(4.0, 6.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Wk")
                                            .size(12.0)
                                            .color(day_strip_palette.header_text)
                                            .strong(),
                                    );
                                });
                        },
                    );
                }
                
                for (idx, day) in day_names.iter().enumerate() {
                    let weekday = (settings.first_day_of_week as usize + idx) % 7;
                    let is_weekend = weekday == 0 || weekday == 6;
                    let header_bg = if is_weekend {
                        // Slightly darker/lighter for weekend headers
                        blend_header_weekend(day_strip_palette.header_bg, theme.is_dark)
                    } else {
                        day_strip_palette.header_bg
                    };
                    let text_color = day_strip_palette.header_text;

                    ui.allocate_ui_with_layout(
                        Vec2::new(col_width, 30.0),
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            egui::Frame::none()
                                .fill(header_bg)
                                .rounding(egui::Rounding::same(6.0))
                                .stroke(Stroke::new(1.0, day_strip_palette.strip_border))
                                .inner_margin(Margin::symmetric(8.0, 6.0))
                                .show(ui, |cell_ui| {
                                    cell_ui.centered_and_justified(|label_ui| {
                                        label_ui.label(
                                            egui::RichText::new(*day)
                                                .size(14.0)
                                                .color(text_color)
                                                .strong(),
                                        );
                                    });
                                });
                        },
                    );
                }
            });

        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);

        // Calculate calendar grid
        let first_of_month = current_date.with_day(1).unwrap();
        let first_weekday = (first_of_month.weekday().num_days_from_sunday() as i32
            - settings.first_day_of_week as i32
            + 7)
            % 7;
        let days_in_month = Self::get_days_in_month(current_date.year(), current_date.month());

        // Calculate how many weeks are needed for this month
        // Total cells needed = days before month start + days in month
        let total_cells = first_weekday + days_in_month;
        let weeks_needed = (total_cells + 6) / 7; // Ceiling division

        // Build calendar grid (dynamic number of rows based on month)
        let mut day_counter = 1 - first_weekday;

        let palette = CalendarCellPalette::from_theme(theme);

        egui::Grid::new("month_grid")
            .spacing([spacing, spacing])
            .show(ui, |ui| {
                for _week_row in 0..weeks_needed {
                    // Week number column
                    if show_week_numbers {
                        // Calculate the date for this row (use middle of week for reliability)
                        let row_day = day_counter + 3; // Middle of week
                        let week_date = if row_day >= 1 && row_day <= days_in_month {
                            NaiveDate::from_ymd_opt(
                                current_date.year(),
                                current_date.month(),
                                row_day as u32,
                            )
                        } else if row_day < 1 {
                            // Previous month - use day 1
                            NaiveDate::from_ymd_opt(
                                current_date.year(),
                                current_date.month(),
                                1,
                            )
                        } else {
                            // Next month - use last day
                            NaiveDate::from_ymd_opt(
                                current_date.year(),
                                current_date.month(),
                                days_in_month as u32,
                            )
                        };
                        
                        if let Some(date) = week_date {
                            let week_num = date.iso_week().week();
                            ui.allocate_ui_with_layout(
                                Vec2::new(WEEK_NUMBER_WIDTH, 80.0),
                                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                |ui| {
                                    egui::Frame::none()
                                        .fill(palette.empty_bg)
                                        .rounding(egui::Rounding::same(4.0))
                                        .inner_margin(Margin::symmetric(2.0, 4.0))
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{}", week_num))
                                                    .size(11.0)
                                                    .color(palette.text.gamma_multiply(0.7)),
                                            );
                                        });
                                },
                            );
                        }
                    }

                    for _day_of_week in 0..7 {
                        if day_counter < 1 || day_counter > days_in_month {
                            // Empty cell for days outside current month
                            let (rect, _response) = ui.allocate_exact_size(
                                Vec2::new(col_width, 80.0),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 2.0, palette.empty_bg);
                        } else {
                            // Day cell
                            let date = NaiveDate::from_ymd_opt(
                                current_date.year(),
                                current_date.month(),
                                day_counter as u32,
                            )
                            .unwrap();

                            let is_today = date == today;

                            // Calculate weekend based on first_day_of_week
                            // If Sunday is first day (0), weekend is days 0 and 6
                            // If Monday is first day (1), weekend is days 5 and 6 (Sat, Sun)
                            let day_of_week = (date.weekday().num_days_from_sunday() as i32
                                - settings.first_day_of_week as i32
                                + 7)
                                % 7;
                            let is_weekend = day_of_week == 5 || day_of_week == 6;

                            // Get events for this day
                            let day_events: Vec<&Event> = events
                                .iter()
                                .filter(|e| {
                                    if e.all_day {
                                        super::event_covers_date(e, date)
                                    } else {
                                        e.start.date_naive() == date
                                    }
                                })
                                .collect();

                            let (cell_action, _clicked_event, delete_request) = Self::render_day_cell(
                                ui,
                                day_counter,
                                date,
                                is_today,
                                is_weekend,
                                &day_events,
                                &synced_event_ids,
                                database,
                                show_event_dialog,
                                event_dialog_date,
                                event_dialog_recurrence,
                                event_to_edit,
                                countdown_requests,
                                active_countdown_events,
                                palette,
                                col_width,
                            );
                            
                            // Check if we need to switch views
                            if !matches!(cell_action, MonthViewAction::None) {
                                result.action = cell_action;
                            }
                            
                            // Check if there's a delete confirmation request
                            if delete_request.is_some() {
                                result.delete_confirm_request = delete_request;
                            }
                        }
                        day_counter += 1;
                    }
                    ui.end_row();
                }
            });
        
        result
    }

    fn get_events_for_month(event_service: &EventService, date: NaiveDate) -> Vec<Event> {
        use chrono::{Local, TimeZone};

        let start_of_month = date.with_day(1).unwrap();
        let start = Local
            .from_local_datetime(&start_of_month.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();

        // Get last day of month
        let days_in_month = Self::get_days_in_month(date.year(), date.month());
        let end_of_month = date.with_day(days_in_month as u32).unwrap();
        let end = Local
            .from_local_datetime(&end_of_month.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();

        event_service
            .expand_recurring_events(start, end)
            .unwrap_or_default()
    }

    fn get_days_in_month(year: i32, month: u32) -> i32 {
        NaiveDate::from_ymd_opt(
            if month == 12 { year + 1 } else { year },
            if month == 12 { 1 } else { month + 1 },
            1,
        )
        .unwrap()
        .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
        .num_days() as i32
    }

    fn get_day_names(first_day_of_week: u8) -> Vec<&'static str> {
        let all_days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let start = first_day_of_week as usize;
        let mut result = Vec::with_capacity(7);
        for i in 0..7 {
            result.push(all_days[(start + i) % 7]);
        }
        result
    }

}
