use chrono::{Datelike, Local, NaiveDate};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::database::Database;
use crate::services::event::EventService;

pub struct MonthView;

impl MonthView {
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        database: &'static Database,
        settings: &Settings,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
        event_to_edit: &mut Option<i64>,
    ) {
        let today = Local::now().date_naive();
        
        // Get events for the month
        let event_service = EventService::new(database.connection());
        let events = Self::get_events_for_month(&event_service, *current_date);
        
        // Day of week headers - use Grid to match column widths below
        let day_names = Self::get_day_names(settings.first_day_of_week);
        let spacing = 2.0;
        let total_spacing = spacing * 6.0; // 6 gaps between 7 columns
        let col_width = (ui.available_width() - total_spacing) / 7.0;
        
        egui::Grid::new("month_header_grid")
            .spacing([spacing, spacing])
            .min_col_width(col_width)
            .show(ui, |ui| {
                for day in &day_names {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(*day)
                                .size(14.0)
                                .strong()
                        );
                    });
                }
            });
        
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
        
        // Calculate calendar grid
        let first_of_month = current_date.with_day(1).unwrap();
        let first_weekday = (first_of_month.weekday().num_days_from_sunday() as i32
            - settings.first_day_of_week as i32 + 7) % 7;
        let days_in_month = Self::get_days_in_month(current_date.year(), current_date.month());
        
        // Build calendar grid (6 rows of 7 days)
        let mut day_counter = 1 - first_weekday;
        
        egui::Grid::new("month_grid")
            .spacing([spacing, spacing])
            .min_col_width(col_width)
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
                            
                            // Calculate weekend based on first_day_of_week
                            // If Sunday is first day (0), weekend is days 0 and 6
                            // If Monday is first day (1), weekend is days 5 and 6 (Sat, Sun)
                            let day_of_week = (date.weekday().num_days_from_sunday() as i32 - settings.first_day_of_week as i32 + 7) % 7;
                            let is_weekend = day_of_week == 5 || day_of_week == 6;
                            
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
                                database,
                                show_event_dialog,
                                event_dialog_date,
                                event_dialog_recurrence,
                                event_to_edit,
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
        database: &'static Database,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
        event_to_edit: &mut Option<i64>,
    ) {
        let desired_size = Vec2::new(ui.available_width(), 80.0);
        // Use union of click and hover to capture both left and right clicks
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click().union(Sense::hover()));
        
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
        let mut event_hitboxes: Vec<(Rect, Event)> = Vec::new();
        for &event in events.iter().take(3) {
            let event_color = event.color.as_deref()
                .and_then(Self::parse_color)
                .unwrap_or(Color32::from_rgb(100, 150, 200));
            
            // Event indicator bar
            let event_rect = Rect::from_min_size(
                Pos2::new(rect.left() + 3.0, rect.top() + y_offset),
                Vec2::new(rect.width() - 6.0, 16.0),
            );
            
            ui.painter().rect_filled(event_rect, 2.0, event_color);
            event_hitboxes.push((event_rect, event.clone()));
            
            // Event title - use available width with proper truncation
            let font_id = egui::FontId::proportional(11.0);
            let available_width = event_rect.width() - 6.0; // 3px padding on each side
            
            let layout_job = egui::text::LayoutJob::simple(
                event.title.clone(),
                font_id.clone(),
                Color32::WHITE,
                available_width,
            );
            
            let galley = ui.fonts(|f| f.layout_job(layout_job));
            
            ui.painter().galley(
                Pos2::new(event_rect.left() + 3.0, event_rect.center().y - galley.size().y / 2.0),
                galley,
                Color32::WHITE,
            );
            
            y_offset += 18.0;
        }
        
        let pointer_pos = response.interact_pointer_pos();
        let pointer_event = pointer_pos.and_then(|pos| {
            event_hitboxes
                .iter()
                .rev()
                .find(|(hit_rect, _)| hit_rect.contains(pos))
                .map(|(_, event)| event.clone())
        });
        let single_event_fallback = if event_hitboxes.len() == 1 {
            Some(event_hitboxes[0].1.clone())
        } else {
            None
        };

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
        
        // Manual context menu handling
        let popup_id = response.id.with(format!("month_context_menu_{}", date));
        let mut popup_anchor_response = response.clone();
        popup_anchor_response.rect = Rect::from_min_size(
            Pos2::new(rect.left() + 5.0, rect.top()),
            Vec2::new(200.0, 30.0),
        );

        let mut context_menu_event: Option<Event> = None;
        if response.secondary_clicked() {
            context_menu_event = pointer_event.clone();
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

                let popup_event = context_menu_event
                    .clone()
                    .or_else(|| single_event_fallback.clone());

                if let Some(event) = popup_event {
                    ui.label(format!("Event: {}", event.title));
                    ui.separator();

                    if ui.button("✏ Edit").clicked() {
                        if let Some(id) = event.id {
                            *event_to_edit = Some(id);
                            *show_event_dialog = true;
                            *event_dialog_date = Some(date);
                        }
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
                        *event_dialog_recurrence = None;
                        ui.memory_mut(|mem| mem.close_popup());
                    }

                    if ui.button("🔄 New Recurring Event").clicked() {
                        *show_event_dialog = true;
                        *event_dialog_date = Some(date);
                        *event_dialog_recurrence = Some("FREQ=MONTHLY".to_string());
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
            },
        );

        // Handle click to edit or create event
        if response.clicked() {
            let mut handled = false;

            if let Some(event) = pointer_event.clone() {
                if let Some(id) = event.id {
                    *show_event_dialog = true;
                    *event_to_edit = Some(id);
                    *event_dialog_date = Some(date);
                }
                handled = true;
            }

            if !handled {
                *show_event_dialog = true;
                *event_dialog_date = Some(date);
                *event_dialog_recurrence = None; // Default to non-recurring
            }
        }
        
        // Handle double-click for recurring event
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
    
    fn get_day_names(first_day_of_week: u8) -> Vec<&'static str> {
        let all_days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let start = first_day_of_week as usize;
        let mut result = Vec::with_capacity(7);
        for i in 0..7 {
            result.push(all_days[(start + i) % 7]);
        }
        result
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
