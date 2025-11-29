use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone};
use egui::{Color32, Margin, Stroke, Vec2};
use std::collections::HashSet;

use super::palette::DayStripPalette;
use super::week_shared::{
    self, format_short_date, get_week_start, render_ribbon_event, render_time_grid,
    EventInteractionResult, TimeCellConfig, COLUMN_SPACING, TIME_LABEL_WIDTH,
};
use super::{filter_events_by_category, AutoFocusRequest, CountdownRequest};
use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::drag::DragView;
use crate::ui_egui::resize::ResizeView;
use crate::ui_egui::theme::CalendarTheme;

/// Blend header color for weekend columns (slightly darker/lighter)
fn blend_header_weekend(header_bg: Color32, is_dark: bool) -> Color32 {
    let factor = if is_dark { 1.15 } else { 0.92 };
    Color32::from_rgb(
        ((header_bg.r() as f32 * factor).min(255.0)) as u8,
        ((header_bg.g() as f32 * factor).min(255.0)) as u8,
        ((header_bg.b() as f32 * factor).min(255.0)) as u8,
    )
}

pub struct WeekView;

impl WeekView {
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
        show_ribbon: bool,
        all_day_events: &[Event],
        focus_request: &mut Option<AutoFocusRequest>,
        category_filter: Option<&str>,
    ) -> EventInteractionResult {
        let mut result = EventInteractionResult::default();
        let today = Local::now().date_naive();
        let day_strip_palette = DayStripPalette::from_theme(theme);
        let grid_palette = super::palette::TimeGridPalette::from_theme(theme);

        // Get week start based on settings
        let week_start = get_week_start(*current_date, settings.first_day_of_week);
        let week_dates: Vec<NaiveDate> = (0..7).map(|i| week_start + Duration::days(i)).collect();

        // Get events for the week
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_week(&event_service, week_start);
        let events = filter_events_by_category(events, category_filter);

        let day_names = Self::get_day_names(settings.first_day_of_week);
        let total_spacing = COLUMN_SPACING * 6.0; // 6 gaps between 7 columns
        let show_week_numbers = settings.show_week_numbers;

        // Week header with day names
        let header_frame = egui::Frame::none()
            .fill(day_strip_palette.header_bg)
            .rounding(egui::Rounding::same(10.0))
            .stroke(Stroke::new(1.0, day_strip_palette.strip_border))
            .inner_margin(Margin {
                left: 0.0,
                right: 0.0,
                top: 10.0,
                bottom: 10.0,
            });

        let header_response = header_frame.show(ui, |strip_ui| {
            // Calculate column width based on actual available width in this context
            let frame_available_width = strip_ui.available_width();
            let frame_available_for_cols =
                frame_available_width - TIME_LABEL_WIDTH - total_spacing;
            let col_width = frame_available_for_cols / 7.0;

            // Header row with day names (and optional week number)
            strip_ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                // Time label placeholder - show week number if enabled
                // Use allocate_exact_size to ensure the space is reserved even when empty
                let (rect, _response) =
                    ui.allocate_exact_size(Vec2::new(TIME_LABEL_WIDTH, 48.0), egui::Sense::hover());
                if show_week_numbers {
                    let week_num = week_start.iso_week().week();
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("W{}", week_num),
                        egui::FontId::proportional(12.0),
                        day_strip_palette.header_text,
                    );
                }

                ui.add_space(COLUMN_SPACING);

                for (i, day_name) in day_names.iter().enumerate() {
                    let date = week_dates[i];
                    let is_today = date == today;
                    let weekday_idx = date.weekday().num_days_from_sunday();
                    let is_weekend = weekday_idx == 0 || weekday_idx == 6;
                    
                    // Use header colors for the day header cells
                    let cell_bg = if is_today {
                        day_strip_palette.today_cell_bg
                    } else if is_weekend {
                        // Slightly different shade for weekend headers
                        blend_header_weekend(day_strip_palette.header_bg, theme.is_dark)
                    } else {
                        day_strip_palette.header_bg
                    };
                    let border_color = if is_today {
                        day_strip_palette.accent_line
                    } else {
                        day_strip_palette.strip_border
                    };
                    let name_color = if is_today {
                        day_strip_palette.today_text
                    } else {
                        day_strip_palette.header_text
                    };
                    let date_color = if is_today {
                        day_strip_palette.today_date_text
                    } else {
                        day_strip_palette.header_text
                    };

                    ui.allocate_ui_with_layout(
                        Vec2::new(col_width, 48.0),
                        egui::Layout::top_down(egui::Align::Center),
                        |cell_ui| {
                            egui::Frame::none()
                                .fill(cell_bg)
                                .rounding(egui::Rounding::same(6.0))
                                .stroke(Stroke::new(1.0, border_color))
                                .inner_margin(Margin::symmetric(6.0, 4.0))
                                .show(cell_ui, |content_ui| {
                                    content_ui.vertical_centered(|ui| {
                                        ui.label(
                                            egui::RichText::new(*day_name)
                                                .size(12.0)
                                                .color(name_color)
                                                .strong(),
                                        );

                                        ui.label(
                                            egui::RichText::new(format_short_date(
                                                date,
                                                &settings.date_format,
                                            ))
                                            .size(11.0)
                                            .color(date_color),
                                        );
                                    });
                                });
                        },
                    );

                    if i < day_names.len() - 1 {
                        ui.add_space(COLUMN_SPACING);
                    }
                }
            });

            // Ribbon row with all-day events
            if show_ribbon && !all_day_events.is_empty() {
                strip_ui.add_space(4.0);

                strip_ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    // Use allocate_exact_size for consistent spacing
                    ui.allocate_exact_size(Vec2::new(TIME_LABEL_WIDTH, 0.0), egui::Sense::hover());

                    ui.add_space(COLUMN_SPACING);

                    for (i, date) in week_dates.iter().enumerate() {
                        ui.vertical(|day_ui| {
                            day_ui.set_width(col_width);

                            let mut multi_day_events = Vec::new();
                            let mut single_day_events = Vec::new();

                            for event in all_day_events {
                                let event_start_date = event.start.date_naive();
                                let event_end_date = event.end.date_naive();

                                if event_start_date != event_end_date {
                                    if event_start_date <= *date && event_end_date >= *date {
                                        multi_day_events.push(event);
                                    }
                                } else if event_start_date == *date {
                                    single_day_events.push(event);
                                }
                            }

                            let found_event =
                                !multi_day_events.is_empty() || !single_day_events.is_empty();

                            for event in multi_day_events {
                                let ribbon_result = render_ribbon_event(
                                    day_ui,
                                    event,
                                    countdown_requests,
                                    active_countdown_events,
                                    database,
                                );
                                result.merge(ribbon_result);
                            }

                            for event in single_day_events {
                                let ribbon_result = render_ribbon_event(
                                    day_ui,
                                    event,
                                    countdown_requests,
                                    active_countdown_events,
                                    database,
                                );
                                result.merge(ribbon_result);
                            }

                            if !found_event {
                                day_ui.allocate_space(Vec2::new(col_width, 24.0));
                            }
                        });

                        if i < week_dates.len() - 1 {
                            ui.add_space(COLUMN_SPACING);
                        }
                    }
                });
            }
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
            .show(ui, |scroll_ui| {
                let available_width = scroll_ui.available_width();
                let available_for_cols = available_width - TIME_LABEL_WIDTH - total_spacing;
                let col_width = available_for_cols / 7.0;

                let config = TimeCellConfig {
                    drag_view: DragView::Week,
                    resize_view: ResizeView::Week,
                    check_weekend: true,
                };

                let grid_result = render_time_grid(
                    scroll_ui,
                    col_width,
                    &week_dates,
                    &events,
                    database,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_time,
                    event_dialog_recurrence,
                    countdown_requests,
                    active_countdown_events,
                    &grid_palette,
                    focus_request,
                    &config,
                );
                result.merge(grid_result);
            });

        result
    }

    pub(crate) fn get_week_start(date: NaiveDate, first_day_of_week: u8) -> NaiveDate {
        week_shared::get_week_start(date, first_day_of_week)
    }

    fn get_day_names(first_day_of_week: u8) -> Vec<&'static str> {
        let all_days = [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ];
        let start = first_day_of_week as usize;
        let mut result = Vec::with_capacity(7);
        for i in 0..7 {
            result.push(all_days[(start + i) % 7]);
        }
        result
    }

    fn get_events_for_week(event_service: &EventService, week_start: NaiveDate) -> Vec<Event> {
        let start = Local
            .from_local_datetime(&week_start.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let week_end = week_start + Duration::days(6);
        let end = Local
            .from_local_datetime(&week_end.and_hms_opt(23, 59, 59).unwrap())
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
