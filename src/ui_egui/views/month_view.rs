use chrono::{Datelike, Local, NaiveDate};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::models::event::Event;
use crate::services::database::Database;
use crate::services::event::EventService;

pub struct MonthView;

impl MonthView {
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        database: &'static Database,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
    ) {
        let today = Local::now().date_naive();
        
        // Get events for the month
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_month(&event_service, *current_date);
        
        // Day of week headers
        let day_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        
        ui.horizontal(|ui| {
            for day in &day_names {
                ui.label(
                    egui::RichText::new(*day)
                        .size(14.0)
                        .strong()
                );
                if day != &"Sat" {
                    ui.add_space(5.0);
                }
            }
        });
        
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
        
        // Calculate calendar grid
        let first_of_month = current_date.with_day(1).unwrap();
        let first_weekday = first_of_month.weekday().num_days_from_sunday() as i32;
        let days_in_month = Self::get_days_in_month(current_date.year(), current_date.month());
        
        // Build calendar grid (6 rows of 7 days)
        let mut day_counter = 1 - first_weekday;
        
        egui::Grid::new("month_grid")
            .spacing([2.0, 2.0])
            .min_col_width(ui.available_width() / 7.0 - 2.0)
            .show(ui, |ui| {
                for _week in 0..6 {
                    for _day_of_week in 0..7 {
                        if day_counter < 1 || day_counter > days_in_month {
                            // Empty cell for days outside current month
                            let (rect, _response) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 80.0),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                rect,
                                2.0,
                                Color32::from_gray(30),
                            );
                        } else {
                            // Day cell
                            let date = NaiveDate::from_ymd_opt(
                                current_date.year(),
                                current_date.month(),
                                day_counter as u32,
                            ).unwrap();
                            
                            let is_today = date == today;
                            let is_weekend = date.weekday().num_days_from_sunday() == 0
                                || date.weekday().num_days_from_sunday() == 6;
                            
                            // Get events for this day
                            let day_events: Vec<&Event> = events.iter()
                                .filter(|e| {
                                    let event_date = e.start.date_naive();
                                    event_date == date
                                })
                                .collect();
                            
                            Self::render_day_cell(
                                ui,
                                day_counter,
                                date,
                                is_today,
                                is_weekend,
                                &day_events,
                                show_event_dialog,
                                event_dialog_date,
                                event_dialog_recurrence,
                            );
                        }
                        day_counter += 1;
                    }
                    ui.end_row();
                }
            });
    }
    
    fn render_day_cell(
        ui: &mut egui::Ui,
        day: i32,
        date: NaiveDate,
        is_today: bool,
        is_weekend: bool,
        events: &[&Event],
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
    ) {
        let desired_size = Vec2::new(ui.available_width(), 80.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
        
        // Background color
        let bg_color = if is_today {
            Color32::from_rgb(60, 90, 150) // Blue for today
        } else if is_weekend {
            Color32::from_gray(35)
        } else {
            Color32::from_gray(40)
        };
        
        // Draw background
        ui.painter().rect_filled(rect, 2.0, bg_color);
        
        // Draw border
        let border_color = if is_today {
            Color32::from_rgb(100, 130, 200)
        } else {
            Color32::from_gray(60)
        };
        ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0, border_color));
        
        // Hover effect
        if response.hovered() {
            ui.painter().rect_stroke(
                rect,
                2.0,
                Stroke::new(2.0, Color32::from_rgb(100, 150, 255)),
            );
        }
        
        // Day number
        let day_text = format!("{}", day);
        let text_color = if is_today {
            Color32::WHITE
        } else {
            Color32::LIGHT_GRAY
        };
        
        ui.painter().text(
            Pos2::new(rect.left() + 5.0, rect.top() + 5.0),
            egui::Align2::LEFT_TOP,
            day_text,
            egui::FontId::proportional(14.0),
            text_color,
        );
        
        // Draw events (up to 3 visible)
        let mut y_offset = 25.0;
        for (i, event) in events.iter().take(3).enumerate() {
            let event_color = event.color.as_deref()
                .and_then(Self::parse_color)
                .unwrap_or(Color32::from_rgb(100, 150, 200));
            
            // Event indicator bar
            let event_rect = Rect::from_min_size(
                Pos2::new(rect.left() + 3.0, rect.top() + y_offset),
                Vec2::new(rect.width() - 6.0, 16.0),
            );
            
            ui.painter().rect_filled(event_rect, 2.0, event_color);
            
            // Event title (truncated)
            let title = if event.title.len() > 15 {
                format!("{}...", &event.title[..12])
            } else {
                event.title.clone()
            };
            
            ui.painter().text(
                Pos2::new(event_rect.left() + 3.0, event_rect.center().y),
                egui::Align2::LEFT_CENTER,
                title,
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );
            
            y_offset += 18.0;
        }
        
        // Show "+N more" if there are more events
        if events.len() > 3 {
            ui.painter().text(
                Pos2::new(rect.left() + 5.0, rect.top() + y_offset),
                egui::Align2::LEFT_TOP,
                format!("+{} more", events.len() - 3),
                egui::FontId::proportional(10.0),
                Color32::GRAY,
            );
        }
        
        // Handle click to create event
        if response.clicked() {
            *show_event_dialog = true;
            *event_dialog_date = Some(date);
            *event_dialog_recurrence = Some("FREQ=DAILY".to_string());
        }
        
        // Handle double-click (egui supports this!)
        if response.double_clicked() {
            *show_event_dialog = true;
            *event_dialog_date = Some(date);
            *event_dialog_recurrence = Some("FREQ=MONTHLY".to_string());
        }
    }
    
    fn get_events_for_month(event_service: &EventService, date: NaiveDate) -> Vec<Event> {
        use chrono::{TimeZone, Local};
        
        let start_of_month = date.with_day(1).unwrap();
        let start = Local.from_local_datetime(&start_of_month.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        
        // Get last day of month
        let days_in_month = Self::get_days_in_month(date.year(), date.month());
        let end_of_month = date.with_day(days_in_month as u32).unwrap();
        let end = Local.from_local_datetime(&end_of_month.and_hms_opt(23, 59, 59).unwrap())
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
