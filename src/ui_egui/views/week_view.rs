use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;

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
    ) -> Option<Event> {
        let today = Local::now().date_naive();
        
        // Get week start based on settings
        let week_start = Self::get_week_start(*current_date, settings.first_day_of_week);
        let week_dates: Vec<NaiveDate> = (0..7)
            .map(|i| week_start + Duration::days(i))
            .collect();
        
        // Get events for the week
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_week(&event_service, week_start);
        
        // Calculate column width accounting for scrollbar (16px typical)
        let day_names = Self::get_day_names(settings.first_day_of_week);
        let scrollbar_width = 16.0;
        let time_label_width = 50.0;
        let spacing = 2.0;
        let total_spacing = spacing * 6.0; // 6 gaps between 7 columns
        let available_for_cols = ui.available_width() - time_label_width - total_spacing - scrollbar_width;
        let col_width = available_for_cols / 7.0;
        
        // Week header with day names
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0; // We'll add spacing manually
            
            // Fixed width for time label area
            ui.add_space(time_label_width);
            ui.add_space(spacing);
            
            for (i, day_name) in day_names.iter().enumerate() {
                let date = week_dates[i];
                let is_today = date == today;
                
                // Allocate exact width for column
                ui.allocate_ui_with_layout(
                    Vec2::new(col_width, 40.0),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        let text = egui::RichText::new(*day_name).size(12.0);
                        let text = if is_today {
                            text.color(Color32::from_rgb(100, 150, 255)).strong()
                        } else {
                            text
                        };
                        ui.label(text);
                        
                        let date_text = egui::RichText::new(Self::format_short_date(date, &settings.date_format)).size(11.0);
                        let date_text = if is_today {
                            date_text.color(Color32::from_rgb(100, 150, 255))
                        } else {
                            date_text.color(Color32::GRAY)
                        };
                        ui.label(date_text);
                    },
                );
                
                // Add spacing between columns (but not after the last one)
                if i < day_names.len() - 1 {
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
                    &week_dates,
                    &events,
                    database,
                    settings,
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
        week_dates: &[NaiveDate],
        events: &[Event],
        database: &'static crate::services::database::Database,
        _settings: &Settings,
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
                                        .color(Color32::GRAY)
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
                        if day_idx < week_dates.len() - 1 {
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
        starting_events: &[&Event],  // Events that start in this slot
        continuing_events: &[&Event], // Events continuing through this slot
        database: &'static crate::services::database::Database,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
    ) -> Option<Event> {
        let today = Local::now().date_naive();
        let is_today = date == today;
        let is_weekend = date.weekday().num_days_from_sunday() == 0
            || date.weekday().num_days_from_sunday() == 6;
        
        let desired_size = Vec2::new(col_width, 30.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
        
        // Background
        let bg_color = if is_today {
            Color32::from_rgb(50, 70, 100)
        } else if is_weekend {
            Color32::from_gray(35)
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
            ui.painter().rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(100, 150, 255, 30));
        }
        
        // Draw continuing events first (colored blocks only)
        for event in continuing_events {
            Self::render_event_continuation(ui, rect, event);
        }
        
        // Draw starting events (full details)
        for event in starting_events {
            Self::render_event_in_cell(ui, rect, event);
        }
        
        // Context menu for right-click - use Rc<RefCell<>> to capture the event
        let context_clicked_event: Rc<RefCell<Option<Event>>> = Rc::new(RefCell::new(None));
        let context_clicked_event_clone = context_clicked_event.clone();
        
        response.context_menu(|ui| {
            // Only show menu if there's an event in this cell
            if let Some(event) = starting_events.first().or_else(|| continuing_events.first()) {
                ui.label(format!("Event: {}", event.title));
                ui.separator();
                
                if ui.button("✏ Edit").clicked() {
                    *context_clicked_event_clone.borrow_mut() = Some((*event).clone());
                    ui.close_menu();
                }
                
                if ui.button("🗑 Delete").clicked() {
                    // Delete the event immediately
                    use crate::services::event::EventService;
                    if let Some(id) = event.id {
                        let service = EventService::new(database.connection());
                        let _ = service.delete(id);
                    }
                    ui.close_menu();
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
                    ui.close_menu();
                }
                
                if ui.button("🔄 New Recurring Event").clicked() {
                    *show_event_dialog = true;
                    *event_dialog_date = Some(date);
                    *event_dialog_time = Some(time);
                    *event_dialog_recurrence = Some("FREQ=WEEKLY".to_string());
                    ui.close_menu();
                }
            }
        });
        
        // Check if context menu returned a clicked event
        let mut clicked_event: Option<Event> = context_clicked_event.borrow().clone();
        
        // Check if user clicked on an event (left click)
        if clicked_event.is_none() && response.clicked() {
            if let Some(_pos) = response.interact_pointer_pos() {
                // Check if click was on an event (any event fills the cell, so any click is on an event if events exist)
                if let Some(event) = starting_events.first() {
                    clicked_event = Some((*event).clone());
                } else if let Some(event) = continuing_events.first() {
                    clicked_event = Some((*event).clone());
                } else {
                    // Click on empty space - create new event
                    *show_event_dialog = true;
                    *event_dialog_date = Some(date);
                    *event_dialog_time = Some(time); // Use the clicked time slot
                    *event_dialog_recurrence = None; // Default to non-recurring
                }
            }
        }
        
        // Handle double-click for recurring event
        if response.double_clicked() {
            *show_event_dialog = true;
            *event_dialog_date = Some(date);
            *event_dialog_time = Some(time); // Use the clicked time slot
            *event_dialog_recurrence = Some("FREQ=WEEKLY".to_string());
        }
        
        clicked_event
    }
    
    fn render_event_in_cell(ui: &mut egui::Ui, cell_rect: Rect, event: &Event) {
        let event_color = event.color.as_deref()
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
    }
    
    fn render_event_continuation(ui: &mut egui::Ui, cell_rect: Rect, event: &Event) {
        let event_color = event.color.as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));
        
        // Just render a lighter colored background to show the event continues
        let bg_rect = Rect::from_min_size(
            Pos2::new(cell_rect.left() + 2.0, cell_rect.top() + 2.0),
            Vec2::new(cell_rect.width() - 4.0, cell_rect.height() - 4.0),
        );
        ui.painter().rect_filled(bg_rect, 2.0, event_color.linear_multiply(0.5));
    }
    
    fn get_week_start(date: NaiveDate, first_day_of_week: u8) -> NaiveDate {
        let weekday = date.weekday().num_days_from_sunday() as i64;
        let offset = (weekday - first_day_of_week as i64 + 7) % 7;
        date - Duration::days(offset)
    }
    
    fn get_day_names(first_day_of_week: u8) -> Vec<&'static str> {
        let all_days = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
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
        use chrono::{TimeZone, Local};
        
        let start = Local.from_local_datetime(&week_start.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let week_end = week_start + Duration::days(6);
        let end = Local.from_local_datetime(&week_end.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();
        
        event_service
            .expand_recurring_events(start, end)
            .unwrap_or_default()
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
