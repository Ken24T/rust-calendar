use super::countdown::OpenEventDialogRequest;
use super::CalendarApp;
use crate::models::event::Event;
use crate::services::countdown::RgbaColor;
use crate::services::event::EventService;
use crate::ui_egui::commands::{CreateEventCommand, UpdateEventCommand};
use crate::ui_egui::dialogs::backup_manager::render_backup_manager_dialog;
use crate::ui_egui::dialogs::category_manager::render_category_manager_dialog;
use crate::ui_egui::dialogs::export_dialog::{render_export_range_dialog, ExportDialogResult};
use crate::ui_egui::dialogs::search_dialog::{render_search_dialog, SearchDialogAction};
use crate::ui_egui::dialogs::template_manager::render_template_manager_dialog;
use crate::ui_egui::event_dialog::{
    render_event_dialog, CountdownCardChanges, EventDialogResult, EventDialogState,
};
use crate::ui_egui::settings_dialog::render_settings_dialog;
use crate::ui_egui::views::CountdownRequest;
use chrono::{Local, NaiveDateTime, TimeZone};

mod theme_handling;

impl CalendarApp {
    pub(super) fn handle_dialogs(&mut self, ctx: &egui::Context) {
        self.capture_root_geometry(ctx);

        if self.show_event_dialog {
            self.ensure_event_dialog_state();
            self.render_event_dialog(ctx);
        }

        if self.show_settings_dialog {
            self.render_settings_dialog(ctx);
        }

        if self.state.show_search_dialog {
            self.render_search_dialog(ctx);
        }

        self.render_theme_dialog(ctx);
        self.render_theme_creator(ctx);
        self.render_about_dialog(ctx);
        self.render_export_range_dialog(ctx);
        self.render_template_manager_dialog(ctx);
        self.render_category_manager_dialog(ctx);

        let should_reload_db =
            render_backup_manager_dialog(ctx, &mut self.state.backup_manager_state);
        if should_reload_db {
            log::info!("Database restored. Application should be restarted.");
            // TODO: Implement graceful restart or reload mechanism
        }
    }

    fn ensure_event_dialog_state(&mut self) {
        if self.event_dialog_state.is_some() {
            return;
        }

        if let Some(event_id) = self.event_to_edit {
            if self.is_synced_event_id(event_id) {
                self.notify_synced_event_read_only();
                self.show_event_dialog = false;
                self.event_to_edit = None;
                self.event_dialog_state = None;
                return;
            }

            let service = self.context.event_service();
            if let Ok(Some(event)) = service.get(event_id) {
                let mut state = EventDialogState::from_event(&event, &self.settings);
                
                // Auto-link countdown card if one exists for this event
                if let Some(card) = self.context.countdown_service().find_card_by_event_id(event_id) {
                    state.link_countdown_card(card.id, card.visuals.clone());
                }
                
                self.event_dialog_state = Some(state);
            } else {
                self.event_dialog_state = Some(EventDialogState::new_event(
                    self.event_dialog_date.unwrap_or(self.current_date),
                    &self.settings,
                ));
            }
        } else {
            self.event_dialog_state = Some(EventDialogState::new_event_with_time(
                self.event_dialog_date.unwrap_or(self.current_date),
                self.event_dialog_time,
                &self.settings,
            ));

            if let (Some(ref mut state), Some(ref rrule)) =
                (&mut self.event_dialog_state, &self.event_dialog_recurrence)
            {
                state.is_recurring = true;
                if rrule.contains("WEEKLY") {
                    state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Weekly;
                } else if rrule.contains("MONTHLY") {
                    state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Monthly;
                } else if rrule.contains("YEARLY") {
                    state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Yearly;
                } else {
                    state.frequency = crate::ui_egui::event_dialog::RecurrenceFrequency::Daily;
                }
            }
        }
    }

