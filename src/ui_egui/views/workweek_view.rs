use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime, Weekday};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;

pub struct WorkWeekView;

impl WorkWeekView {
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        database: &'static Database,
        settings: &Settings,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
    ) {
        let today = Local::now().date_naive();
        
        // Get work week dates based on settings
        let week_start = Self::get_week_start(*current_date);
        let work_week_dates = Self::get_work_week_dates(week_start, settings);
        
        // Get events for the work week
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_dates(&event_service, &work_week_dates);
        
        // Work week header with day names
        ui.horizontal(|ui| {
            ui.add_space(50.0); // Space for time labels
            
            for date in &work_week_dates {
                let is_today = *date == today;
                let day_name = date.format("%A").to_string();
                
                ui.vertical(|ui| {
                    let text = egui::RichText::new(&day_name).size(12.0);
                    let text = if is_today {
                        text.color(Color32::from_rgb(100, 150, 255)).strong()
                    } else {
                        text
                    };
                    ui.label(text);
                    
                    let date_text = egui::RichText::new(date.format("%m/%d").to_string()).size(11.0);
                    let date_text = if is_today {
                        date_text.color(Color32::from_rgb(100, 150, 255))
                    } else {
                        date_text.color(Color32::GRAY)
                    };
                    ui.label(date_text);
                });
                
                ui.add_space(5.0);
            }
        });
        
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
        
        // Scrollable time slots
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Self::render_time_grid(
                    ui,
                    &work_week_dates,
                    &events,
                    settings,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_recurrence,
                );
            });
    }
    
    fn render_time_grid(
        ui: &mut egui::Ui,
        work_week_dates: &[NaiveDate],
        events: &[Event],
        settings: &Settings,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
    ) {
        let time_slot_interval = settings.time_slot_interval as i64;
        let slots_per_hour = 60 / time_slot_interval;
        let num_days = work_week_dates.len();
        
        // Draw 24 hours
        for hour in 0..24 {
            for slot in 0..slots_per_hour {
                let minute = slot * time_slot_interval;
                let time = NaiveTime::from_hms_opt(hour as u32, minute as u32, 0).unwrap();
                let is_hour_start = slot == 0;
                
                ui.horizontal(|ui| {
                    // Time label (only on hour starts)
                    if is_hour_start {
                        let time_str = format!("{:02}:00", hour);
                        ui.label(
                            egui::RichText::new(time_str)
                                .size(12.0)
                                .color(Color32::GRAY)
                        );
                    } else {
                        ui.add_space(50.0);
                    }
                    
                    // Day columns
                    for (day_idx, date) in work_week_dates.iter().enumerate() {
                        // Find events for this day and time slot
                        let slot_events: Vec<&Event> = events.iter()
                            .filter(|e| {
                                let event_date = e.start.date_naive();
                                if event_date != *date {
                                    return false;
                                }
                                
                                let event_start = e.start.time();
                                let event_end = e.end.time();
                                
                                let slot_start = time;
                                let slot_end = NaiveTime::from_hms_opt(
                                    hour as u32,
                                    (minute + time_slot_interval) as u32 % 60,
                                    0,
                                ).unwrap();
                                
                                event_start < slot_end && event_end > slot_start
                            })
                            .collect();
                        
                        Self::render_time_cell(
                            ui,
                            *date,
                            time,
                            is_hour_start,
                            num_days,
                            &slot_events,
                            show_event_dialog,
                            event_dialog_date,
                            event_dialog_recurrence,
                        );
                        
                        if day_idx < num_days - 1 {
                            ui.add_space(2.0);
                        }
                    }
                });
            }
        }
    }
    
    fn render_time_cell(
        ui: &mut egui::Ui,
        date: NaiveDate,
        time: NaiveTime,
        is_hour_start: bool,
        num_days: usize,
        events: &[&Event],
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
    ) {
        let today = Local::now().date_naive();
        let is_today = date == today;
        
        let cell_width = (ui.available_width() - 30.0) / num_days as f32;
        let desired_size = Vec2::new(cell_width, 30.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
        
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
            ui.painter().rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(100, 150, 255, 30));
        }
        
        // Draw events
        for event in events {
            Self::render_event_in_cell(ui, rect, event);
        }
        
        // Handle click to create event
        if response.clicked() {
            *show_event_dialog = true;
            *event_dialog_date = Some(date);
            *event_dialog_recurrence = Some("FREQ=DAILY".to_string());
        }
        
        // Handle double-click
        if response.double_clicked() {
            *show_event_dialog = true;
            *event_dialog_date = Some(date);
            *event_dialog_recurrence = Some("FREQ=WEEKLY".to_string());
        }
    }
    
    fn render_event_in_cell(ui: &mut egui::Ui, cell_rect: Rect, event: &Event) {
        let event_color = event.color.as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));
        
        // Event indicator (small bar)
        let bar_rect = Rect::from_min_size(
            Pos2::new(cell_rect.left() + 2.0, cell_rect.top() + 2.0),
            Vec2::new(cell_rect.width() - 4.0, 4.0),
        );
        ui.painter().rect_filled(bar_rect, 1.0, event_color);
        
        // Event title (truncated)
        let title = if event.title.len() > 12 {
            format!("{}...", &event.title[..9])
        } else {
            event.title.clone()
        };
        
        ui.painter().text(
            Pos2::new(cell_rect.left() + 3.0, cell_rect.top() + 10.0),
            egui::Align2::LEFT_TOP,
            title,
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );
    }
    
    fn get_week_start(date: NaiveDate) -> NaiveDate {
        let weekday = date.weekday().num_days_from_sunday();
        date - Duration::days(weekday as i64)
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
        use chrono::{TimeZone, Local};
        
        if dates.is_empty() {
            return Vec::new();
        }
        
        let start = Local.from_local_datetime(&dates[0].and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let end = Local.from_local_datetime(&dates[dates.len() - 1].and_hms_opt(23, 59, 59).unwrap())
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
