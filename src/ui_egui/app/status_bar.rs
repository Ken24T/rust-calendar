//! Status bar component showing view info, date, event count, and keyboard hints.

use super::state::ViewType;
use super::CalendarApp;
use chrono::{Datelike, Local, NaiveDate, TimeZone};
use egui::{Color32, RichText};

impl CalendarApp {
    /// Render the status bar at the bottom of the window
    pub(super) fn render_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // View indicator
                    let view_name = match self.current_view {
                        ViewType::Day => "📅 Day",
                        ViewType::Week => "📆 Week",
                        ViewType::WorkWeek => "💼 Work Week",
                        ViewType::Month => "🗓️ Month",
                    };
                    ui.label(RichText::new(view_name).small());
                    
                    ui.separator();
                    
                    // Current date
                    let date_str = self.current_date.format("%A, %B %d, %Y").to_string();
                    ui.label(RichText::new(&date_str).small());
                    
                    ui.separator();
                    
                    // Event count for visible period
                    let event_count = self.get_visible_event_count();
                    let event_text = if event_count == 1 {
                        "1 event".to_string()
                    } else {
                        format!("{} events", event_count)
                    };
                    ui.label(RichText::new(&event_text).small());
                    
                    // Spacer to push hints to the right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Keyboard hints
                        let hint_color = Color32::from_gray(140);
                        ui.label(RichText::new("Ctrl+N: New Event").small().color(hint_color));
                        ui.separator();
                        ui.label(RichText::new("←/→: Navigate").small().color(hint_color));
                        ui.separator();
                        ui.label(RichText::new("Esc: Close").small().color(hint_color));
                    });
                });
            });
    }
    
    /// Get the count of events visible in the current view period
    fn get_visible_event_count(&self) -> usize {
        let event_service = self.context.event_service();
        
        let (start_date, end_date) = match self.current_view {
            ViewType::Day => (self.current_date, self.current_date),
            ViewType::Week => {
                let weekday = self.current_date.weekday().num_days_from_sunday() as i64;
                let offset = (weekday - self.settings.first_day_of_week as i64 + 7) % 7;
                let week_start = self.current_date - chrono::Duration::days(offset);
                let week_end = week_start + chrono::Duration::days(6);
                (week_start, week_end)
            }
            ViewType::WorkWeek => {
                let weekday = self.current_date.weekday().num_days_from_sunday() as i64;
                let offset = (weekday - self.settings.first_day_of_work_week as i64 + 7) % 7;
                let work_start = self.current_date - chrono::Duration::days(offset);
                let work_end = work_start + chrono::Duration::days(4);
                (work_start, work_end)
            }
            ViewType::Month => {
                let first_of_month = self.current_date.with_day(1).unwrap();
                let last_of_month = if self.current_date.month() == 12 {
                    NaiveDate::from_ymd_opt(self.current_date.year() + 1, 1, 1)
                        .unwrap()
                        .pred_opt()
                        .unwrap()
                } else {
                    NaiveDate::from_ymd_opt(self.current_date.year(), self.current_date.month() + 1, 1)
                        .unwrap()
                        .pred_opt()
                        .unwrap()
                };
                (first_of_month, last_of_month)
            }
        };
        
        let start_datetime = Local
            .from_local_datetime(&start_date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let end_datetime = Local
            .from_local_datetime(&end_date.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();
        
        event_service
            .expand_recurring_events(start_datetime, end_datetime)
            .map(|events| events.len())
            .unwrap_or(0)
    }
}