    pub(super) fn render_event_dialog(&mut self, ctx: &egui::Context) {
        if self.event_dialog_state.is_none() {
            self.show_event_dialog = false;
            return;
        }

        // Capture old event state before rendering (for undo on updates)
        let old_event: Option<Event> = {
            let state = self.event_dialog_state.as_ref().expect("dialog state just checked");
            if let Some(event_id) = state.event_id {
                // This is an edit - fetch the current state before any changes
                self.context.event_service().get(event_id).ok().flatten()
            } else {
                None
            }
        };

        let (saved_event, card_changes, auto_create_card, was_new_event, event_saved, delete_request) = {
            let state = self
                .event_dialog_state
                .as_mut()
                .expect("dialog state just checked");
            let EventDialogResult {
                saved_event,
                card_changes,
                delete_request,
            } = render_event_dialog(
                ctx,
                state,
                self.context.database(),
                &self.settings,
                &mut self.show_event_dialog,
            );

            // For auto-creating countdown cards, check if the event is in the future
            // For multi-day events, check if the end date/time is in the future
            let now = Local::now();
            let event_end_dt = state.end_date.and_time(state.end_time);
            let event_ends_in_future = event_end_dt > now.naive_local();
            
            let auto_create_card = state.create_countdown 
                && state.event_id.is_none()
                && event_ends_in_future;
            let was_new_event = state.event_id.is_none();
            let event_saved = saved_event.is_some();
            (
                saved_event,
                card_changes,
                auto_create_card,
                was_new_event,
                event_saved,
                delete_request,
            )
        };

        // If delete was requested, trigger confirmation dialog
        if let Some(request) = delete_request {
            use super::confirm::ConfirmAction;
            self.confirm_dialog.request(ConfirmAction::DeleteEvent {
                event_id: request.event_id,
                event_title: request.event_title,
            });
        }

        // Apply card changes if any
        if let Some(changes) = card_changes {
            self.apply_countdown_card_changes(changes);
        }

        if let Some(ref event) = saved_event {
            // Push undo command for the saved event
            if was_new_event {
                // New event created - push CreateEventCommand
                let cmd = CreateEventCommand::new(event.clone());
                self.undo_manager.push(Box::new(cmd));
            } else if let Some(old) = old_event {
                // Existing event updated - push UpdateEventCommand
                let cmd = UpdateEventCommand::new(old, event.clone());
                self.undo_manager.push(Box::new(cmd));
            }

            if auto_create_card {
                self.consume_countdown_requests(vec![CountdownRequest::from_event(event)]);
            }
            self.sync_cards_from_event(event);

            if was_new_event {
                self.focus_on_event(event);
                self.toast_manager.success(format!("Created \"{}\"", event.title));
            } else {
                self.toast_manager.success("Event saved");
            }
        }

        if event_saved || !self.show_event_dialog {
            self.event_dialog_state = None;
            self.event_dialog_time = None;
        }
    }

    /// Apply changes from the unified event dialog to the linked countdown card
    fn apply_countdown_card_changes(&mut self, changes: CountdownCardChanges) {
        let service = self.context.countdown_service_mut();

        // Update title override if different from event title
        service.set_title_override(changes.card_id, None); // Clear override, use event title

        // Update comment (synced with event description)
        service.set_comment(changes.card_id, changes.description);

        // Update start_at from date + time
        let naive_dt = NaiveDateTime::new(changes.start_date, changes.start_time);
        if let Some(start_at) = Local.from_local_datetime(&naive_dt).single() {
            service.set_start_at(changes.card_id, start_at);
        }

        // Update color if provided
        if let Some(color_hex) = changes.color {
            if let Some(rgba) = RgbaColor::from_hex_str(&color_hex) {
                service.set_title_bg_color(changes.card_id, rgba);
            }
        }

        // Update card-specific visuals
        service.set_always_on_top(changes.card_id, changes.always_on_top);
        service.set_title_font_size(changes.card_id, changes.title_font_size);
        service.set_days_font_size(changes.card_id, changes.days_font_size);

        log::info!(
            "Applied countdown card changes from event dialog for card {:?}",
            changes.card_id
        );
    }

