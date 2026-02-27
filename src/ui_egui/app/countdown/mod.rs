mod container;
mod refresh;
mod render;
mod settings;
mod state;
mod sync;

pub(super) use state::{CountdownUiState, OpenEventDialogRequest};

use super::CalendarApp;
use crate::services::countdown::{CountdownCardGeometry, RgbaColor};
use crate::ui_egui::views::CountdownRequest;
use chrono::Local;
use directories::ProjectDirs;
use std::path::PathBuf;

fn normalized_non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

fn compose_countdown_body(
    body: Option<String>,
    display_label: Option<String>,
    sync_source_name: Option<String>,
) -> Option<String> {
    let mut sections = Vec::new();

    if let Some(description) = normalized_non_empty(body) {
        sections.push(description);
    }

    if let Some(label) = normalized_non_empty(display_label) {
        sections.push(format!("Location: {}", label));
    }

    if let Some(source_name) = normalized_non_empty(sync_source_name) {
        sections.push(format!("Synced source: {}", source_name));
    }

    if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    }
}

fn has_recurrence_rule(rule: Option<&str>) -> bool {
    rule.map(str::trim)
        .is_some_and(|value| !value.is_empty() && value != "None")
}

impl CalendarApp {
    pub(super) fn resolve_countdown_storage_path() -> PathBuf {
        if let Some(dirs) = ProjectDirs::from("com", "RustCalendar", "CalendarApp") {
            let dir = dirs.data_dir();
            std::fs::create_dir_all(dir).ok();
            dir.join("countdowns.json")
        } else {
            log::warn!("Unable to resolve project directory; using current dir for countdowns");
            PathBuf::from("countdowns.json")
        }
    }

    pub(super) fn persist_countdowns_if_needed(&mut self) {
        if !self.context.countdown_service().is_dirty() {
            return;
        }

        // Get connection first, then save to database
        let conn = self.context.database().connection();
        if let Err(err) = self.context.countdown_service_mut().save_to_database(conn) {
            log::error!("Failed to persist countdown cards to database: {err:?}");
        } else {
            self.context.countdown_service_mut().mark_clean();
        }
    }

    pub(super) fn consume_countdown_requests(&mut self, requests: Vec<CountdownRequest>) {
        let now = Local::now();
        for request in requests {
            let CountdownRequest {
                event_id,
                title,
                start_at,
                end_at,
                color,
                body,
                display_label,
            } = request;

            let (effective_start_at, effective_end_at) = event_id
                .and_then(|id| self.context.event_service().get(id).ok().flatten())
                .and_then(|event| {
                    let is_non_recurring = !has_recurrence_rule(event.recurrence_rule.as_deref());
                    let is_multi_day = event.start.date_naive() != event.end.date_naive();
                    if is_non_recurring && is_multi_day {
                        Some((event.start, event.end))
                    } else {
                        None
                    }
                })
                .unwrap_or((start_at, end_at));

            let target_at = if effective_start_at > now {
                effective_start_at
            } else if effective_end_at > now {
                effective_end_at
            } else {
                log::info!(
                    "Skipping countdown for finished event {:?} ({}): start {:?}, end {:?}",
                    event_id,
                    title,
                    effective_start_at,
                    effective_end_at
                );
                continue;
            };

            // Check if a card already exists for this event
            log::debug!(
                "Checking for existing card for event {:?}, total cards in service: {}",
                event_id,
                self.context.countdown_service().cards().len()
            );
            if let Some(existing_card) = self
                .context
                .countdown_service()
                .cards()
                .iter()
                .find(|card| card.event_id == event_id)
            {
                log::info!(
                    "Card already exists for event {:?} ({}), reopening card {:?} with geometry: {}x{}",
                    event_id,
                    title,
                    existing_card.id,
                    existing_card.geometry.width,
                    existing_card.geometry.height
                );
                self.countdown_ui
                    .mark_card_pending(existing_card.id, existing_card.geometry);
                continue;
            }

            let event_color = color.as_deref().and_then(RgbaColor::from_hex_str);
            let sync_source_name = event_id.and_then(|id| self.synced_event_source_name(id));
            let event_body = compose_countdown_body(body, display_label, sync_source_name);

            log::info!(
                "Creating countdown card with dimensions from settings: width={}, height={}",
                self.settings.default_card_width,
                self.settings.default_card_height
            );
            let card_id = self.context.countdown_service_mut().create_card(
                event_id,
                title,
                target_at,
                Some(effective_start_at),
                Some(effective_end_at),
                event_color,
                event_body,
                self.settings.default_card_width,
                self.settings.default_card_height,
            );

            // Card title defaults to event title, user can override in settings
            let geometry = self
                .context
                .countdown_service()
                .cards()
                .iter()
                .find(|card| card.id == card_id)
                .map(|card| card.geometry)
                .unwrap_or(CountdownCardGeometry {
                    x: 50.0,
                    y: 50.0,
                    width: self.settings.default_card_width,
                    height: self.settings.default_card_height,
                });
            self.countdown_ui.mark_card_pending(card_id, geometry);
            log::info!("created countdown card {:?}", card_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compose_countdown_body;

    #[test]
    fn compose_countdown_body_merges_description_location_and_source() {
        let result = compose_countdown_body(
            Some("Bring projector".to_string()),
            Some("Room 4".to_string()),
            Some("Google Work".to_string()),
        );

        assert_eq!(
            result.as_deref(),
            Some("Bring projector\n\nLocation: Room 4\n\nSynced source: Google Work")
        );
    }

    #[test]
    fn compose_countdown_body_ignores_empty_segments() {
        let result = compose_countdown_body(
            Some("   ".to_string()),
            Some("Office".to_string()),
            None,
        );

        assert_eq!(result.as_deref(), Some("Location: Office"));
    }
}
