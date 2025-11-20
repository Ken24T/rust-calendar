use super::state::ViewType;
use super::CalendarApp;
use crate::models::event::Event;
use crate::ui_egui::event_dialog::EventDialogState;
use crate::ui_egui::views::day_view::DayView;
use crate::ui_egui::views::month_view::MonthView;
use crate::ui_egui::views::week_view::WeekView;
use crate::ui_egui::views::workweek_view::WorkWeekView;
use crate::ui_egui::views::{AutoFocusRequest, CountdownRequest};
use chrono::{Datelike, Local};
use std::collections::HashSet;

impl CalendarApp {
    pub(super) fn render_day_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        focus_request: &mut Option<AutoFocusRequest>,
    ) {
        if let Some(clicked_event) = DayView::show(
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
        ) {
            self.event_dialog_state =
                Some(EventDialogState::from_event(&clicked_event, &self.settings));
            self.show_event_dialog = true;
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

        if let Some(clicked_event) = WeekView::show(
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
        ) {
            self.event_dialog_state =
                Some(EventDialogState::from_event(&clicked_event, &self.settings));
            self.show_event_dialog = true;
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

        if let Some(clicked_event) = WorkWeekView::show(
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
        ) {
            self.event_dialog_state =
                Some(EventDialogState::from_event(&clicked_event, &self.settings));
            self.show_event_dialog = true;
        }
    }

    pub(super) fn render_month_view(&mut self, ui: &mut egui::Ui) {
        MonthView::show(
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
}
