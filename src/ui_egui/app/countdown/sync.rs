use super::CalendarApp;
use crate::models::event::Event;
use crate::services::countdown::CountdownService;
use crate::services::database::Database;
use crate::services::event::EventService;
use std::collections::HashSet;

impl CalendarApp {
    pub(crate) fn hydrate_countdown_titles_from_events(
        countdown_service: &mut CountdownService,
        database: &'static Database,
    ) {
        let mut seen_ids = HashSet::new();
        let mut event_ids = Vec::new();

        for card in countdown_service.cards() {
            if let Some(event_id) = card.event_id {
                if seen_ids.insert(event_id) {
                    event_ids.push(event_id);
                }
            }
        }

        if event_ids.is_empty() {
            return;
        }

        let event_service = EventService::new(database.connection());
        for event_id in event_ids {
            match event_service.get(event_id) {
                Ok(Some(event)) => {
                    let location_label = event
                        .location
                        .as_deref()
                        .map(str::trim)
                        .filter(|loc| !loc.is_empty())
                        .map(|loc| loc.to_string());

                    countdown_service.sync_title_for_event(event_id, event.title.clone());
                    countdown_service.sync_title_override_for_event(event_id, location_label);
                }
                Ok(None) => {
                    log::warn!(
                        "Countdown card references missing event id {} while syncing titles",
                        event_id
                    );
                }
                Err(err) => {
                    log::error!(
                        "Failed to load event {} while syncing countdown titles: {}",
                        event_id,
                        err
                    );
                }
            }
        }
    }

    pub(crate) fn flush_pending_event_bodies(&mut self) {
        let updates = self.countdown_ui.drain_pending_event_bodies();
        if updates.is_empty() {
            return;
        }

        for (event_id, body) in updates {
            match self.context.event_service().get(event_id) {
                Ok(Some(mut event)) => {
                    event.description = body.clone();
                    if let Err(err) = self.context.event_service().update(&event) {
                        log::error!(
                            "Failed to update event {} body from countdown settings: {err}",
                            event_id
                        );
                        continue;
                    }
                    self.context
                        .countdown_service_mut()
                        .sync_comment_for_event(event_id, body.clone());
                }
                Ok(None) => {
                    log::warn!(
                        "Countdown requested update for missing event id {}",
                        event_id
                    );
                }
                Err(err) => {
                    log::error!(
                        "Failed to load event {} for countdown body sync: {err}",
                        event_id
                    );
                }
            }
        }
    }

    pub(crate) fn sync_cards_from_event(&mut self, event: &Event) {
        if let Some(event_id) = event.id {
            let location_label = event
                .location
                .as_deref()
                .map(str::trim)
                .filter(|loc| !loc.is_empty())
                .map(|loc| loc.to_string());

            // Parse the event color from hex string
            let event_color = event.color.as_ref().and_then(|hex| {
                crate::services::countdown::RgbaColor::from_hex_str(hex)
            });

            let countdown_service = self.context.countdown_service_mut();
            countdown_service.sync_title_for_event(event_id, event.title.clone());
            countdown_service.sync_title_override_for_event(event_id, location_label);
            countdown_service.sync_comment_for_event(event_id, event.description.clone());
            countdown_service.sync_event_color_for_event(event_id, event_color);
            countdown_service.sync_start_at_for_event(event_id, event.start);
        }
    }
}
