use chrono::{Datelike, Local, NaiveDate};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

pub struct QuarterView;

impl QuarterView {
    pub fn show(
        ui: &mut egui::Ui,
        current_date: &mut NaiveDate,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
    ) {
        let today = Local::now().date_naive();
        
        // Determine which quarter we're in
        let quarter_start_month = ((current_date.month() - 1) / 3) * 3 + 1;
        
        // Show 3 months in the quarter
        ui.horizontal(|ui| {
            for month_offset in 0..3 {
                let month = quarter_start_month + month_offset;
                let year = current_date.year();
                let month_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
                
                ui.vertical(|ui| {
                    Self::render_mini_month(
                        ui,
                        month_date,
                        today,
                        show_event_dialog,
                        event_dialog_date,
                        event_dialog_recurrence,
                    );
                });
                
                if month_offset < 2 {
                    ui.add_space(10.0);
                }
            }
        });
    }
    
    fn render_mini_month(
        ui: &mut egui::Ui,
        month_date: NaiveDate,
        today: NaiveDate,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
    ) {
        // Month header
        let month_name = month_date.format("%B %Y").to_string();
        ui.heading(&month_name);
        ui.add_space(5.0);
        
        // Day of week headers (abbreviated)
        let day_names = ["S", "M", "T", "W", "T", "F", "S"];
        
        egui::Grid::new(format!("quarter_day_headers_{}", month_date.month()))
            .spacing([2.0, 2.0])
            .show(ui, |ui| {
                for day in &day_names {
                    ui.label(
                        egui::RichText::new(*day)
                            .size(12.0)
                            .strong()
                    );
                }
                ui.end_row();
            });
        
        ui.add_space(3.0);
        
        // Calculate calendar grid
        let first_weekday = month_date.weekday().num_days_from_sunday() as i32;
        let days_in_month = Self::get_days_in_month(month_date.year(), month_date.month());
        
        // Build mini calendar grid
        let mut day_counter = 1 - first_weekday;
        
        egui::Grid::new(format!("quarter_grid_{}", month_date.month()))
            .spacing([2.0, 2.0])
            .show(ui, |ui| {
                for _week in 0..6 {
                    for _day_of_week in 0..7 {
                        if day_counter < 1 || day_counter > days_in_month {
                            // Empty cell
                            let (rect, _response) = ui.allocate_exact_size(
                                Vec2::new(30.0, 30.0),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                rect,
                                2.0,
                                Color32::from_gray(25),
                            );
                        } else {
                            // Day cell
                            let date = NaiveDate::from_ymd_opt(
                                month_date.year(),
                                month_date.month(),
                                day_counter as u32,
                            ).unwrap();
                            
                            let is_today = date == today;
                            let is_weekend = date.weekday().num_days_from_sunday() == 0
                                || date.weekday().num_days_from_sunday() == 6;
                            
                            Self::render_mini_day_cell(
                                ui,
                                day_counter,
                                date,
                                is_today,
                                is_weekend,
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
    
    fn render_mini_day_cell(
        ui: &mut egui::Ui,
        day: i32,
        date: NaiveDate,
        is_today: bool,
        is_weekend: bool,
        show_event_dialog: &mut bool,
        event_dialog_date: &mut Option<NaiveDate>,
        event_dialog_recurrence: &mut Option<String>,
    ) {
        let desired_size = Vec2::new(30.0, 30.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
        
        // Background color
        let bg_color = if is_today {
            Color32::from_rgb(60, 90, 150)
        } else if is_weekend {
            Color32::from_gray(30)
        } else {
            Color32::from_gray(35)
        };
        
        ui.painter().rect_filled(rect, 2.0, bg_color);
        
        // Border
        let border_color = if is_today {
            Color32::from_rgb(100, 130, 200)
        } else {
            Color32::from_gray(50)
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
        let text_color = if is_today {
            Color32::WHITE
        } else {
            Color32::LIGHT_GRAY
        };
        
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}", day),
            egui::FontId::proportional(12.0),
            text_color,
        );
        
        // Handle click to create event (quarterly recurrence)
        if response.clicked() {
            *show_event_dialog = true;
            *event_dialog_date = Some(date);
            *event_dialog_recurrence = Some("FREQ=DAILY".to_string());
        }
        
        // Handle double-click for quarterly recurrence
        if response.double_clicked() {
            *show_event_dialog = true;
            *event_dialog_date = Some(date);
            *event_dialog_recurrence = Some("FREQ=MONTHLY;INTERVAL=3".to_string());
        }
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
}
