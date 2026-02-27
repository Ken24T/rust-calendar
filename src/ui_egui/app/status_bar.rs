//! Status bar component showing view info, date, event count, and contextual hints.
//!
//! The status bar displays:
//! - Current view type and date
//! - Event counts (today's events + visible period events)
//! - Next upcoming event with countdown
//! - Save/sync status indicator
//! - Contextual keyboard shortcuts based on current state

use super::state::ViewType;
use super::CalendarApp;
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use egui::{Color32, RichText, Sense};

/// Status bar section separator
const SEPARATOR_WIDTH: f32 = 8.0;

/// Get theme-aware secondary text color
fn secondary_text_color(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_gray(160)
    } else {
        Color32::from_gray(100)
    }
}

/// Get theme-aware hint text color
fn hint_text_color(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_gray(140)
    } else {
        Color32::from_gray(110)
    }
}

/// Get theme-aware separator color
fn separator_color(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_gray(80)
    } else {
        Color32::from_gray(180)
    }
}

impl CalendarApp {
    /// Render the status bar at the bottom of the window
    pub(super) fn render_status_bar(&mut self, ctx: &egui::Context) {
        let is_dark = self.active_theme.is_dark;
        
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // Left side: View info, date, event counts
                    self.render_status_left(ui, is_dark);
                    
                    // Spacer
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Right side: Hints, save status, next event (rendered right-to-left)
                        self.render_status_right(ui, is_dark);
                    });
                });
            });
    }
    
    /// Render left side of status bar: view indicator, date, event counts
    fn render_status_left(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let secondary_color = secondary_text_color(is_dark);
        let sep_color = separator_color(is_dark);
        
        // View indicator
        let view_name = match self.current_view {
            ViewType::Day => "📅 Day",
            ViewType::Week => "📆 Week",
            ViewType::WorkWeek => "💼 Work Week",
            ViewType::Month => "🗓️ Month",
        };
        ui.label(RichText::new(view_name).small());
        
        ui.add_space(SEPARATOR_WIDTH);
        ui.separator();
        ui.add_space(SEPARATOR_WIDTH);
        
        // Current date - clickable to open date picker
        let date_str = self.current_date.format("%A, %B %d, %Y").to_string();
        let date_response = ui.add(
            egui::Label::new(RichText::new(&date_str).small())
                .sense(Sense::click())
        );
        if date_response.clicked() {
            self.state.date_picker_state.open(self.current_date);
        }
        if date_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        date_response.on_hover_text("Click to go to a specific date");
        
        ui.add_space(SEPARATOR_WIDTH);
        ui.separator();
        ui.add_space(SEPARATOR_WIDTH);
        
        // Week number
        let week_num = self.current_date.iso_week().week();
        ui.label(RichText::new(format!("W{}", week_num)).small().color(secondary_color));
        
        ui.add_space(SEPARATOR_WIDTH);
        ui.separator();
        ui.add_space(SEPARATOR_WIDTH);
        
        // Event counts - clickable to open search
        let (today_count, visible_count) = self.get_event_counts();
        
        // Today's events (always shown)
        let today_text = if today_count == 1 {
            "1 today".to_string()
        } else {
            format!("{} today", today_count)
        };
        let today_response = ui.add(
            egui::Label::new(RichText::new(&today_text).small())
                .sense(Sense::click())
        );
        if today_response.clicked() {
            self.state.show_search_dialog = true;
        }
        if today_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        today_response.on_hover_text("Click to search events");
        
        // Visible period events (if different from today)
        if self.current_view != ViewType::Day {
            ui.label(RichText::new(" · ").small().color(sep_color));
            let visible_text = if visible_count == 1 {
                "1 in view".to_string()
            } else {
                format!("{} in view", visible_count)
            };
            ui.label(RichText::new(&visible_text).small());
        }
    }
    
    /// Render right side of status bar: save status, next event, keyboard hints
    fn render_status_right(&mut self, ui: &mut egui::Ui, is_dark: bool) {
        let hint_color = hint_text_color(is_dark);
        let sep_color = separator_color(is_dark);
        
        // Keyboard hints (contextual)
        self.render_contextual_hints(ui, hint_color, sep_color);
        
        ui.add_space(SEPARATOR_WIDTH);
        ui.separator();
        ui.add_space(SEPARATOR_WIDTH);
        
        // Save/sync status
        self.render_save_status(ui, is_dark);
        
        ui.add_space(SEPARATOR_WIDTH);
        ui.separator();
        ui.add_space(SEPARATOR_WIDTH);

        // Calendar sync scheduler status
        self.render_calendar_sync_status(ui, is_dark);

        ui.add_space(SEPARATOR_WIDTH);
        ui.separator();
        ui.add_space(SEPARATOR_WIDTH);
        
        // Next upcoming event
        self.render_next_event(ui, is_dark);
    }
    
    /// Render contextual keyboard hints based on current app state
    fn render_contextual_hints(&self, ui: &mut egui::Ui, hint_color: Color32, sep_color: Color32) {
        let any_dialog_open = self.show_event_dialog
            || self.show_settings_dialog
            || self.state.show_search_dialog
            || self.state.theme_dialog_state.is_open
            || self.state.date_picker_state.is_open
            || self.state.show_about_dialog
            || self.confirm_dialog.is_open();
        
        if any_dialog_open {
            // Dialog is open - show Esc hint
            ui.label(RichText::new("Esc: Close").small().color(hint_color));
        } else {
            // No dialog - show navigation hints
            ui.label(RichText::new("Ctrl+N: New").small().color(hint_color));
            ui.add_space(4.0);
            ui.label(RichText::new("·").small().color(sep_color));
            ui.add_space(4.0);
            ui.label(RichText::new("D/W/M: View").small().color(hint_color));
            ui.add_space(4.0);
            ui.label(RichText::new("·").small().color(sep_color));
            ui.add_space(4.0);
            ui.label(RichText::new("Arrows: Navigate").small().color(hint_color));
        }
    }
    
    /// Render save/sync status indicator
    fn render_save_status(&self, ui: &mut egui::Ui, is_dark: bool) {
        let countdown_dirty = self.context.countdown_service().is_dirty();
        
        // Colors that work in both light and dark themes
        let (unsaved_color, saved_color) = if is_dark {
            (Color32::from_rgb(255, 180, 80), Color32::from_rgb(100, 200, 120))
        } else {
            (Color32::from_rgb(200, 120, 0), Color32::from_rgb(60, 140, 60))
        };
        
        if countdown_dirty {
            // Unsaved changes
            let response = ui.label(RichText::new("● Unsaved").small().color(unsaved_color));
            response.on_hover_text("There are unsaved changes");
        } else {
            // All saved
            let response = ui.label(RichText::new("✓ Saved").small().color(saved_color));
            response.on_hover_text("All changes saved");
        }
    }

    /// Render calendar sync scheduler status and next-run countdown.
    fn render_calendar_sync_status(&self, ui: &mut egui::Ui, is_dark: bool) {
        let neutral_color = secondary_text_color(is_dark);
        let info_color = if is_dark {
            Color32::from_rgb(110, 180, 255)
        } else {
            Color32::from_rgb(50, 110, 190)
        };
        let error_color = if is_dark {
            Color32::from_rgb(255, 150, 150)
        } else {
            Color32::from_rgb(190, 60, 60)
        };

        let countdown_text = self
            .calendar_sync_next_due_in
            .map(format_scheduler_countdown);

        let (status_text, color) = if let Some(message) = &self.calendar_sync_status_message {
            let with_countdown = if let Some(wait) = countdown_text {
                format!("{} · next {}", message, wait)
            } else {
                message.clone()
            };

            let color = if self.calendar_sync_status_is_error {
                error_color
            } else {
                info_color
            };

            (with_countdown, color)
        } else if let Some(wait) = countdown_text {
            (format!("↻ Calendar sync in {}", wait), neutral_color)
        } else {
            ("↻ Calendar sync idle".to_string(), neutral_color)
        };

        let response = ui.label(RichText::new(status_text).small().color(color));
        response.on_hover_text("Google Calendar sync status and next scheduled run");
    }
    
    /// Render next upcoming event with countdown
    fn render_next_event(&self, ui: &mut egui::Ui, is_dark: bool) {
        // Theme-aware accent color for next event
        let accent_color = if is_dark {
            Color32::from_rgb(100, 180, 255)
        } else {
            Color32::from_rgb(40, 100, 180)
        };
        let muted_color = secondary_text_color(is_dark);
        
        if let Some((title, countdown)) = self.get_next_upcoming_event() {
            let truncated_title = if title.len() > 25 {
                format!("{}…", &title[..24])
            } else {
                title.clone()
            };
            
            let next_text = format!("Next: {} in {}", truncated_title, countdown);
            let response = ui.label(RichText::new(&next_text).small().color(accent_color));
            response.on_hover_text(format!("Upcoming: {}", title));
        } else {
            ui.label(RichText::new("No upcoming events").small().color(muted_color));
        }
    }
    
    /// Get event counts: (today's events, visible period events)
    fn get_event_counts(&self) -> (usize, usize) {
        let event_service = self.context.event_service();
        let today = Local::now().date_naive();
        
        // Today's events
        let today_start = Local
            .from_local_datetime(&today.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let today_end = Local
            .from_local_datetime(&today.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();
        
        let today_count = event_service
            .expand_recurring_events(today_start, today_end)
            .map(|events| events.len())
            .unwrap_or(0);
        
        // Visible period events
        let visible_count = self.get_visible_event_count();
        
        (today_count, visible_count)
    }
    
    /// Get the count of events visible in the current view period
    fn get_visible_event_count(&self) -> usize {
        let event_service = self.context.event_service();
        
        let (start_date, end_date) = match self.current_view {
            ViewType::Day => (self.current_date, self.current_date),
            ViewType::Week => {
                let weekday = self.current_date.weekday().num_days_from_sunday() as i64;
                let offset = (weekday - self.settings.first_day_of_week as i64 + 7) % 7;
                let week_start = self.current_date - Duration::days(offset);
                let week_end = week_start + Duration::days(6);
                (week_start, week_end)
            }
            ViewType::WorkWeek => {
                let weekday = self.current_date.weekday().num_days_from_sunday() as i64;
                let offset = (weekday - self.settings.first_day_of_work_week as i64 + 7) % 7;
                let work_start = self.current_date - Duration::days(offset);
                let work_end = work_start + Duration::days(4);
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
    
    /// Get the next upcoming event title and countdown string
    fn get_next_upcoming_event(&self) -> Option<(String, String)> {
        let event_service = self.context.event_service();
        let now = Local::now();
        
        // Look ahead up to 7 days for the next event
        let end = now + Duration::days(7);
        
        let events = event_service
            .expand_recurring_events(now, end)
            .ok()?;
        
        // Find the next event that starts after now
        let next_event = events
            .into_iter()
            .filter(|e| e.start > now)
            .min_by_key(|e| e.start)?;
        
        let duration = next_event.start.signed_duration_since(now);
        let countdown = format_duration_short(duration);
        
        Some((next_event.title.clone(), countdown))
    }
}

fn format_scheduler_countdown(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

/// Format a duration into a short human-readable string
fn format_duration_short(duration: Duration) -> String {
    let total_seconds = duration.num_seconds();
    
    if total_seconds < 0 {
        return "now".to_string();
    }
    
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    
    if days > 0 {
        if hours > 0 {
            format!("{}d {}h", days, hours)
        } else {
            format!("{}d", days)
        }
    } else if hours > 0 {
        if minutes > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else {
        "< 1m".to_string()
    }
}
