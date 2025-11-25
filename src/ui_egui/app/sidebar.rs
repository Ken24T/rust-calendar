//! Sidebar component with mini calendar and upcoming events.

use super::CalendarApp;
use crate::models::event::Event;
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use egui::{Color32, RichText};

impl CalendarApp {
    /// Render the sidebar panel (left or right based on settings)
    pub(super) fn render_sidebar(&mut self, ctx: &egui::Context) {
        if !self.settings.show_sidebar {
            return;
        }

        let panel_id = "sidebar";
        
        // Mini calendar needs: 7 cols × 20px + 6 gaps × 2px + padding ≈ 160px
        const SIDEBAR_MIN_WIDTH: f32 = 160.0;
        const SIDEBAR_DEFAULT_WIDTH: f32 = 180.0;
        const SIDEBAR_MAX_WIDTH: f32 = 300.0;
        
        if self.settings.my_day_position_right {
            egui::SidePanel::right(panel_id)
                .default_width(SIDEBAR_DEFAULT_WIDTH)
                .min_width(SIDEBAR_MIN_WIDTH)
                .max_width(SIDEBAR_MAX_WIDTH)
                .resizable(true)
                .show(ctx, |ui| {
                    self.render_sidebar_content(ui);
                });
        } else {
            egui::SidePanel::left(panel_id)
                .default_width(SIDEBAR_DEFAULT_WIDTH)
                .min_width(SIDEBAR_MIN_WIDTH)
                .max_width(SIDEBAR_MAX_WIDTH)
                .resizable(true)
                .show(ctx, |ui| {
                    self.render_sidebar_content(ui);
                });
        }
    }

