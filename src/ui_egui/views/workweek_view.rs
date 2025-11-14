use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime, Weekday};
use egui::{Color32, CursorIcon, Pos2, Rect, Sense, Stroke, Vec2};

use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::drag::{DragContext, DragManager, DragView};

pub struct WorkWeekView;

impl WorkWeekView {
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        database: &'static Database,
        settings: &Settings,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
    ) -> Option<Event> {
        let today = Local::now().date_naive();

        // Get work week dates based on settings
        let week_start = Self::get_week_start(*current_date, settings.first_day_of_week);
        let work_week_dates = Self::get_work_week_dates(week_start, settings);

        // Get events for the work week
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_dates(&event_service, &work_week_dates);

        // Calculate column width accounting for scrollbar (16px typical)
        let scrollbar_width = 16.0;
        let time_label_width = 50.0;
        let spacing = 2.0;
        let num_days = work_week_dates.len();
        let total_spacing = spacing * (num_days - 1) as f32; // n-1 gaps between n columns
        let available_for_cols =
            ui.available_width() - time_label_width - total_spacing - scrollbar_width;
        let col_width = available_for_cols / num_days as f32;

        // Work week header with day names
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0; // We'll add spacing manually

            // Fixed width for time label area
            ui.add_space(time_label_width);
            ui.add_space(spacing);

            for (i, date) in work_week_dates.iter().enumerate() {
                let is_today = *date == today;
                let day_name = date.format("%A").to_string();

                // Allocate exact width for column
                ui.allocate_ui_with_layout(
                    Vec2::new(col_width, 40.0),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        let text = egui::RichText::new(&day_name).size(12.0);
                        let text = if is_today {
                            text.color(Color32::from_rgb(100, 150, 255)).strong()
                        } else {
                            text
                        };
                        ui.label(text);

                        let date_text = egui::RichText::new(Self::format_short_date(
                            *date,
                            &settings.date_format,
                        ))
                        .size(11.0);
                        let date_text = if is_today {
                            date_text.color(Color32::from_rgb(100, 150, 255))
                        } else {
                            date_text.color(Color32::GRAY)
                        };
                        ui.label(date_text);
                    },
                );

                // Add spacing between columns (but not after the last one)
                if i < work_week_dates.len() - 1 {
                    ui.add_space(spacing);
                }
            }
        });

        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);

        // Scrollable time slots
        let mut clicked_event = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(event) = Self::render_time_grid(
                    ui,
                    col_width,
                    &work_week_dates,
                    &events,
                    settings,
                    database,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_time,
                    event_dialog_recurrence,
                ) {
                    clicked_event = Some(event);
                }
            });

        clicked_event
    }

    fn render_time_grid(
        ui: &mut egui::Ui,
        col_width: f32,
        work_week_dates: &[NaiveDate],
        events: &[Event],
        _settings: &Settings,
        database: &'static Database,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
    ) -> Option<Event> {
        // Always render 15-minute intervals (4 slots per hour)
        const SLOT_INTERVAL: i64 = 15;

        let time_label_width = 50.0;
        let spacing = 2.0;

        let mut clicked_event: Option<Event> = None;

        // Draw 24 hours with 4 slots each
        for hour in 0..24 {
            for slot in 0..4 {
                let minute = slot * SLOT_INTERVAL;
                let time = NaiveTime::from_hms_opt(hour as u32, minute as u32, 0).unwrap();
                let is_hour_start = slot == 0;

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0; // We'll add spacing manually

                    // Time label with fixed width (only on hour starts)
                    ui.allocate_ui_with_layout(
                        Vec2::new(time_label_width, 30.0),
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

                    ui.add_space(spacing);

                    // Day columns with exact width
                    for (day_idx, date) in work_week_dates.iter().enumerate() {
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
                            let event_date = event.start.date_naive();
                            if event_date != *date {
                                continue;
                            }

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

                        if let Some(event) = Self::render_time_cell(
                            ui,
                            col_width,
                            *date,
                            time,
                            is_hour_start,
                            &starting_events,
                            &continuing_events,
                            database,
                            show_event_dialog,
                            event_dialog_date,
                            event_dialog_time,
                            event_dialog_recurrence,
                        ) {
                            clicked_event = Some(event);
                        }

                        // Add spacing between columns (but not after the last one)
                        if day_idx < work_week_dates.len() - 1 {
                            ui.add_space(spacing);
                        }
                    }
                });
            }
        }

        clicked_event
    }

    fn render_time_cell(
        ui: &mut egui::Ui,
        col_width: f32,
        date: NaiveDate,
        time: NaiveTime,
        is_hour_start: bool,
        starting_events: &[&Event],   // Events that start in this slot
        continuing_events: &[&Event], // Events continuing through this slot
        database: &'static Database,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
    ) -> Option<Event> {
        let today = Local::now().date_naive();
        let is_today = date == today;

        let desired_size = Vec2::new(col_width, 30.0);
        // Use union of click and hover to capture both left and right clicks
        let drag_sense = Sense::click_and_drag().union(Sense::hover());
        let (rect, response) = ui.allocate_exact_size(desired_size, drag_sense);

        // Background
        let bg_color = if is_today {
            Color32::from_rgb(50, 70, 100)
        } else {
            Color32::from_gray(40)
        };
        ui.painter().rect_filled(rect, 0.0, bg_color);

        // Horizontal grid line
        let line_color = if is_hour_start {
            Color32::from_gray(60)
        } else {
            Color32::from_gray(50)
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
            Stroke::new(1.0, Color32::from_gray(50)),
        );

        // Hover effect
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                0.0,
                Color32::from_rgba_unmultiplied(100, 150, 255, 30),
            );
        }

        let mut event_hitboxes: Vec<(Rect, Event)> = Vec::new();

        // Draw continuing events first (colored blocks only)
        for event in continuing_events {
            let event_rect = Self::render_event_continuation(ui, rect, event);
            event_hitboxes.push((event_rect, (*event).clone()));
        }

        // Draw starting events (full details)
        for event in starting_events {
            let event_rect = Self::render_event_in_cell(ui, rect, event);
            event_hitboxes.push((event_rect, (*event).clone()));
        }

        let pointer_pos = response.interact_pointer_pos();
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

        let pointer_for_hover = ui
            .ctx()
            .pointer_interact_pos()
            .or_else(|| ui.input(|i| i.pointer.hover_pos()));
        if let Some(pointer) = pointer_for_hover {
            if rect.contains(pointer) {
                DragManager::update_hover(ui.ctx(), date, time, rect, pointer);
                if DragManager::is_active_for_view(ui.ctx(), DragView::WorkWeek) {
                    ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                    ui.ctx().request_repaint();
                }
            }
        }

        if let Some(drag_state) = DragManager::active_for_view(ui.ctx(), DragView::WorkWeek) {
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

        // Manual context menu handling - store popup state in egui memory
        let mut context_clicked_event: Option<Event> = None;
        let mut context_menu_event: Option<Event> = None;
        let popup_id = response
            .id
            .with(format!("context_menu_{}_{:?}", date, time));

        // Open popup on right-click
        let show_context_menu = response.secondary_clicked();
        if show_context_menu {
            context_menu_event = pointer_event.clone();
            ui.memory_mut(|mem| mem.open_popup(popup_id));
        }

        // Show popup if it's open
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

                // Only show menu if there's an event in this cell
                if let Some(event) = popup_event {
                    ui.label(format!("Event: {}", event.title));
                    ui.separator();

                    if ui.button("✏ Edit").clicked() {
                        context_clicked_event = Some(event.clone());
                        ui.memory_mut(|mem| mem.close_popup());
                    }

                    if ui.button("🗑 Delete").clicked() {
                        // Delete the event immediately
                        use crate::services::event::EventService;
                        if let Some(id) = event.id {
                            let service = EventService::new(database.connection());
                            let _ = service.delete(id);
                        }
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                } else {
                    // Right-click on empty space
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

        // Check if context menu returned a clicked event
        let mut clicked_event: Option<Event> = context_clicked_event;

        if clicked_event.is_none() && response.clicked() {
            if let Some(event) = pointer_event.clone() {
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

        if response.drag_started() {
            if let Some((hit_rect, event)) = pointer_hit {
                if event.recurrence_rule.is_none() {
                    if let Some(drag_context) = DragContext::from_event(
                        &event,
                        pointer_pos
                            .map(|pos| pos - hit_rect.min)
                            .unwrap_or(Vec2::ZERO),
                        DragView::WorkWeek,
                    ) {
                        DragManager::begin(ui.ctx(), drag_context);
                        ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                    }
                }
            }
        }

        if response.dragged() {
            ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
        }

        if response.drag_stopped() {
            if let Some(drag_context) = DragManager::finish_for_view(ui.ctx(), DragView::WorkWeek) {
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

    fn render_event_in_cell(ui: &mut egui::Ui, cell_rect: Rect, event: &Event) -> Rect {
        let event_color = event
            .color
            .as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));

        // Event indicator bar - fills the cell
        let bar_rect = Rect::from_min_size(
            Pos2::new(cell_rect.left() + 2.0, cell_rect.top() + 2.0),
            Vec2::new(cell_rect.width() - 4.0, cell_rect.height() - 4.0),
        );
        ui.painter().rect_filled(bar_rect, 2.0, event_color);

        // Event title - use available width with proper truncation
        let font_id = egui::FontId::proportional(10.0);
        let available_width = cell_rect.width() - 10.0; // 5px padding on each side

        // Use egui's layout system to properly truncate text
        let layout_job = egui::text::LayoutJob::simple(
            event.title.clone(),
            font_id.clone(),
            Color32::WHITE,
            available_width,
        );

        let galley = ui.fonts(|f| f.layout_job(layout_job));

        ui.painter().galley(
            Pos2::new(cell_rect.left() + 5.0, cell_rect.top() + 5.0),
            galley,
            Color32::WHITE,
        );

        bar_rect
    }

    fn render_event_continuation(ui: &mut egui::Ui, cell_rect: Rect, event: &Event) -> Rect {
        let event_color = event
            .color
            .as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));

        // Just render a lighter colored background to show the event continues
        let bg_rect = Rect::from_min_size(
            Pos2::new(cell_rect.left() + 2.0, cell_rect.top() + 2.0),
            Vec2::new(cell_rect.width() - 4.0, cell_rect.height() - 4.0),
        );
        ui.painter()
            .rect_filled(bg_rect, 2.0, event_color.linear_multiply(0.5));

        bg_rect
    }

    fn get_week_start(date: NaiveDate, first_day_of_week: u8) -> NaiveDate {
        let weekday = date.weekday().num_days_from_sunday() as i64;
        let offset = (weekday - first_day_of_week as i64 + 7) % 7;
        date - Duration::days(offset)
    }

    fn get_work_week_dates(week_start: NaiveDate, settings: &Settings) -> Vec<NaiveDate> {
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
            } else {
                if day_num >= first_num || day_num <= last_num {
                    dates.push(date);
                }
            }
        }

        dates
    }

    fn get_events_for_dates(event_service: &EventService, dates: &[NaiveDate]) -> Vec<Event> {
        use chrono::{Local, TimeZone};

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

    fn format_short_date(date: NaiveDate, date_format: &str) -> String {
        // Parse date_format setting and return appropriate short format
        if date_format.starts_with("DD/MM") || date_format.starts_with("dd/mm") {
            date.format("%d/%m").to_string()
        } else if date_format.starts_with("YYYY") || date_format.starts_with("yyyy") {
            date.format("%Y/%m/%d").to_string()
        } else {
            // Default to MM/DD for US format
            date.format("%m/%d").to_string()
        }
    }

    fn parse_color(hex: &str) -> Option<Color32> {
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
}
