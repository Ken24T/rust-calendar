//! Sidebar component with mini calendar and upcoming events.

use super::CalendarApp;
use crate::models::event::Event;
use crate::ui_egui::views::utils::parse_color;
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use egui::{Color32, RichText};

impl CalendarApp {
    /// Render the sidebar panel (left or right based on settings)
    pub(super) fn render_sidebar(&mut self, ctx: &egui::Context) {
        if !self.settings.show_sidebar {
            return;
        }

        // Mini calendar needs: 7 cols × 18px + 6 gaps × 2px + padding ≈ 150px
        const SIDEBAR_MIN_WIDTH: f32 = 150.0;
        const SIDEBAR_MAX_WIDTH: f32 = 300.0;
        
        // Use persisted width from settings, clamped to valid range
        let width = self.settings.sidebar_width.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
        
        // Use unique IDs for left and right panels
        let panel_id = if self.settings.my_day_position_right {
            "sidebar_right"
        } else {
            "sidebar_left"
        };
        
        // Build the panel - use exact_width to force our size
        // Use a frame with no outer margin and no stroke to prevent gaps between panels
        // Inner margin provides padding for content
        let panel_frame = egui::Frame::side_top_panel(&ctx.style())
            .outer_margin(egui::Margin::ZERO)
            .inner_margin(egui::Margin {
                left: 8.0,
                right: 0.0,  // No right margin - central panel will provide its padding
                top: 8.0,
                bottom: 8.0,
            })
            .stroke(egui::Stroke::NONE);
        
        let panel = if self.settings.my_day_position_right {
            egui::SidePanel::right(panel_id)
        } else {
            egui::SidePanel::left(panel_id)
        };
        
        // Use exact_width which disables egui's built-in resizing
        // Set min/max to same value to prevent any resize margin allocation
        let response = panel
            .min_width(width)
            .max_width(width)
            .default_width(width)
            .resizable(false) // Disable egui's resize, we'll handle it ourselves
            .show_separator_line(false) // No separator line
            .frame(panel_frame)
            .show(ctx, |ui| {
                self.render_sidebar_content(ui);
            });
        
        // DEBUG: Log panel rect info
        let panel_rect = response.response.rect;
        let screen_rect = ctx.screen_rect();
        log::info!(
            "SIDEBAR DEBUG: panel_rect={:?}, screen_rect={:?}, sidebar_position_right={}, width_setting={}",
            panel_rect, screen_rect, self.settings.my_day_position_right, width
        );
        
        // Manual resize handle on the panel edge
        let panel_rect = response.response.rect;
        let resize_grab_width = 4.0; // Fixed grab width
        
        let resize_rect = if self.settings.my_day_position_right {
            // Resize handle on the left edge of right panel
            egui::Rect::from_x_y_ranges(
                (panel_rect.left() - resize_grab_width)..=(panel_rect.left() + resize_grab_width),
                panel_rect.y_range(),
            )
        } else {
            // Resize handle on the right edge of left panel
            egui::Rect::from_x_y_ranges(
                (panel_rect.right() - resize_grab_width)..=(panel_rect.right() + resize_grab_width),
                panel_rect.y_range(),
            )
        };
        
        let resize_id = egui::Id::new(panel_id).with("__resize");
        
        // Check for pointer interaction directly
        let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
        let is_hovering = pointer_pos.map(|p| resize_rect.contains(p)).unwrap_or(false);
        let is_dragging = ctx.is_being_dragged(resize_id);
        let drag_started = is_hovering && ctx.input(|i| i.pointer.any_pressed());
        let drag_released = ctx.input(|i| i.pointer.any_released());
        
        // Start drag
        if drag_started && !is_dragging {
            ctx.set_dragged_id(resize_id);
        }
        
        // Show resize cursor when hovering or dragging
        if is_hovering || is_dragging {
            ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        
        // Handle drag to resize
        if is_dragging {
            if let Some(pos) = pointer_pos {
                let screen = ctx.screen_rect();
                let new_width = if self.settings.my_day_position_right {
                    screen.right() - pos.x
                } else {
                    pos.x - screen.left()
                };
                
                let clamped_width = new_width.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
                if (clamped_width - self.settings.sidebar_width).abs() > 0.5 {
                    self.settings.sidebar_width = clamped_width;
                }
            }
            
            // Draw resize indicator line
            let x = if self.settings.my_day_position_right {
                panel_rect.left()
            } else {
                panel_rect.right()
            };
            ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, resize_id))
                .line_segment(
                    [egui::pos2(x, panel_rect.top()), egui::pos2(x, panel_rect.bottom())],
                    egui::Stroke::new(2.0, ctx.style().visuals.selection.bg_fill),
                );
        }
        
        // Stop dragging and save on release
        if drag_released && is_dragging {
            ctx.stop_dragging();
            self.save_settings();
        }
    }

    /// Render the sidebar content
    fn render_sidebar_content(&mut self, ui: &mut egui::Ui) {
        // Clip content to panel width to prevent content from forcing panel wider
        ui.set_clip_rect(ui.available_rect_before_wrap());
        
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
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

            // Center the month label between the arrows
            let header = viewing_date.format("%B %Y").to_string();
            let available_width = ui.available_width() - 20.0; // Reserve space for right arrow
            ui.add_space((available_width - ui.text_style_height(&egui::TextStyle::Body) * header.len() as f32 * 0.5).max(0.0) / 2.0);
            if ui.selectable_label(false, RichText::new(&header).strong())
                .on_hover_text("Go to today")
                .clicked()
            {
                self.current_date = today;
                self.focus_on_current_time_if_visible();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("▶").on_hover_text("Next month").clicked() {
                    self.current_date = shift_month(self.current_date, 1);
                }
            });
        });

        ui.add_space(4.0);

        // Day of week headers and calendar grid
        let day_names = ["S", "M", "T", "W", "T", "F", "S"];
        
        let available = ui.available_width();

        egui::Grid::new("sidebar_mini_calendar")
            .num_columns(7)
            .spacing([1.0, 1.0])
            .min_col_width(0.0)  // Let columns be as small as needed
            .max_col_width(available / 7.0)  // Distribute available width
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
            .and_then(parse_color)
            .unwrap_or(Color32::from_rgb(100, 150, 200));

        let available_width = ui.available_width();
        
        ui.horizontal(|ui| {
            ui.set_max_width(available_width);
            
            // Color indicator
            let (rect, _) = ui.allocate_exact_size(egui::vec2(4.0, 16.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, event_color);

            ui.vertical(|ui| {
                ui.set_max_width(available_width - 10.0);
                
                // Event title (clickable) - truncate to fit
                let title_response = ui.add(
                    egui::Label::new(RichText::new(&event.title).small())
                        .sense(egui::Sense::click())
                        .truncate()
                );
                if title_response.clicked() {
                    // Navigate to the event date
                    self.current_date = event.start.date_naive();
                    self.focus_on_event(event);
                }
                title_response.on_hover_text(&event.title);

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
                ui.add(egui::Label::new(RichText::new(&time_str).weak().small()).truncate());
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
