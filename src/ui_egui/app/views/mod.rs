use super::confirm::ConfirmAction;
use super::state::ViewType;
use super::CalendarApp;
use crate::models::event::Event;
use crate::ui_egui::event_dialog::EventDialogState;
use crate::ui_egui::views::day_view::DayView;
use crate::ui_egui::views::month_view::{MonthView, MonthViewAction};
use crate::ui_egui::views::week_shared::DeleteConfirmRequest;
use crate::ui_egui::views::week_view::WeekView;
use crate::ui_egui::views::workweek_view::WorkWeekView;
use crate::ui_egui::views::{AutoFocusRequest, CountdownRequest};
use chrono::{Datelike, Local, NaiveDate};
use std::collections::HashSet;

impl CalendarApp {
    /// Handle a delete confirmation request from a view
    fn handle_delete_confirm_request(&mut self, request: DeleteConfirmRequest) {
        let action = if request.occurrence_only {
            if let Some(date) = request.occurrence_date {
                ConfirmAction::DeleteEventOccurrence {
                    event_id: request.event_id,
                    event_title: request.event_title,
                    occurrence_date: date,
                }
            } else {
                // Fallback to full event deletion if no date provided
                ConfirmAction::DeleteEvent {
                    event_id: request.event_id,
                    event_title: request.event_title,
                }
            }
        } else {
            ConfirmAction::DeleteEvent {
                event_id: request.event_id,
                event_title: request.event_title,
            }
        };
        self.confirm_dialog.request(action);
    }
    
