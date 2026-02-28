use super::confirm::ConfirmAction;
use super::state::ViewType;
use super::CalendarApp;
use crate::models::event::Event;
use crate::services::calendar_sync::CalendarSourceService;
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
use chrono::{Datelike, Local};
use std::collections::HashSet;

mod date_picker;

impl CalendarApp {
    /// Handle a delete confirmation request from a view
    fn handle_delete_confirm_request(&mut self, request: DeleteConfirmRequest) {
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
            let mut enabled_synced_sources: Vec<(i64, String)> = CalendarSourceService::new(
                self.context.database().connection(),
            )
            .list_all()
            .unwrap_or_default()
            .into_iter()
            .filter(|source| source.enabled)
            .filter_map(|source| source.id.map(|id| (id, source.name)))
            .collect();
            enabled_synced_sources.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));

            if self
                .selected_synced_source_id
                .is_some_and(|selected_id| {
                    !enabled_synced_sources
                        .iter()
                        .any(|(source_id, _)| *source_id == selected_id)
                })
            {
                self.selected_synced_source_id = None;
            }

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

                ui.separator();

                ui.checkbox(&mut self.show_synced_events_only, "🔒 Synced only")
                    .on_hover_text("Show only synced events (optionally scoped to a selected synced calendar)");

                ui.label("Calendar:");
                egui::ComboBox::from_id_source("synced_source_filter")
                    .selected_text(
                        self.selected_synced_source_id
                            .and_then(|selected_id| {
                                enabled_synced_sources
                                    .iter()
                                    .find(|(source_id, _)| *source_id == selected_id)
                                    .map(|(_, name)| name.clone())
                            })
                            .unwrap_or_else(|| "All synced".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.selected_synced_source_id,
                            None,
                            "All synced",
                        );
                        for (source_id, name) in &enabled_synced_sources {
                            ui.selectable_value(
                                &mut self.selected_synced_source_id,
                                Some(*source_id),
                                name,
                            );
                        }
                    });

                if !self.show_synced_events_only && self.selected_synced_source_id.is_some() {
                    ui.label(
                        egui::RichText::new("(selected source + local events)")
                            .small()
                            .italics(),
                    )
                    .on_hover_text(
                        "With Synced only off, a selected calendar source shows that source plus local events.",
                    );
                }
            });

            ui.separator();

            let synced_source_filter = self.selected_synced_source_id;

            let mut focus_request = self.pending_focus.take();
            match self.current_view {
                ViewType::Day => self.render_day_view(
                    ui,
                    countdown_requests,
                    &active_countdown_events,
                    &mut focus_request,
                    synced_source_filter,
                ),
                ViewType::Week => self.render_week_view(
                    ui,
                    countdown_requests,
                    &active_countdown_events,
                    self.show_ribbon,
                    &mut focus_request,
                    synced_source_filter,
                ),
                ViewType::WorkWeek => self.render_workweek_view(
                    ui,
                    countdown_requests,
                    &active_countdown_events,
                    &mut focus_request,
                    synced_source_filter,
                ),
                ViewType::Month => {
                    self.render_month_view(
                        ui,
                        countdown_requests,
                        &active_countdown_events,
                        synced_source_filter,
                    )
                }
            }
            self.pending_focus = focus_request;
        });
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

            let start_datetime = Local
                .from_local_datetime(&week_start.and_hms_opt(0, 0, 0).unwrap())
                .single()
                .unwrap();
            let end_datetime = Local
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
                let start_datetime = Local
                    .from_local_datetime(&first_day.and_hms_opt(0, 0, 0).unwrap())
                    .single()
                    .unwrap();
                let end_datetime = Local
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