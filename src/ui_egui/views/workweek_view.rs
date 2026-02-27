use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Weekday};
use egui::{Color32, Margin, Stroke, Vec2};
use std::collections::HashSet;

use super::palette::DayStripPalette;
use super::week_shared::{
    format_short_date, get_week_start, render_ribbon_event, render_ribbon_event_with_handles,
    render_time_grid, EventInteractionResult, TimeCellConfig, COLUMN_SPACING, TIME_LABEL_WIDTH,
};
use super::{
    filter_events_by_category, filter_events_by_sync_scope, load_synced_event_ids,
    AutoFocusRequest, CountdownRequest,
};
use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::drag::DragView;
use crate::ui_egui::resize::{ResizeManager, ResizeView};
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

pub struct WorkWeekView;

impl WorkWeekView {
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
        show_ribbon: bool,
        all_day_events: &[Event],
        focus_request: &mut Option<AutoFocusRequest>,
        category_filter: Option<&str>,
        synced_only: bool,
        synced_source_id: Option<i64>,
    ) -> EventInteractionResult {
        let mut result = EventInteractionResult::default();
        let today = Local::now().date_naive();
        let day_strip_palette = DayStripPalette::from_theme(theme);
        let grid_palette = super::palette::TimeGridPalette::from_theme(theme);

        // Get work week dates based on settings
        let week_start = get_week_start(*current_date, settings.first_day_of_week);
        let work_week_dates = Self::get_work_week_dates(week_start, settings);

        // Get events for the work week
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_dates(&event_service, &work_week_dates);
        let events = filter_events_by_category(events, category_filter);
        let events = filter_events_by_sync_scope(events, database, synced_only, synced_source_id);
        let synced_event_ids = load_synced_event_ids(database, None);

        // Calculate column width accounting for scrollbar (16px typical)
        let scrollbar_width = 16.0;
        let num_days = work_week_dates.len();
        let total_spacing = COLUMN_SPACING * (num_days - 1) as f32;
        let available_for_cols =
            ui.available_width() - TIME_LABEL_WIDTH - total_spacing - scrollbar_width;
        let col_width = available_for_cols / num_days as f32;

        // Work week header with day names
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

        let show_week_numbers = settings.show_week_numbers;

        let header_response = header_frame.show(ui, |strip_ui| {
            strip_ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                // Time label placeholder - show week number if enabled
                // Use allocate_exact_size to ensure the space is reserved even when empty
                let (rect, _response) =
                    ui.allocate_exact_size(Vec2::new(TIME_LABEL_WIDTH, 48.0), egui::Sense::hover());
                if show_week_numbers {
                    if let Some(first_date) = work_week_dates.first() {
                        let week_num = first_date.iso_week().week();
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("W{}", week_num),
                            egui::FontId::proportional(12.0),
                            day_strip_palette.header_text,
                        );
                    }
                }

                ui.add_space(COLUMN_SPACING);

                for (i, date) in work_week_dates.iter().enumerate() {
                    let is_today = *date == today;
                    let weekday_idx = date.weekday().num_days_from_sunday();
                    let is_weekend = weekday_idx == 0 || weekday_idx == 6;
                    let day_name = date.format("%A").to_string();
                    
                    // Use header colors for day header cells
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
                                            egui::RichText::new(&day_name)
                                                .size(12.0)
                                                .color(name_color)
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format_short_date(
                                                *date,
                                                &settings.date_format,
                                            ))
                                            .size(11.0)
                                            .color(date_color),
                                        );
                                    });
                                });
                        },
                    );

                    if i < work_week_dates.len() - 1 {
                        ui.add_space(COLUMN_SPACING);
                    }
                }
            });

            let ribbon_lanes = super::build_ribbon_lanes(all_day_events);
            if show_ribbon && !ribbon_lanes.is_empty() {
                strip_ui.add_space(2.0);

                strip_ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    let ribbon_height = (ribbon_lanes.len() as f32 * 18.0).max(18.0);

                    // Use allocate_exact_size with matching height for consistent spacing
                    ui.allocate_exact_size(Vec2::new(TIME_LABEL_WIDTH, ribbon_height), egui::Sense::hover());

                    ui.add_space(COLUMN_SPACING);

                    for (i, date) in work_week_dates.iter().enumerate() {
                        // Use Min alignment so ribbon events fill the full column width
                        let col_response = ui.allocate_ui_with_layout(
                            Vec2::new(col_width, ribbon_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |day_ui| {
                                day_ui.spacing_mut().item_spacing.y = 0.0;

                                for lane in &ribbon_lanes {
                                    day_ui.allocate_ui_with_layout(
                                        Vec2::new(col_width, 18.0),
                                        egui::Layout::left_to_right(egui::Align::Min),
                                        |lane_ui| {
                                            if let Some(event) = lane
                                                .iter()
                                                .copied()
                                                .find(|event| super::event_covers_date(event, *date))
                                            {
                                                let event_start_date = event.start.date_naive();
                                                let event_end_date = super::event_display_end_date(event);

                                                if event_start_date != event_end_date {
                                                    let is_first_day = event_start_date == *date;
                                                    let is_last_day = event_end_date == *date;

                                                    let (ribbon_result, _event_rect) = render_ribbon_event_with_handles(
                                                        lane_ui,
                                                        event,
                                                        countdown_requests,
                                                        active_countdown_events,
                                                        database,
                                                        &synced_event_ids,
                                                        is_first_day,
                                                        is_last_day,
                                                        Some(*date),
                                                    );
                                                    result.merge(ribbon_result);
                                                } else {
                                                    let ribbon_result = render_ribbon_event(
                                                        lane_ui,
                                                        event,
                                                        countdown_requests,
                                                        active_countdown_events,
                                                        database,
                                                        &synced_event_ids,
                                                    );
                                                    result.merge(ribbon_result);
                                                }
                                            }
                                        },
                                    );
                                }
                            },
                        );
                        
                        // Track resize hover for this column using the response rect
                        let col_rect = col_response.response.rect;
                        if ResizeManager::is_active_for_view(ui.ctx(), ResizeView::Ribbon) {
                            if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                                if col_rect.contains(pointer_pos) {
                                    // Update hovered date for ribbon resize
                                    // Use midnight as time since we're resizing dates, not times
                                    let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
                                    let end_of_day = NaiveTime::from_hms_opt(23, 59, 59).unwrap();
                                    ResizeManager::update_hover(
                                        ui.ctx(),
                                        *date,
                                        midnight,
                                        end_of_day,
                                        pointer_pos,
                                    );
                                }
                            }
                        }

                        if i < work_week_dates.len() - 1 {
                            ui.add_space(COLUMN_SPACING);
                        }
                    }
                    
                    // Handle ribbon resize completion (mouse release)
                    let primary_released = ui.input(|i| i.pointer.primary_released());
                    if primary_released && ResizeManager::is_active_for_view(ui.ctx(), ResizeView::Ribbon) {
                        if let Some(resize_ctx) = ResizeManager::finish_for_view(ui.ctx(), ResizeView::Ribbon) {
                            log::info!(
                                "Ribbon resize finished: handle={:?}, hovered_date={:?}",
                                resize_ctx.handle,
                                resize_ctx.hovered_date
                            );
                            // Calculate new dates based on handle
                            if let (Some(new_start), Some(mut new_end)) = (resize_ctx.calculate_new_start(), resize_ctx.calculate_new_end()) {
                                let event_service = EventService::new(database.connection());
                                if let Ok(Some(mut event)) = event_service.get(resize_ctx.event_id) {
                                    // For all-day events, convert hovered inclusive
                                    // end date to exclusive end (iCal convention).
                                    if event.all_day {
                                        if let Some(next) = new_end.date_naive().succ_opt() {
                                            let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
                                            if let Some(dt) = next
                                                .and_time(midnight)
                                                .and_local_timezone(Local)
                                                .single()
                                            {
                                                new_end = dt;
                                            }
                                        }
                                    }
                                    log::info!("New dates: start={}, end={}", new_start, new_end);
                                    event.start = new_start;
                                    event.end = new_end;
                                    if let Err(err) = event_service.update(&event) {
                                        log::error!(
                                            "Failed to resize ribbon event {}: {}",
                                            resize_ctx.event_id, err
                                        );
                                    } else {
                                        result.moved_events.push(event);
                                    }
                                }
                            }
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
                let config = TimeCellConfig {
                    drag_view: DragView::WorkWeek,
                    resize_view: ResizeView::WorkWeek,
                    check_weekend: false, // WorkWeek doesn't highlight weekends differently
                };

                let grid_result = render_time_grid(
                    scroll_ui,
                    col_width,
                    &work_week_dates,
                    &events,
                    database,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_time,
                    event_dialog_recurrence,
                    countdown_requests,
                    active_countdown_events,
                    &synced_event_ids,
                    &grid_palette,
                    focus_request,
                    &config,
                );
                result.merge(grid_result);
            });

        result
    }

    pub(crate) fn get_week_start(date: NaiveDate, first_day_of_week: u8) -> NaiveDate {
        get_week_start(date, first_day_of_week)
    }

    pub(crate) fn get_work_week_dates(
        week_start: NaiveDate,
        settings: &Settings,
    ) -> Vec<NaiveDate> {
        let first_day = Self::weekday_from_num(settings.first_day_of_work_week);
        let last_day = Self::weekday_from_num(settings.last_day_of_work_week);

        let first_num = first_day.num_days_from_monday();
        let last_num = last_day.num_days_from_monday();

        let mut dates = Vec::new();
        for i in 0..7 {
            let date = week_start + Duration::days(i);
            let day_num = date.weekday().num_days_from_monday();

            if first_num <= last_num {
                if day_num >= first_num && day_num <= last_num {
                    dates.push(date);
                }
            } else if day_num >= first_num || day_num <= last_num {
                dates.push(date);
            }
        }

        dates
    }

    fn get_events_for_dates(event_service: &EventService, dates: &[NaiveDate]) -> Vec<Event> {
        if dates.is_empty() {
            return Vec::new();
        }

        let start = Local
            .from_local_datetime(&dates[0].and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let end = Local
            .from_local_datetime(&dates[dates.len() - 1].and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();

        event_service
            .expand_recurring_events(start, end)
            .unwrap_or_default()
            .into_iter()
            .filter(|e| !super::is_ribbon_event(e))
            .collect()
    }

    fn weekday_from_num(n: u8) -> Weekday {
        match n {
            0 => Weekday::Sun,
            1 => Weekday::Mon,
            2 => Weekday::Tue,
            3 => Weekday::Wed,
            4 => Weekday::Thu,
            5 => Weekday::Fri,
            6 => Weekday::Sat,
            _ => Weekday::Mon,
        }
    }
}