    pub(super) fn render_main_panel(
        &mut self,
        ctx: &egui::Context,
        countdown_requests: &mut Vec<CountdownRequest>,
    ) {
        let active_countdown_events: HashSet<i64> = self
            .context
            .countdown_service()
            .cards()
            .iter()
            .filter_map(|card| card.event_id)
            .collect();

        // Use a frame with no outer/inner margin to prevent gaps between panels
        // The sidebar already has its own inner padding
        let panel_frame = egui::Frame::central_panel(&ctx.style())
            .outer_margin(egui::Margin::ZERO)
            .inner_margin(egui::Margin::same(8.0));

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
            // DEBUG: Log central panel rect
            let panel_rect = ui.max_rect();
            let available_rect = ui.available_rect_before_wrap();
            log::info!(
                "CENTRAL PANEL DEBUG: max_rect={:?}, available_rect={:?}",
                panel_rect, available_rect
            );
            // Clickable heading - double-click to go to today
            let heading_text = format!(
                "{} View - {}",
                match self.current_view {
                    ViewType::Day => "Day",
                    ViewType::Week => "Week",
                    ViewType::WorkWeek => "Work Week",
                    ViewType::Month => "Month",
                },
                self.current_date.format("%B %Y")
            );
            let heading_response = ui.heading(&heading_text);
            if heading_response.double_clicked() {
                self.jump_to_today();
            }
            heading_response.on_hover_text("Double-click to go to today");

            ui.separator();

            ui.horizontal(|ui| {
                // Navigation buttons with keyboard hint tooltips
                if ui.button("◀").on_hover_text("Previous (← Arrow)").clicked() {
                    self.navigate_previous();
                }
                if ui.button("Today").on_hover_text("Ctrl+T").clicked() {
                    self.jump_to_today();
                }
                if ui.button("▶").on_hover_text("Next (→ Arrow)").clicked() {
                    self.navigate_next();
                }

                ui.separator();

                // Date picker button with mini calendar popup
                if ui.button("📅").on_hover_text("Go to date...").clicked() {
                    if self.state.date_picker_state.is_open {
                        self.state.date_picker_state.close();
                    } else {
                        self.state.date_picker_state.open(self.current_date);
                    }
                }

                ui.separator();

                // View type buttons with keyboard shortcuts
                ui.label("View:");
                if ui.selectable_label(self.current_view == ViewType::Day, "Day")
                    .on_hover_text("Press D")
                    .clicked()
                {
                    self.current_view = ViewType::Day;
                    self.focus_on_current_time_if_visible();
                }
                if ui.selectable_label(self.current_view == ViewType::Week, "Week")
                    .on_hover_text("Press W")
                    .clicked()
                {
                    self.current_view = ViewType::Week;
                    self.focus_on_current_time_if_visible();
                }
                if ui.selectable_label(self.current_view == ViewType::WorkWeek, "Work Week")
                    .on_hover_text("Press K")
                    .clicked()
                {
                    self.current_view = ViewType::WorkWeek;
                    self.focus_on_current_time_if_visible();
                }
                if ui.selectable_label(self.current_view == ViewType::Month, "Month")
                    .on_hover_text("Press M")
                    .clicked()
                {
                    self.current_view = ViewType::Month;
                }
            });

            ui.separator();

            let mut focus_request = self.pending_focus.take();
            match self.current_view {
                ViewType::Day => self.render_day_view(
                    ui,
                    countdown_requests,
                    &active_countdown_events,
                    &mut focus_request,
                ),
                ViewType::Week => self.render_week_view(
                    ui,
                    countdown_requests,
                    &active_countdown_events,
                    self.show_ribbon,
                    &mut focus_request,
                ),
                ViewType::WorkWeek => self.render_workweek_view(
                    ui,
                    countdown_requests,
                    &active_countdown_events,
                    &mut focus_request,
                ),
                ViewType::Month => self.render_month_view(ui),
            }
            self.pending_focus = focus_request;
        });
    }

    pub(super) fn render_day_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        focus_request: &mut Option<AutoFocusRequest>,
    ) {
        let view_result = DayView::show(
            ui,
            &mut self.current_date,
            self.context.database(),
            &self.settings,
            &self.active_theme,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_time,
            &mut self.event_dialog_recurrence,
            countdown_requests,
            active_countdown_events,
            focus_request,
        );
        
        // Handle clicked event - open edit dialog
        if let Some(clicked_event) = view_result.event_to_edit {
            self.event_dialog_state =
                Some(EventDialogState::from_event(&clicked_event, &self.settings));
            self.show_event_dialog = true;
        }
        
        // Handle delete confirmation request
        if let Some(request) = view_result.delete_confirm_request {
            self.handle_delete_confirm_request(request);
        }
        
        // Handle deleted events - remove countdown cards (legacy path)
        for event_id in view_result.deleted_event_ids {
            self.context.countdown_service_mut().remove_cards_for_event(event_id);
        }
        
        // Handle moved events - sync countdown cards
        for event in view_result.moved_events {
            self.sync_cards_from_event(&event);
        }
    }

    pub(super) fn render_week_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        show_ribbon: bool,
        focus_request: &mut Option<AutoFocusRequest>,
    ) {
        let all_day_events = if show_ribbon {
            use chrono::TimeZone;
            let event_service = self.context.event_service();

            let weekday = self.current_date.weekday().num_days_from_sunday() as i64;
            let offset = (weekday - self.settings.first_day_of_week as i64 + 7) % 7;
            let week_start = self.current_date - chrono::Duration::days(offset);
            let week_end = week_start + chrono::Duration::days(6);

            let start_datetime = Local
                .from_local_datetime(&week_start.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .unwrap();
            let end_datetime = Local
                .from_local_datetime(&week_end.and_hms_opt(23, 59, 59).unwrap())
                .single()
                .unwrap();

            event_service
                .expand_recurring_events(start_datetime, end_datetime)
                .unwrap_or_default()
                .into_iter()
                .filter(|e| e.all_day)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let view_result = WeekView::show(
            ui,
            &mut self.current_date,
            self.context.database(),
            &self.settings,
            &self.active_theme,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_time,
            &mut self.event_dialog_recurrence,
            countdown_requests,
            active_countdown_events,
            show_ribbon,
            &all_day_events,
            focus_request,
        );
        
        // Handle clicked event - open edit dialog
        if let Some(clicked_event) = view_result.event_to_edit {
            self.event_dialog_state =
                Some(EventDialogState::from_event(&clicked_event, &self.settings));
            self.show_event_dialog = true;
        }
        
        // Handle delete confirmation request
        if let Some(request) = view_result.delete_confirm_request {
            self.handle_delete_confirm_request(request);
        }
        
        // Handle deleted events - remove countdown cards (legacy path)
        for event_id in view_result.deleted_event_ids {
            self.context.countdown_service_mut().remove_cards_for_event(event_id);
        }
        
        // Handle moved events - sync countdown cards
        for event in view_result.moved_events {
            self.sync_cards_from_event(&event);
        }
    }

    pub(super) fn render_workweek_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        focus_request: &mut Option<AutoFocusRequest>,
    ) {
        let all_day_events = if self.show_ribbon {
            use chrono::TimeZone;

            let week_start =
                WorkWeekView::get_week_start(self.current_date, self.settings.first_day_of_week);
            let work_week_dates = WorkWeekView::get_work_week_dates(week_start, &self.settings);

            if let (Some(first_day), Some(last_day)) =
                (work_week_dates.first(), work_week_dates.last())
            {
                let event_service = self.context.event_service();
                let start_datetime = Local
                    .from_local_datetime(&first_day.and_hms_opt(0, 0, 0).unwrap())
                    .single()
                    .unwrap();
                let end_datetime = Local
                    .from_local_datetime(&last_day.and_hms_opt(23, 59, 59).unwrap())
                    .single()
                    .unwrap();

                event_service
                    .expand_recurring_events(start_datetime, end_datetime)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|e| e.all_day)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let view_result = WorkWeekView::show(
            ui,
            &mut self.current_date,
            self.context.database(),
            &self.settings,
            &self.active_theme,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_time,
            &mut self.event_dialog_recurrence,
            countdown_requests,
            active_countdown_events,
            self.show_ribbon,
            &all_day_events,
            focus_request,
        );
        
        // Handle clicked event - open edit dialog
        if let Some(clicked_event) = view_result.event_to_edit {
            self.event_dialog_state =
                Some(EventDialogState::from_event(&clicked_event, &self.settings));
            self.show_event_dialog = true;
        }
        
        // Handle delete confirmation request
        if let Some(request) = view_result.delete_confirm_request {
            self.handle_delete_confirm_request(request);
        }
        
        // Handle deleted events - remove countdown cards (legacy path)
        for event_id in view_result.deleted_event_ids {
            self.context.countdown_service_mut().remove_cards_for_event(event_id);
        }
        
        // Handle moved events - sync countdown cards
        for event in view_result.moved_events {
            self.sync_cards_from_event(&event);
        }
    }

    pub(super) fn render_month_view(&mut self, ui: &mut egui::Ui) {
        let result = MonthView::show(
            ui,
            &mut self.current_date,
            self.context.database(),
            &self.settings,
            &self.active_theme,
            &mut self.show_event_dialog,
            &mut self.event_dialog_date,
            &mut self.event_dialog_recurrence,
            &mut self.event_to_edit,
        );
        
        // Handle month view actions
        if let MonthViewAction::SwitchToDayView(date) = result.action {
            self.current_date = date;
            self.current_view = ViewType::Day;
        }
        
        // Handle delete confirmation request
        if let Some(request) = result.delete_confirm_request {
            self.handle_delete_confirm_request(request);
        }
    }

    pub(super) fn focus_on_event(&mut self, event: &Event) {
        self.current_date = event.start.date_naive();
        if matches!(
            self.current_view,
            ViewType::Day | ViewType::Week | ViewType::WorkWeek
        ) {
            self.pending_focus = Some(AutoFocusRequest::from_event(event));
        }
    }

    pub(super) fn focus_on_current_time_if_visible(&mut self) {
        if !matches!(
            self.current_view,
            ViewType::Day | ViewType::Week | ViewType::WorkWeek
        ) {
            return;
        }

        let now = Local::now();
        let today = now.date_naive();

        let should_focus = match self.current_view {
            ViewType::Day => self.current_date == today,
            ViewType::Week => {
                let week_start =
                    WeekView::get_week_start(self.current_date, self.settings.first_day_of_week);
                let week_end = week_start + chrono::Duration::days(6);
                today >= week_start && today <= week_end
            }
            ViewType::WorkWeek => {
                let week_start = WorkWeekView::get_week_start(
                    self.current_date,
                    self.settings.first_day_of_week,
                );
                let work_week_dates = WorkWeekView::get_work_week_dates(week_start, &self.settings);
                work_week_dates.contains(&today)
            }
            ViewType::Month => false,
        };

        if should_focus {
            self.pending_focus = Some(AutoFocusRequest {
                date: today,
                time: Some(now.time()),
            });
        }
    }

    /// Render the mini calendar date picker popup as a floating window
    pub(super) fn render_date_picker_popup(&mut self, ctx: &egui::Context) {
        if !self.state.date_picker_state.is_open {
            return;
        }

        let viewing_date = self.state.date_picker_state.viewing_date.unwrap_or(self.current_date);
        let today = Local::now().date_naive();

        let mut is_open = true;
        egui::Window::new("📅 Go to Date")
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.set_max_width(220.0);
                // Month/Year header with navigation
                ui.horizontal(|ui| {
                    if ui.small_button("◀◀").on_hover_text("Previous year").clicked() {
                        if let Some(new_date) = viewing_date.with_year(viewing_date.year() - 1) {
                            self.state.date_picker_state.viewing_date = Some(new_date);
                        }
                    }
                    if ui.small_button("◀").on_hover_text("Previous month").clicked() {
                        self.state.date_picker_state.viewing_date = Some(shift_month(viewing_date, -1));
                    }

                    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        let header = format!("{}", viewing_date.format("%B %Y"));
                        if ui.selectable_label(false, &header).on_hover_text("Click to go to today").clicked() {
                            self.state.date_picker_state.viewing_date = Some(today);
                        }
                    });

                    if ui.small_button("▶").on_hover_text("Next month").clicked() {
                        self.state.date_picker_state.viewing_date = Some(shift_month(viewing_date, 1));
                    }
                    if ui.small_button("▶▶").on_hover_text("Next year").clicked() {
                        if let Some(new_date) = viewing_date.with_year(viewing_date.year() + 1) {
                            self.state.date_picker_state.viewing_date = Some(new_date);
                        }
                    }
                });

                ui.separator();

                // Day of week headers and calendar grid using Grid for alignment
                let day_names = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
                
                egui::Grid::new("date_picker_grid")
                    .num_columns(7)
                    .spacing([4.0, 2.0])
                    .min_col_width(24.0)
                    .show(ui, |ui| {
                        // Header row
                        for name in &day_names {
                            ui.label(egui::RichText::new(*name).small().strong());
                        }
                        ui.end_row();

                        // Calendar grid
                        let first_of_month = viewing_date.with_day(1).unwrap();
                        let start_weekday = first_of_month.weekday().num_days_from_sunday() as i64;

                        // Start from the Sunday before the first of the month
                        let grid_start = first_of_month - chrono::Duration::days(start_weekday);

                        let mut current = grid_start;
                        for _week in 0..6 {
                            for _day in 0..7 {
                                let is_current_month = current.month() == viewing_date.month();
                                let is_today = current == today;
                                let is_selected = current == self.current_date;

                                let day_str = format!("{}", current.day());

                                let text = if is_today {
                                    egui::RichText::new(&day_str).strong().color(egui::Color32::from_rgb(50, 150, 50))
                                } else if !is_current_month {
                                    egui::RichText::new(&day_str).weak()
                                } else {
                                    egui::RichText::new(&day_str)
                                };

                                if ui.selectable_label(is_selected, text).clicked() {
                                    self.current_date = current;
                                    self.state.date_picker_state.close();
                                    self.focus_on_current_time_if_visible();
                                }

                                current += chrono::Duration::days(1);
                            }
                            ui.end_row();

                            // Stop if we've gone past this month
                            if current.month() != viewing_date.month() && current.day() > 7 {
                                break;
                            }
                        }
                    });

                ui.separator();

                // Quick actions
                ui.horizontal(|ui| {
                    if ui.button("Today").clicked() {
                        self.current_date = today;
                        self.state.date_picker_state.close();
                        self.focus_on_current_time_if_visible();
                    }
                });
            });

        if !is_open {
            self.state.date_picker_state.close();
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
