use super::state::ViewType;
use super::CalendarApp;
use crate::models::event::Event;
use crate::services::calendar_sync::CalendarSourceService;
use crate::ui_egui::views::week_view::WeekView;
use crate::ui_egui::views::workweek_view::WorkWeekView;
use crate::ui_egui::views::{AutoFocusRequest, CountdownRequest};
use chrono::Local;
use std::collections::HashSet;

mod date_picker;

mod view_rendering;

impl CalendarApp {
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
