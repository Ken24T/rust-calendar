use chrono::{Datelike, Local, NaiveDate, NaiveTime, Timelike};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;

pub struct DayView;

impl DayView {
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
        let is_today = *current_date == today;
        
        // Get events for this day
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_day(&event_service, *current_date);
        
        // Day header
        let day_name = current_date.format("%A, %B %d, %Y").to_string();
        ui.heading(&day_name);
        if is_today {
            ui.label(egui::RichText::new("Today").color(Color32::from_rgb(100, 150, 255)));
        }
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
        
        // Scrollable time slots
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Self::render_time_slots(
                    ui,
                    *current_date,
                    &events,
                    settings,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_recurrence,
                );
            });
    }
    
    fn render_time_slots(
        ui: &mut egui::Ui,
        date: NaiveDate,
        events: &[Event],
        settings: &Settings,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
    ) {
        let time_slot_interval = settings.time_slot_interval as i64;
        let slots_per_hour = 60 / time_slot_interval;
        
        // Draw 24 hours
        for hour in 0..24 {
            for slot in 0..slots_per_hour {
                let minute = slot * time_slot_interval;
                let time = NaiveTime::from_hms_opt(hour as u32, minute as u32, 0).unwrap();
                
                // Find events in this time slot
                let slot_events: Vec<&Event> = events.iter()
                    .filter(|e| {
                        let event_start = e.start.time();
                        let event_end = e.end.time();
                        
                        // Check if event overlaps with this time slot
                        let slot_start = time;
                        let slot_end = NaiveTime::from_hms_opt(
                            hour as u32,
                            (minute + time_slot_interval) as u32 % 60,
                            0,
                        ).unwrap();
                        
                        event_start < slot_end && event_end > slot_start
                    })
                    .collect();
                
                Self::render_time_slot(
                    ui,
                    date,
                    time,
                    hour,
                    slot == 0,
                    &slot_events,
                    show_event_dialog,
                    event_dialog_date,
                    event_dialog_recurrence,
                );
            }
        }
    }
    
    fn render_time_slot(
        ui: &mut egui::Ui,
        date: NaiveDate,
        time: NaiveTime,
        hour: i64,
        is_hour_start: bool,
        events: &[&Event],
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
    ) {
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
            
            // Time slot area
            let desired_size = Vec2::new(ui.available_width(), 40.0);
            let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
            
            // Background
            let bg_color = if is_hour_start {
                Color32::from_gray(45)
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
            
            // Hover effect
            if response.hovered() {
                ui.painter().rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(100, 150, 255, 30));
            }
            
            // Draw events
            for event in events {
                Self::render_event_in_slot(ui, rect, event);
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
                *event_dialog_recurrence = Some("FREQ=DAILY".to_string());
            }
        });
    }
    
    fn render_event_in_slot(ui: &mut egui::Ui, slot_rect: Rect, event: &Event) {
        let event_color = event.color.as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));
        
        // Event bar (left side)
        let bar_rect = Rect::from_min_size(
            Pos2::new(slot_rect.left() + 55.0, slot_rect.top() + 2.0),
            Vec2::new(4.0, slot_rect.height() - 4.0),
        );
        ui.painter().rect_filled(bar_rect, 2.0, event_color);
        
        // Event title
        let text_rect = Rect::from_min_size(
            Pos2::new(bar_rect.right() + 5.0, slot_rect.top() + 2.0),
            Vec2::new(slot_rect.width() - 70.0, slot_rect.height() - 4.0),
        );
        
        let title = if event.title.len() > 40 {
            format!("{}...", &event.title[..37])
        } else {
            event.title.clone()
        };
        
        ui.painter().text(
            Pos2::new(text_rect.left(), text_rect.center().y),
            egui::Align2::LEFT_CENTER,
            title,
            egui::FontId::proportional(13.0),
            Color32::WHITE,
        );
        
        // Time range
        let time_str = format!(
            "{} - {}",
            event.start.format("%H:%M"),
            event.end.format("%H:%M")
        );
        ui.painter().text(
            Pos2::new(text_rect.left(), text_rect.top() + 2.0),
            egui::Align2::LEFT_TOP,
            time_str,
            egui::FontId::proportional(10.0),
            Color32::GRAY,
        );
    }
    
    fn get_events_for_day(event_service: &EventService, date: NaiveDate) -> Vec<Event> {
        use chrono::{TimeZone, Local};
        
        let start = Local.from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let end = Local.from_local_datetime(&date.and_hms_opt(23, 59, 59).unwrap())
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
