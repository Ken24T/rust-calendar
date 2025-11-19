use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime, Timelike};
use egui::{Color32, CursorIcon, Margin, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashSet;

use super::palette::DayStripPalette;
use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::drag::{DragContext, DragManager, DragView};
use crate::ui_egui::views::{event_time_segment_for_date, CountdownRequest};

pub struct WeekView;

impl WeekView {
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        database: &'static Database,
        settings: &Settings,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        show_ribbon: bool,
        all_day_events: &[Event],
    ) -> Option<Event> {
        let today = Local::now().date_naive();
        let day_strip_palette = DayStripPalette::from_ui(ui);

        // Get week start based on settings
        let week_start = Self::get_week_start(*current_date, settings.first_day_of_week);
        let week_dates: Vec<NaiveDate> = (0..7).map(|i| week_start + Duration::days(i)).collect();

        // Get events for the week
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_week(&event_service, week_start);

        let day_names = Self::get_day_names(settings.first_day_of_week);
        let time_label_width = 50.0;
        let spacing = 2.0;
        let total_spacing = spacing * 6.0; // 6 gaps between 7 columns

        // Week header with day names
        let header_frame = egui::Frame::none()
            .fill(day_strip_palette.strip_bg)
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
            let frame_available_for_cols = frame_available_width - time_label_width - total_spacing;
            let col_width = frame_available_for_cols / 7.0;
            
            // Header row with day names
            strip_ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                // Time label placeholder - use allocate_ui_with_layout like the time grid does
                ui.allocate_ui_with_layout(
                    Vec2::new(time_label_width, 48.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |_ui| {
                        // Empty placeholder to match time grid spacing
                    },
                );
                
                ui.add_space(spacing);

                for (i, day_name) in day_names.iter().enumerate() {
                    let date = week_dates[i];
                    let is_today = date == today;
                    let cell_bg = if is_today {
                        day_strip_palette.today_cell_bg
                    } else {
                        day_strip_palette.cell_bg
                    };
                    let border_color = if is_today {
                        day_strip_palette.accent_line
                    } else {
                        day_strip_palette.strip_border
                    };
                    let name_color = if is_today {
                        day_strip_palette.today_text
                    } else {
                        day_strip_palette.text
                    };
                    let date_color = if is_today {
                        day_strip_palette.today_date_text
                    } else {
                        day_strip_palette.date_text
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
                                            egui::RichText::new(Self::format_short_date(
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

                    // Add spacing between columns (but not after the last one)
                    if i < day_names.len() - 1 {
                        ui.add_space(spacing);
                    }
                }
            });

            // Ribbon row with all-day events (inside same frame, using same col_width)
            if show_ribbon && !all_day_events.is_empty() {
                strip_ui.add_space(4.0);
                
                strip_ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    // Time label placeholder - same width as header
                    ui.allocate_ui_with_layout(
                        Vec2::new(time_label_width, 24.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |_ui| {},
                    );
                    
                    ui.add_space(spacing);

                    // Render each day column - same loop structure as header
                    for (i, date) in week_dates.iter().enumerate() {
                        ui.allocate_ui_with_layout(
                            Vec2::new(col_width, 24.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |day_ui| {
                                // Find event for this specific date
                                for event in all_day_events {
                                    let event_start_date = event.start.date_naive();
                                    let event_end_date = event.end.date_naive();
                                    
                                    if event_start_date <= *date && event_end_date >= *date {
                                        Self::render_ribbon_event(day_ui, event, countdown_requests, active_countdown_events);
                                    }
                                }
                            },
                        );

                        // Add spacing between columns (but not after the last one)
                        if i < week_dates.len() - 1 {
                            ui.add_space(spacing);
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
        let mut clicked_event = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |scroll_ui| {
                // Calculate column width for time grid
                let available_width = scroll_ui.available_width();
                let available_for_cols = available_width - time_label_width - total_spacing;
                let col_width = available_for_cols / 7.0;
                
                if let Some(event) = Self::render_time_grid(
                    scroll_ui,
                    col_width,
                    &week_dates,
                    &events,
                    database,
                    settings,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_time,
                    event_dialog_recurrence,
                    countdown_requests,
                    active_countdown_events,
                ) {
                    clicked_event = Some(event);
                }
            });

        clicked_event
    }

    fn render_ribbon_event(
        ui: &mut egui::Ui,
        event: &Event,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
    ) {
        let event_color = event
            .color
            .as_deref()
            .and_then(|c| Self::parse_color(c))
            .unwrap_or(Color32::from_rgb(100, 150, 200));

        // Set max width to prevent overflow
        ui.set_max_width(ui.available_width());
        
        let event_frame = egui::Frame::none()
            .fill(event_color)
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(8.0, 4.0));

        let response = event_frame.show(ui, |ui| {
            ui.set_max_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&event.title).color(Color32::WHITE).size(13.0));
                
                // Show date range if multi-day
                if event.start.date_naive() != event.end.date_naive() {
                    ui.label(
                        egui::RichText::new(format!(
                            "({} - {})",
                            event.start.format("%b %d"),
                            event.end.format("%b %d")
                        ))
                        .color(Color32::from_gray(220))
                        .size(11.0)
                    );
                }
            });
        }).response;

        // Context menu on right-click
        response.context_menu(|ui| {
            ui.set_min_width(150.0);

            if event.start > chrono::Local::now() {
                let timer_exists = event
                    .id
                    .map(|id| active_countdown_events.contains(&id))
                    .unwrap_or(false);
                if timer_exists {
                    ui.label(
                        egui::RichText::new("Countdown already exists")
                            .italics()
                            .color(Color32::from_gray(150))
                            .size(11.0),
                    );
                } else if ui.button("⏱ Create Countdown").clicked() {
                    countdown_requests.push(CountdownRequest::from_event(event));
                    ui.close_menu();
                }
            }
        });
    }

    fn render_time_grid(
        ui: &mut egui::Ui,
        col_width: f32,
        week_dates: &[NaiveDate],
        events: &[Event],
        database: &'static Database,
        _settings: &Settings,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
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
                    for (day_idx, date) in week_dates.iter().enumerate() {
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
                            countdown_requests,
                            active_countdown_events,
                        ) {
                            clicked_event = Some(event);
                        }

                        // Add spacing between columns (but not after the last one)
                        if day_idx < week_dates.len() - 1 {
                            ui.add_space(spacing);
                        }
                    }
                });
            }
        }

        // Draw current time indicator
        let now = Local::now();
        let now_date = now.date_naive();
        let now_time = now.time();
        
        // Check if current time is within the week
        if let Some(day_index) = week_dates.iter().position(|d| *d == now_date) {
            // Calculate Y position based on time
            let hours_since_midnight = now_time.hour() as f32 + (now_time.minute() as f32 / 60.0);
            let slots_since_midnight = (hours_since_midnight * 4.0).floor() as usize; // 4 slots per hour
            let slot_offset = (hours_since_midnight * 4.0) - slots_since_midnight as f32;
            
            // Each slot is 30.0 pixels high, calculate relative Y
            let relative_y = (slots_since_midnight as f32 * 30.0) + (slot_offset * 30.0);
            
            // Get the UI's current position to calculate absolute coordinates
            let ui_top = ui.min_rect().top();
            let y_position = ui_top + relative_y;
            
            // Calculate X position for the day column
            let ui_left = ui.min_rect().left();
            let x_start = ui_left + time_label_width + spacing + (day_index as f32 * (col_width + spacing));
            let x_end = x_start + col_width;
            
            // Draw the indicator line
            let painter = ui.painter();
            let line_color = Color32::from_rgb(255, 100, 100); // Red indicator
            let circle_center = egui::pos2(x_start - 4.0, y_position);
            
            // Draw a small circle at the start
            painter.circle_filled(circle_center, 3.0, line_color);
            
            // Draw the horizontal line
            painter.line_segment(
                [egui::pos2(x_start, y_position), egui::pos2(x_end, y_position)],
                egui::Stroke::new(2.0, line_color),
            );
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
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
    ) -> Option<Event> {
        let today = Local::now().date_naive();
        let is_today = date == today;
        let is_weekend = date.weekday().num_days_from_sunday() == 0
            || date.weekday().num_days_from_sunday() == 6;

        let desired_size = Vec2::new(col_width, 30.0);
        // Use union of click and hover to capture both left and right clicks
        let drag_sense = Sense::click_and_drag().union(Sense::hover());
        let (rect, response) = ui.allocate_exact_size(desired_size, drag_sense);

        // Add manual context menu handling with secondary_clicked
        let show_context_menu = response.secondary_clicked();

        let dark_mode = ui.style().visuals.dark_mode;
        let (
            regular_bg,
            today_bg,
            weekend_bg,
            hour_line_color,
            slot_line_color,
            divider_color,
            hover_overlay,
        ) = if dark_mode {
            (
                Color32::from_gray(40),
                Color32::from_rgb(50, 70, 100),
                Color32::from_gray(35),
                Color32::from_gray(60),
                Color32::from_gray(50),
                Color32::from_gray(50),
                Color32::from_rgba_unmultiplied(100, 150, 255, 30),
            )
        } else {
            (
                Color32::from_rgb(245, 245, 245),
                Color32::from_rgb(222, 236, 255),
                Color32::from_rgb(235, 238, 244),
                Color32::from_rgb(210, 210, 210),
                Color32::from_rgb(230, 230, 230),
                Color32::from_rgb(210, 210, 210),
                Color32::from_rgba_unmultiplied(80, 120, 200, 25),
            )
        };

        // Background
        let bg_color = if is_today {
            today_bg
        } else if is_weekend {
            weekend_bg
        } else {
            regular_bg
        };
        ui.painter().rect_filled(rect, 0.0, bg_color);

        // Horizontal grid line
        let line_color = if is_hour_start {
            hour_line_color
        } else {
            slot_line_color
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
            Stroke::new(1.0, divider_color),
        );

        // Hover effect
        if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, hover_overlay);
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

        // Check for pointer position - use hover position to catch right-clicks too
        let pointer_pos = response.interact_pointer_pos()
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

        let pointer_for_hover = ui
            .ctx()
            .pointer_interact_pos()
            .or_else(|| ui.input(|i| i.pointer.hover_pos()));
        if let Some(pointer) = pointer_for_hover {
            if rect.contains(pointer) {
                DragManager::update_hover(ui.ctx(), date, time, rect, pointer);
                if DragManager::is_active_for_view(ui.ctx(), DragView::Week) {
                    ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                    ui.ctx().request_repaint();
                }
            }
        }

        if let Some(drag_state) = DragManager::active_for_view(ui.ctx(), DragView::Week) {
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

                if let Some(event) = popup_event {
                    ui.label(format!("Event: {}", event.title));
                    ui.separator();

                    if ui.button("✏ Edit").clicked() {
                        context_clicked_event = Some(event.clone());
                        ui.memory_mut(|mem| mem.close_popup());
                    }

                    // Delete options - different for recurring events
                    if event.recurrence_rule.is_some() {
                        if ui.button("🗑 Delete This Occurrence").clicked() {
                            use crate::services::event::EventService;
                            if let Some(id) = event.id {
                                let service = EventService::new(database.connection());
                                let _ = service.delete_occurrence(id, event.start);
                            }
                            ui.memory_mut(|mem| mem.close_popup());
                        }
                        if ui.button("🗑 Delete All Occurrences").clicked() {
                            use crate::services::event::EventService;
                            if let Some(id) = event.id {
                                let service = EventService::new(database.connection());
                                let _ = service.delete(id);
                            }
                            ui.memory_mut(|mem| mem.close_popup());
                        }
                    } else {
                        if ui.button("🗑 Delete").clicked() {
                            use crate::services::event::EventService;
                            if let Some(id) = event.id {
                                let service = EventService::new(database.connection());
                                let _ = service.delete(id);
                            }
                            ui.memory_mut(|mem| mem.close_popup());
                        }
                    }

                    if ui.button("📤 Export this event").clicked() {
                        // Export event to ICS file
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

                    if event.start > Local::now() {
                        let timer_exists = event
                            .id
                            .map(|id| active_countdown_events.contains(&id))
                            .unwrap_or(false);
                        if timer_exists {
                            ui.label(
                                egui::RichText::new("Countdown already exists")
                                    .italics()
                                    .color(Color32::from_gray(150))
                                    .size(11.0),
                            );
                        } else if ui.button("⏱ Create Countdown").clicked() {
                            countdown_requests.push(CountdownRequest::from_event(&event));
                            ui.memory_mut(|mem| mem.close_popup());
                        }
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

        let mut clicked_event: Option<Event> = context_clicked_event;

        // Check drag_started BEFORE clicked to ensure drag detection works
        if response.drag_started() {
            eprintln!("[week_view] drag_started detected, pointer_hit: {:?}", pointer_hit.as_ref().map(|(_, e)| &e.title));
            if let Some((hit_rect, event)) = pointer_hit {
                eprintln!("[week_view] event under pointer: {}", event.title);
                if event.recurrence_rule.is_none() {
                    if let Some(drag_context) = DragContext::from_event(
                        &event,
                        pointer_pos
                            .map(|pos| pos - hit_rect.min)
                            .unwrap_or(Vec2::ZERO),
                        DragView::Week,
                    ) {
                        eprintln!("[week_view] starting drag for event: {}", event.title);
                        DragManager::begin(ui.ctx(), drag_context);
                        ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
                    }
                } else {
                    eprintln!("[week_view] cannot drag recurring event: {}", event.title);
                }
            } else {
                eprintln!("[week_view] drag_started but no event under pointer");
            }
        } else if clicked_event.is_none() && response.clicked() {
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

        if response.dragged() {
            ui.output_mut(|out| out.cursor_icon = CursorIcon::Grabbing);
        }

        if response.drag_stopped() {
            if let Some(drag_context) = DragManager::finish_for_view(ui.ctx(), DragView::Week) {
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

    fn get_events_for_week(event_service: &EventService, week_start: NaiveDate) -> Vec<Event> {
        use chrono::{Local, TimeZone};

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