    /// Open the event dialog for a countdown card, linking the card for bidirectional updates
    pub(super) fn open_event_dialog_for_card(&mut self, request: OpenEventDialogRequest) {
        // Load the event from the database
        let service = EventService::new(self.context.database().connection());
        let event = match service.get(request.event_id) {
            Ok(Some(event)) => event,
            Ok(None) => {
                log::warn!(
                    "Event {} not found for countdown card {:?}",
                    request.event_id,
                    request.card_id
                );
                return;
            }
            Err(e) => {
                log::error!("Failed to load event {}: {}", request.event_id, e);
                return;
            }
        };

        if self.is_synced_event_id(request.event_id) {
            self.notify_synced_event_read_only();
            return;
        }

        // Create event dialog state from the event
        let mut state = EventDialogState::from_event(&event, &self.settings);

        // Link the countdown card
        state.link_countdown_card(request.card_id, request.visuals);

        // Open the dialog
        self.event_dialog_state = Some(state);
        self.show_event_dialog = true;

        log::info!(
            "Opened event dialog for card {:?} (event {})",
            request.card_id,
            request.event_id
        );
    }

    pub(super) fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        let response = render_settings_dialog(
            ctx,
            &mut self.settings,
            self.context.database(),
            &mut self.state.settings_dialog_state,
            &mut self.show_settings_dialog,
        );

        if response.show_ribbon_changed || response.saved {
            self.show_ribbon = self.settings.show_ribbon;
        }

        if response.saved {
            self.apply_theme_from_db(ctx);
        }
    }

    pub(super) fn render_search_dialog(&mut self, ctx: &egui::Context) {
        let action = render_search_dialog(
            ctx,
            &mut self.state.search_dialog_state,
            self.context.database(),
            &self.active_theme,
            &mut self.state.show_search_dialog,
        );

        match action {
            SearchDialogAction::None => {}
            SearchDialogAction::NavigateToDate(date) => {
                self.current_date = date;
                self.state.show_search_dialog = false;
            }
            SearchDialogAction::EditEvent(event_id) => {
                if self.is_synced_event_id(event_id) {
                    self.notify_synced_event_read_only();
                } else {
                    self.event_to_edit = Some(event_id);
                    self.show_event_dialog = true;
                }
                self.state.show_search_dialog = false;
            }
            SearchDialogAction::CreateCountdown(event_id) => {
                match self.context.event_service().get(event_id) {
                    Ok(Some(event)) => {
                        self.consume_countdown_requests(vec![CountdownRequest::from_event(&event)]);
                    }
                    Ok(None) => {
                        log::warn!(
                            "Search dialog requested countdown for missing event {}",
                            event_id
                        );
                    }
                    Err(err) => {
                        log::error!(
                            "Failed to load event {} for countdown from search dialog: {}",
                            event_id,
                            err
                        );
                    }
                }
                self.state.show_search_dialog = false;
            }
            SearchDialogAction::Close => {
                self.state.show_search_dialog = false;
            }
        }
    }

    fn render_export_range_dialog(&mut self, ctx: &egui::Context) {
        if !self.state.show_export_range_dialog {
            return;
        }

        let result = render_export_range_dialog(
            ctx,
            &mut self.state.export_dialog_state,
        );

        match result {
            ExportDialogResult::None => {}
            ExportDialogResult::Cancelled => {
                self.state.show_export_range_dialog = false;
                self.state.export_dialog_state.reset();
            }
            ExportDialogResult::Export { start, end } => {
                self.state.show_export_range_dialog = false;
                self.export_events_in_range(start, end);
                self.state.export_dialog_state.reset();
            }
        }
    }

    fn render_template_manager_dialog(&mut self, ctx: &egui::Context) {
        render_template_manager_dialog(
            ctx,
            &mut self.state.template_manager_state,
            self.context.database(),
            &self.settings,
        );
    }

    fn render_category_manager_dialog(&mut self, ctx: &egui::Context) {
        let response = render_category_manager_dialog(
            ctx,
            &mut self.state.category_manager_state,
            self.context.database(),
        );

        if response.categories_changed {
            // Categories were modified - could refresh event display if needed
            log::info!("Categories changed");
        }
    }
}
