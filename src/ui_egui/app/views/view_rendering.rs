//! Individual view rendering methods for `CalendarApp`.
//!
//! Contains the per-view render methods (`render_day_view`, `render_week_view`,
//! `render_workweek_view`, `render_month_view`), the shared
//! `handle_timed_view_result` helper, and `handle_delete_confirm_request`.

use super::super::confirm::ConfirmAction;
use super::super::state::ViewType;
use super::super::CalendarApp;
use crate::ui_egui::commands::UpdateEventCommand;
use crate::ui_egui::views::day_view::DayView;
use crate::ui_egui::views::month_view::{MonthView, MonthViewAction};
use crate::ui_egui::views::week_shared::{DeleteConfirmRequest, EventInteractionResult};
use crate::ui_egui::views::week_view::WeekView;
use crate::ui_egui::views::workweek_view::WorkWeekView;
use crate::ui_egui::views::{
    filter_events_by_category, is_ribbon_event, is_synced_event, load_synced_event_ids,
    AutoFocusRequest, CountdownRequest,
};
use chrono::Datelike;
use std::collections::HashSet;

impl CalendarApp {
    /// Handle a delete confirmation request from a view
    pub(in crate::ui_egui::app) fn handle_delete_confirm_request(
        &mut self,
        request: DeleteConfirmRequest,
    ) {
        if self.is_synced_event_id(request.event_id) {
            self.notify_synced_event_read_only();
            return;
        }

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

    /// Handle the common result fields returned by day, week, and workweek views.
    fn handle_timed_view_result(&mut self, view_result: EventInteractionResult) {
        // Handle clicked event - open edit dialog
        if let Some(clicked_event) = view_result.event_to_edit {
            if let Some(event_id) = clicked_event.id {
                if self.is_synced_event_id(event_id) {
                    self.notify_synced_event_read_only();
                } else {
                    self.event_to_edit = Some(event_id);
                    self.show_event_dialog = true;
                }
            }
        }

        // Handle delete confirmation request
        if let Some(request) = view_result.delete_confirm_request {
            self.handle_delete_confirm_request(request);
        }

        // Handle template selection from context menu
        if let Some((template_id, date, time)) = view_result.template_selection {
            self.create_event_from_template_with_date(template_id, date, time);
        }

        // Handle deleted events - remove countdown cards (legacy path)
        for event_id in view_result.deleted_event_ids {
            self.context
                .countdown_service_mut()
                .remove_cards_for_event(event_id);
        }

        // Handle moved events - sync countdown cards
        for event in view_result.moved_events {
            self.sync_cards_from_event(&event);
        }

        // Handle undo requests from drag/resize operations
        for (old_event, new_event) in view_result.undo_requests {
            let cmd: Box<dyn crate::ui_egui::commands::Command + Send + Sync> =
                Box::new(UpdateEventCommand::new(old_event, new_event));
            self.undo_manager.push(cmd);
        }
    }

    pub(super) fn render_day_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        focus_request: &mut Option<AutoFocusRequest>,
        synced_source_id: Option<i64>,
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
            self.active_category_filter.as_deref(),
            self.show_synced_events_only,
            synced_source_id,
        );

        self.handle_timed_view_result(view_result);
    }

    pub(super) fn render_week_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        show_ribbon: bool,
        focus_request: &mut Option<AutoFocusRequest>,
        synced_source_id: Option<i64>,
    ) {
        let all_day_events = if show_ribbon {
            use chrono::TimeZone;
            let event_service = self.context.event_service();

            let weekday = self.current_date.weekday().num_days_from_sunday() as i64;
            let offset = (weekday - self.settings.first_day_of_week as i64 + 7) % 7;
            let week_start = self.current_date - chrono::Duration::days(offset);
            let week_end = week_start + chrono::Duration::days(6);

            let start_datetime = chrono::Local
                .from_local_datetime(&week_start.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .unwrap();
            let end_datetime = chrono::Local
                .from_local_datetime(&week_end.and_hms_opt(23, 59, 59).unwrap())
                .single()
                .unwrap();

            let all_events = event_service
                .expand_recurring_events(start_datetime, end_datetime)
                .unwrap_or_default()
                .into_iter()
                .filter(is_ribbon_event)
                .collect::<Vec<_>>();

            let all_events = filter_events_by_category(all_events, self.active_category_filter.as_deref());
            if self.show_synced_events_only {
                let synced_event_ids = load_synced_event_ids(self.context.database(), synced_source_id);
                all_events
                    .into_iter()
                    .filter(|event| is_synced_event(event.id, &synced_event_ids))
                    .collect::<Vec<_>>()
            } else {
                all_events
            }
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
            self.active_category_filter.as_deref(),
            self.show_synced_events_only,
            synced_source_id,
        );

        self.handle_timed_view_result(view_result);
    }

    pub(super) fn render_workweek_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        focus_request: &mut Option<AutoFocusRequest>,
        synced_source_id: Option<i64>,
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
                let start_datetime = chrono::Local
                    .from_local_datetime(&first_day.and_hms_opt(0, 0, 0).unwrap())
                    .single()
                    .unwrap();
                let end_datetime = chrono::Local
                    .from_local_datetime(&last_day.and_hms_opt(23, 59, 59).unwrap())
                    .single()
                    .unwrap();

                let all_events = event_service
                    .expand_recurring_events(start_datetime, end_datetime)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(is_ribbon_event)
                    .collect::<Vec<_>>();

                let all_events = filter_events_by_category(all_events, self.active_category_filter.as_deref());
                if self.show_synced_events_only {
                    let synced_event_ids = load_synced_event_ids(self.context.database(), synced_source_id);
                    all_events
                        .into_iter()
                        .filter(|event| is_synced_event(event.id, &synced_event_ids))
                        .collect::<Vec<_>>()
                } else {
                    all_events
                }
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
            self.active_category_filter.as_deref(),
            self.show_synced_events_only,
            synced_source_id,
        );

        self.handle_timed_view_result(view_result);
    }

    pub(super) fn render_month_view(
        &mut self,
        ui: &mut egui::Ui,
        countdown_requests: &mut Vec<CountdownRequest>,
        active_countdown_events: &HashSet<i64>,
        synced_source_id: Option<i64>,
    ) {
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
            countdown_requests,
            active_countdown_events,
            self.active_category_filter.as_deref(),
            self.show_synced_events_only,
            synced_source_id,
        );
        
        // Handle month view actions
        match result.action {
            MonthViewAction::SwitchToDayView(date) => {
                self.current_date = date;
                self.current_view = ViewType::Day;
            }
            MonthViewAction::SwitchToDefaultView(date) => {
                self.current_date = date;
                // Parse the default view from settings
                self.current_view = match self.settings.current_view.as_str() {
                    "Day" => ViewType::Day,
                    "Week" => ViewType::Week,
                    "WorkWeek" => ViewType::WorkWeek,
                    "Month" => ViewType::Day, // If already in Month, go to Day
                    _ => ViewType::Day, // Default fallback
                };
            }
            MonthViewAction::CreateFromTemplate(template_id, date) => {
                self.create_event_from_template_with_date(template_id, date, None);
            }
            MonthViewAction::None => {}
        }
        
        // Handle delete confirmation request
        if let Some(request) = result.delete_confirm_request {
            self.handle_delete_confirm_request(request);
        }
    }
}
