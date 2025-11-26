use super::CalendarApp;
use crate::models::event::Event;
use crate::services::countdown::{CountdownCardGeometry, RgbaColor};
use chrono::Local;

impl CalendarApp {
    pub(super) fn handle_file_drops(&mut self, ctx: &egui::Context) {
        // Collect files outside of ctx.input to avoid borrow issues
        let dropped_files: Vec<_> = ctx.input(|i| {
            i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect()
        });
        
        if dropped_files.is_empty() {
            return;
        }
        
        eprintln!("[COUNTDOWN DEBUG] handle_file_drops: {} files dropped, self ptr={:p}", dropped_files.len(), self);
        log::info!("[COUNTDOWN DEBUG] handle_file_drops: {} files dropped", dropped_files.len());
        
        for path in dropped_files {

            eprintln!("[COUNTDOWN DEBUG] Processing dropped file: {:?}", path);
            log::info!("[COUNTDOWN DEBUG] Processing dropped file: {:?}", path);

            match std::fs::read_to_string(&path) {
                Ok(ics_content) => {
                    if !(ics_content.contains("BEGIN:VCALENDAR")
                        || ics_content.contains("BEGIN:VEVENT"))
                    {
                        log::warn!(
                            "Dropped file {:?} does not look like an iCalendar file",
                            path
                        );
                        continue;
                    }

                    use crate::services::icalendar::import;

                    match import::from_str(&ics_content) {
                        Ok(events) => {
                            eprintln!("[COUNTDOWN DEBUG] Parsed {} events from ICS file", events.len());
                            log::info!("[COUNTDOWN DEBUG] Parsed {} events from ICS file", events.len());
                            eprintln!("[COUNTDOWN DEBUG] Cards in service BEFORE import: {}", self.context.countdown_service().cards().len());
                            log::info!("[COUNTDOWN DEBUG] Cards in service BEFORE import: {}", self.context.countdown_service().cards().len());
                            self.handle_ics_import(events, "drag-and-drop");
                            eprintln!("[COUNTDOWN DEBUG] Cards in service AFTER import: {}", self.context.countdown_service().cards().len());
                            log::info!("[COUNTDOWN DEBUG] Cards in service AFTER import: {}", self.context.countdown_service().cards().len());
                        }
                        Err(e) => {
                            log::error!("Failed to parse dropped ICS file {:?}: {}", path, e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to read dropped file {:?}: {}", path, e);
                }
            }
        }
    }

    pub(super) fn handle_ics_import(&mut self, events: Vec<Event>, source_label: &str) {
        eprintln!(
            "[COUNTDOWN DEBUG] handle_ics_import START: {} events, edit_before_import={}, auto_create={}",
            events.len(),
            self.settings.edit_before_import,
            self.settings.auto_create_countdown_on_import
        );
        log::info!(
            "[COUNTDOWN DEBUG] handle_ics_import START: {} events from {}, edit_before_import={}, auto_create_countdown_on_import={}",
            events.len(),
            source_label,
            self.settings.edit_before_import,
            self.settings.auto_create_countdown_on_import
        );

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

                            // Also create countdown card if enabled and event is in the future
                            log::info!(
                                "[COUNTDOWN DEBUG] edit_before_import path: checking auto_create. setting={}, event_start={}, now={}, is_future={}",
                                self.settings.auto_create_countdown_on_import,
                                created_event.start,
                                Local::now(),
                                created_event.start > Local::now()
                            );
                            if self.settings.auto_create_countdown_on_import
                                && created_event.start > Local::now()
                            {
                                log::info!("[COUNTDOWN DEBUG] edit_before_import path: CALLING create_countdown_card_for_event");
                                self.create_countdown_card_for_event(&created_event);
                            } else {
                                log::info!("[COUNTDOWN DEBUG] edit_before_import path: NOT creating card (conditions not met)");
                            }
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

                    // Auto-create countdown card if enabled and event is in the future
                    log::info!(
                        "[COUNTDOWN DEBUG] batch import path: checking auto_create. setting={}, event_start={}, now={}, is_future={}",
                        self.settings.auto_create_countdown_on_import,
                        created_event.start,
                        Local::now(),
                        created_event.start > Local::now()
                    );
                    if self.settings.auto_create_countdown_on_import
                        && created_event.start > Local::now()
                    {
                        log::info!("[COUNTDOWN DEBUG] batch import path: CALLING create_countdown_card_for_event");
                        self.create_countdown_card_for_event(&created_event);
                    } else {
                        log::info!("[COUNTDOWN DEBUG] batch import path: NOT creating card (conditions not met)");
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

    /// Creates a countdown card for the given event and marks it for display.
    fn create_countdown_card_for_event(&mut self, event: &Event) {
        eprintln!(
            "[COUNTDOWN DEBUG] create_countdown_card_for_event ENTER: event_id={:?}, title='{}'",
            event.id,
            event.title
        );
        log::info!(
            "[COUNTDOWN DEBUG] create_countdown_card_for_event ENTER: event_id={:?}, title='{}', start={}",
            event.id,
            event.title,
            event.start
        );
        
        let Some(event_id) = event.id else {
            log::warn!("[COUNTDOWN DEBUG] Cannot create countdown card: event has no ID");
            return;
        };

        let event_color = event.color.as_ref().and_then(|hex| {
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

        let location_label = event
            .location
            .as_deref()
            .map(str::trim)
            .filter(|loc| !loc.is_empty())
            .map(|loc| loc.to_string());

        log::info!(
            "[COUNTDOWN DEBUG] About to call countdown_service_mut().create_card with width={}, height={}",
            self.settings.default_card_width,
            self.settings.default_card_height
        );
        
        let card_id = self.context.countdown_service_mut().create_card(
            Some(event_id),
            event.title.clone(),
            event.start,
            event_color,
            event.description.clone(),
            self.settings.default_card_width,
            self.settings.default_card_height,
        );

        eprintln!(
            "[COUNTDOWN DEBUG] create_card returned: card_id={:?}, total cards: {}, self ptr={:p}, service ptr={:p}",
            card_id,
            self.context.countdown_service().cards().len(),
            self,
            self.context.countdown_service() as *const _
        );
        log::info!(
            "[COUNTDOWN DEBUG] create_card returned: card_id={:?}, total cards in service now: {}",
            card_id,
            self.context.countdown_service().cards().len()
        );

        if let Some(label) = location_label {
            self.context
                .countdown_service_mut()
                .set_auto_title_override(card_id, Some(label));
        }

        // Mark card for display so it opens the countdown window
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
        log::info!(
            "[COUNTDOWN DEBUG] About to call mark_card_pending: card_id={:?}, geometry={:?}",
            card_id,
            geometry
        );
        self.countdown_ui.mark_card_pending(card_id, geometry);
        log::info!(
            "[COUNTDOWN DEBUG] create_countdown_card_for_event COMPLETE: card {:?} for event '{}' created and marked pending",
            card_id,
            event.title
        );
    }
}