    /// Render the sidebar content
    fn render_sidebar_content(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            self.render_sidebar_mini_calendar(ui);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            self.render_sidebar_today_agenda(ui);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            self.render_sidebar_upcoming_events(ui);
        });
    }

    /// Render the mini calendar in the sidebar
    fn render_sidebar_mini_calendar(&mut self, ui: &mut egui::Ui) {
        let today = Local::now().date_naive();
        let viewing_date = self.current_date;

        // Header with month navigation
        ui.horizontal(|ui| {
            if ui.small_button("◀").on_hover_text("Previous month").clicked() {
                self.current_date = shift_month(self.current_date, -1);
            }

            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                let header = viewing_date.format("%B %Y").to_string();
                if ui.selectable_label(false, RichText::new(&header).strong())
                    .on_hover_text("Go to today")
                    .clicked()
                {
                    self.current_date = today;
                    self.focus_on_current_time_if_visible();
                }
            });

            if ui.small_button("▶").on_hover_text("Next month").clicked() {
                self.current_date = shift_month(self.current_date, 1);
            }
        });

        ui.add_space(4.0);

        // Day of week headers and calendar grid
        let day_names = ["S", "M", "T", "W", "T", "F", "S"];

        egui::Grid::new("sidebar_mini_calendar")
            .num_columns(7)
            .spacing([2.0, 2.0])
            .min_col_width(18.0)
            .show(ui, |ui| {
                // Header row
                for name in &day_names {
                    ui.label(RichText::new(*name).small().weak());
                }
                ui.end_row();

                // Calendar grid
                let first_of_month = viewing_date.with_day(1).unwrap();
                let start_weekday = first_of_month.weekday().num_days_from_sunday() as i64;
                let grid_start = first_of_month - Duration::days(start_weekday);

                let mut current = grid_start;
                for _week in 0..6 {
                    for _day in 0..7 {
                        let is_current_month = current.month() == viewing_date.month();
                        let is_today = current == today;
                        let is_selected = current == self.current_date;

                        let day_str = format!("{}", current.day());

                        let text = if is_today {
                            RichText::new(&day_str).strong().color(Color32::from_rgb(50, 150, 50))
                        } else if !is_current_month {
                            RichText::new(&day_str).weak()
                        } else {
                            RichText::new(&day_str)
                        };

                        if ui.selectable_label(is_selected, text).clicked() {
                            self.current_date = current;
                            self.focus_on_current_time_if_visible();
                        }

                        current += Duration::days(1);
                    }
                    ui.end_row();

                    // Stop if we've shown enough weeks
                    if current.month() != viewing_date.month() && current.day() > 7 {
                        break;
                    }
                }
            });
    }

    /// Render the selected day's agenda in the sidebar
    fn render_sidebar_today_agenda(&mut self, ui: &mut egui::Ui) {
        let today = Local::now().date_naive();
        let selected_date = self.current_date;
        let is_today = selected_date == today;
        
        let events = self.get_events_for_date(selected_date);

        // Show different header based on whether viewing today or another date
        let header = if is_today {
            "📅 Today's Agenda".to_string()
        } else {
            format!("📅 {}", selected_date.format("%b %d"))
        };
        ui.label(RichText::new(&header).strong());
        ui.add_space(4.0);

        let empty_msg = if is_today {
            "No events today"
        } else {
            "No events on this day"
        };

        if events.is_empty() {
            ui.label(RichText::new(empty_msg).weak().italics());
        } else {
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .id_source("today_agenda_scroll")
                .show(ui, |ui| {
                    for event in events.iter().take(5) {
                        self.render_sidebar_event_item(ui, event, !is_today);
                    }
                    if events.len() > 5 {
                        ui.label(RichText::new(format!("...and {} more", events.len() - 5)).weak().small());
                    }
                });
        }
    }

    /// Render upcoming events in the sidebar
    fn render_sidebar_upcoming_events(&mut self, ui: &mut egui::Ui) {
        let upcoming = self.get_upcoming_events(10);

        ui.label(RichText::new("📆 Upcoming Events").strong());
        ui.add_space(4.0);

        if upcoming.is_empty() {
            ui.label(RichText::new("No upcoming events").weak().italics());
        } else {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .id_source("upcoming_events_scroll")
                .show(ui, |ui| {
                    for event in &upcoming {
                        self.render_sidebar_event_item(ui, event, true);
                    }
                });
        }
    }

    /// Render a single event item in the sidebar
    fn render_sidebar_event_item(&mut self, ui: &mut egui::Ui, event: &Event, show_date: bool) {
        let event_color = event
            .color
            .as_deref()
            .and_then(parse_hex_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));

        ui.horizontal(|ui| {
            // Color indicator
            let (rect, _) = ui.allocate_exact_size(egui::vec2(4.0, 16.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, event_color);

            ui.vertical(|ui| {
                // Event title (clickable)
                let title_response = ui.add(
                    egui::Label::new(RichText::new(&event.title).small())
                        .sense(egui::Sense::click())
                );
                if title_response.clicked() {
                    // Navigate to the event date
                    self.current_date = event.start.date_naive();
                    self.focus_on_event(event);
                }
                title_response.on_hover_text("Click to go to event");

                // Time/date info
                let time_str = if event.all_day {
                    if show_date {
                        event.start.format("%b %d - All day").to_string()
                    } else {
                        "All day".to_string()
                    }
                } else if show_date {
                    event.start.format("%b %d, %H:%M").to_string()
                } else {
                    event.start.format("%H:%M").to_string()
                };
                ui.label(RichText::new(&time_str).weak().small());
            });
        });
        ui.add_space(2.0);
    }

    /// Get events for a specific date
    fn get_events_for_date(&self, date: NaiveDate) -> Vec<Event> {
        let event_service = self.context.event_service();

        let start_of_day = Local
            .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let end_of_day = Local
            .from_local_datetime(&date.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();

        event_service
            .expand_recurring_events(start_of_day, end_of_day)
            .unwrap_or_default()
    }

    /// Get upcoming events (starting from now)
    fn get_upcoming_events(&self, limit: usize) -> Vec<Event> {
        let event_service = self.context.event_service();
        let now = Local::now();

        // Look ahead 30 days
        let end = now + Duration::days(30);

        let mut events = event_service
            .expand_recurring_events(now, end)
            .unwrap_or_default();

        // Filter to only future events and sort by start time
        events.retain(|e| e.start > now);
        events.sort_by(|a, b| a.start.cmp(&b.start));
        events.truncate(limit);

        events
    }

    /// Toggle sidebar visibility
    pub(super) fn toggle_sidebar(&mut self) {
        self.settings.show_sidebar = !self.settings.show_sidebar;
        self.save_settings();
    }

    /// Save settings to database
    fn save_settings(&self) {
        let settings_service = self.context.settings_service();
        if let Err(err) = settings_service.update(&self.settings) {
            log::error!("Failed to save settings: {}", err);
        }
    }
}

/// Shift a date by the given number of months
fn shift_month(date: NaiveDate, delta: i32) -> NaiveDate {
    let total_months = (date.year() * 12) as i32 + (date.month() as i32 - 1) + delta;
    let new_year = total_months.div_euclid(12);
    let new_month = (total_months.rem_euclid(12) + 1) as u32;
    let max_day = days_in_month(new_year, new_month);
    let day = date.day().min(max_day);
    NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap_or(date)
}

/// Get the number of days in a given month
fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(30)
}

/// Parse a hex color string to Color32
fn parse_hex_color(s: &str) -> Option<Color32> {
    let s = s.trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else {
        None
    }
}
