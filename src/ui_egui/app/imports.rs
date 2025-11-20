use super::CalendarApp;
use crate::models::event::Event;
use crate::services::countdown::RgbaColor;
use chrono::Local;

impl CalendarApp {
    pub(super) fn handle_ics_import(&mut self, events: Vec<Event>, source_label: &str) {
        if events.is_empty() {
            log::info!("No events found in {} import", source_label);
            return;
        }

        let mut existing_events = self
            .context
            .event_service()
            .list_all()
            .unwrap_or_else(|err| {
                log::error!(
                    "Failed to list existing events before {} import: {}",
                    source_label,
                    err
                );
                Vec::new()
            });

        if self.settings.edit_before_import {
            let first_event = events[0].clone();
            let remaining = events.len().saturating_sub(1);

            if Self::is_duplicate_event(&existing_events, &first_event) {
                log::info!(
                    "Skipping duplicate event (edit mode) from {}: '{}'",
                    source_label,
                    first_event.title
                );
            } else {
                match self.context.event_service().create(first_event.clone()) {
                    Ok(created_event) => {
                        self.focus_on_event(&created_event);
                        if let Some(event_id) = created_event.id {
                            self.event_to_edit = Some(event_id);
                            self.show_event_dialog = true;
                            log::info!(
                                "Opening event '{}' for editing from {}",
                                created_event.title,
                                source_label
                            );
                        }
                        existing_events.push(created_event);
                    }
                    Err(err) => {
                        log::error!(
                            "Failed to create event for editing from {}: {}",
                            source_label,
                            err
                        );
                    }
                }
            }

            if remaining > 0 {
                log::info!(
                    "Note: Only the first event was opened for editing from {}. {} other event(s) were not imported.",
                    source_label,
                    remaining
                );
            }

            return;
        }

        let mut imported_count = 0;
        let mut failed_count = 0;
        let mut duplicate_count = 0;

        for event in events {
            let event_title = event.title.clone();

            if Self::is_duplicate_event(&existing_events, &event) {
                log::info!(
                    "Skipping duplicate event from {}: '{}'",
                    source_label,
                    event_title
                );
                duplicate_count += 1;
                continue;
            }

            match self.context.event_service().create(event) {
                Ok(created_event) => {
                    self.focus_on_event(&created_event);
                    imported_count += 1;

                    if self.settings.auto_create_countdown_on_import
                        && created_event.start > Local::now()
                    {
                        if let Some(event_id) = created_event.id {
                            let event_color = created_event.color.as_ref().and_then(|hex| {
                                if hex.starts_with('#') && hex.len() == 7 {
                                    u32::from_str_radix(&hex[1..], 16).ok().map(|rgb| {
                                        let r = ((rgb >> 16) & 0xFF) as u8;
                                        let g = ((rgb >> 8) & 0xFF) as u8;
                                        let b = (rgb & 0xFF) as u8;
                                        RgbaColor::new(r, g, b, 255)
                                    })
                                } else {
                                    None
                                }
                            });

                            let location_label = created_event
                                .location
                                .as_deref()
                                .map(str::trim)
                                .filter(|loc| !loc.is_empty())
                                .map(|loc| loc.to_string());

                            let card_id = self.context.countdown_service_mut().create_card(
                                Some(event_id),
                                created_event.title.clone(),
                                created_event.start,
                                event_color,
                                created_event.description.clone(),
                                self.settings.default_card_width,
                                self.settings.default_card_height,
                            );

                            if let Some(label) = location_label {
                                self.context
                                    .countdown_service_mut()
                                    .set_auto_title_override(card_id, Some(label));
                            }
                        }
                    }

                    existing_events.push(created_event.clone());
                }
                Err(err) => {
                    log::error!(
                        "Failed to import event '{}' from {}: {}",
                        event_title,
                        source_label,
                        err
                    );
                    failed_count += 1;
                }
            }
        }

        if duplicate_count > 0 {
            log::info!(
                "{} import complete: {} events imported, {} duplicates skipped, {} failed",
                source_label,
                imported_count,
                duplicate_count,
                failed_count
            );
        } else {
            log::info!(
                "{} import complete: {} events imported, {} failed",
                source_label,
                imported_count,
                failed_count
            );
        }
    }

    fn is_duplicate_event(existing_events: &[Event], candidate: &Event) -> bool {
        existing_events.iter().any(|event| {
            event.title == candidate.title
                && event.start == candidate.start
                && event.end == candidate.end
        })
    }
}
