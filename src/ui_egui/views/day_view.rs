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
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
    ) -> Option<Event> {
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
        let mut clicked_event = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(event) = Self::render_time_slots(
                    ui,
                    *current_date,
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
    
    fn render_time_slots(
        ui: &mut egui::Ui,
        date: NaiveDate,
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
        
        let mut clicked_event: Option<Event> = None;
        
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
                
                if let Some(event) = Self::render_time_slot(
                    ui,
                    date,
                    time,
                    hour,
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
            }
        }
        
        clicked_event
    }
    
    fn render_time_slot(
        ui: &mut egui::Ui,
        date: NaiveDate,
        time: NaiveTime,
        hour: i64,
        is_hour_start: bool,
        starting_events: &[&Event],  // Events that start in this slot
        continuing_events: &[&Event], // Events continuing through this slot
        database: &'static Database,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_time: &mut Option<NaiveTime>,
        event_dialog_recurrence: &mut Option<String>,
    ) -> Option<Event> {
        let mut clicked_event: Option<Event> = None;
        
        ui.horizontal(|ui| {
            // Time label with fixed width (only on hour starts)
            ui.allocate_ui_with_layout(
                Vec2::new(50.0, 40.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if is_hour_start {
                        let time_str = format!("{:02}:00", hour);
                        ui.add_space(5.0); // Small padding on the right
                        ui.label(
                            egui::RichText::new(time_str)
                                .size(12.0)
                                .color(Color32::GRAY)
                        );
                    }
                },
            );
            
            // Time slot area
            let desired_size = Vec2::new(ui.available_width(), 40.0);
            let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click().union(Sense::hover()));
            
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
            
            // Draw continuing events first (colored blocks only)
            for event in continuing_events {
                Self::render_event_continuation(ui, rect, event);
            }
            
            // Draw starting events (full details)
            for event in starting_events {
                Self::render_event_in_slot(ui, rect, event);
            }

            // Manual context menu handling - store popup state in egui memory
            let mut context_clicked_event: Option<Event> = None;
            let popup_id = response.id.with(format!("context_menu_{}_{:?}", date, time));

            // Derive a narrower anchor rect from the slot so the popup doesn't stretch full width
            let mut popup_anchor_response = response.clone();
            popup_anchor_response.rect = Rect::from_min_size(
                Pos2::new(rect.left() + 55.0, rect.top()),
                Vec2::new(200.0, rect.height()),
            );

            if response.secondary_clicked() {
                ui.memory_mut(|mem| mem.open_popup(popup_id));
            }

            egui::popup::popup_above_or_below_widget(
                ui,
                popup_id,
                &popup_anchor_response,
                egui::AboveOrBelow::Below,
                egui::PopupCloseBehavior::CloseOnClickOutside,
                |ui| {
                    ui.set_width(190.0);

                    if let Some(event) = starting_events.first().or_else(|| continuing_events.first()) {
                        ui.label(format!("Event: {}", event.title));
                        ui.separator();

                        if ui.button("✏ Edit").clicked() {
                            context_clicked_event = Some((*event).clone());
                            ui.memory_mut(|mem| mem.close_popup());
                        }

                        if ui.button("🗑 Delete").clicked() {
                            if let Some(id) = event.id {
                                let service = EventService::new(database.connection());
                                let _ = service.delete(id);
                            }
                            ui.memory_mut(|mem| mem.close_popup());
                        }
                    } else {
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
                            *event_dialog_recurrence = Some("FREQ=DAILY".to_string());
                            ui.memory_mut(|mem| mem.close_popup());
                        }
                    }
                },
            );

            let mut clicked_from_ui: Option<Event> = context_clicked_event;
            
            // Handle click - check if we clicked on an event first
            if clicked_from_ui.is_none() && response.clicked() {
                // Check if click is over an event (starting or continuing)
                let click_pos = response.interact_pointer_pos();
                if let Some(pos) = click_pos {
                    let event_area = Rect::from_min_size(
                        Pos2::new(rect.left() + 55.0, rect.top()),
                        Vec2::new(rect.width() - 60.0, rect.height()),
                    );
                    
                    if event_area.contains(pos) {
                        // Clicked on event area - prefer starting events, then continuing
                        if let Some(event) = starting_events.first() {
                            clicked_from_ui = Some((*event).clone());
                        } else if let Some(event) = continuing_events.first() {
                            clicked_from_ui = Some((*event).clone());
                        }
                    }
                }
                
                // If no event was clicked, create new event
                if clicked_from_ui.is_none() {
                    *show_event_dialog = true;
                    *event_dialog_date = Some(date);
                    *event_dialog_time = Some(time); // Use the clicked time slot
                    *event_dialog_recurrence = None; // Default to non-recurring
                }
            }
            
            // Handle double-click for recurring event
            if response.double_clicked() {
                *show_event_dialog = true;
                *event_dialog_date = Some(date);
                *event_dialog_time = Some(time); // Use the clicked time slot
                *event_dialog_recurrence = Some("FREQ=DAILY".to_string());
            }

            clicked_event = clicked_from_ui;
        });
        
        clicked_event
    }
    
    fn render_event_in_slot(ui: &mut egui::Ui, slot_rect: Rect, event: &Event) {
        let event_color = event.color.as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));
        
        // Event background - fill the slot area (after the time label)
        let bg_rect = Rect::from_min_size(
            Pos2::new(slot_rect.left() + 55.0, slot_rect.top() + 2.0),
            Vec2::new(slot_rect.width() - 60.0, slot_rect.height() - 4.0),
        );
        ui.painter().rect_filled(bg_rect, 2.0, event_color);
        
        // Event bar (left side) - darker accent
        let bar_rect = Rect::from_min_size(
            Pos2::new(slot_rect.left() + 55.0, slot_rect.top() + 2.0),
            Vec2::new(4.0, slot_rect.height() - 4.0),
        );
        ui.painter().rect_filled(bar_rect, 2.0, event_color.linear_multiply(0.7));
        
        // Event title
        let text_rect = Rect::from_min_size(
            Pos2::new(bar_rect.right() + 5.0, slot_rect.top() + 2.0),
            Vec2::new(slot_rect.width() - 70.0, slot_rect.height() - 4.0),
        );
        
        // Use egui's layout system to properly truncate text
        let font_id = egui::FontId::proportional(13.0);
        let available_width = text_rect.width();
        
        let layout_job = egui::text::LayoutJob::simple(
            event.title.clone(),
            font_id.clone(),
            Color32::WHITE,
            available_width,
        );
        
        let galley = ui.fonts(|f| f.layout_job(layout_job));
        
        ui.painter().galley(
            Pos2::new(text_rect.left(), text_rect.center().y - galley.size().y / 2.0),
            galley,
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
            Color32::WHITE,
        );
    }
    
    fn render_event_continuation(ui: &mut egui::Ui, slot_rect: Rect, event: &Event) {
        let event_color = event.color.as_deref()
            .and_then(Self::parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));
        
        // Just render a colored bar to show the event continues through this slot
        let bar_rect = Rect::from_min_size(
            Pos2::new(slot_rect.left() + 55.0, slot_rect.top() + 2.0),
            Vec2::new(4.0, slot_rect.height() - 4.0),
        );
        ui.painter().rect_filled(bar_rect, 2.0, event_color);
        
        // Optional: render a lighter background to show continuation
        let bg_rect = Rect::from_min_size(
            Pos2::new(bar_rect.right() + 5.0, slot_rect.top() + 2.0),
            Vec2::new(slot_rect.width() - 70.0, slot_rect.height() - 4.0),
        );
        ui.painter().rect_filled(bg_rect, 2.0, event_color.linear_multiply(0.3));
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
