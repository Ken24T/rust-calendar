use super::CalendarApp;
use crate::services::countdown::CountdownWarningState;
use crate::services::notification::NotificationUrgency;
use chrono::{DateTime, Local};

impl CalendarApp {
    /// Check countdown timers for notification triggers and show system notifications.
    /// Returns true if any changes occurred that require a repaint.
    pub(super) fn check_and_show_countdown_notifications(&mut self, ctx: &egui::Context) {
        let now = Local::now();

        // Check for notification triggers (warning state transitions)
        let notification_triggers = self
            .context
            .countdown_service_mut()
            .check_notification_triggers(now);

        if notification_triggers.is_empty() {
            return;
        }

        // Get notification config to check if system notifications are enabled
        let notification_config = self.context.countdown_service().notification_config();

        if notification_config.use_system_notifications {
            for (card_id, _old_state, new_state) in &notification_triggers {
                let card_info = self
                    .context
                    .countdown_service()
                    .cards()
                    .iter()
                    .find(|c| c.id == *card_id)
                    .map(|card| (card.effective_title().to_owned(), card.start_at));

                if let Some((title, start_at)) = card_info {
                    let (message, urgency) =
                        Self::notification_message_for_state(*new_state, start_at, now);

                    if let Err(e) = self
                        .context
                        .notification_service_mut()
                        .show_countdown_alert(&title, &message, urgency)
                    {
                        log::warn!("Failed to show system notification: {}", e);
                    } else {
                        log::info!(
                            "Showed system notification for card {:?} ({}) - state: {:?}",
                            card_id,
                            title,
                            new_state
                        );
                    }
                }
            }
        }

        // Log all transitions
        for (card_id, old_state, new_state) in notification_triggers {
            log::info!(
                "Countdown notification trigger: card {:?} transitioned from {:?} to {:?}",
                card_id,
                old_state,
                new_state
            );
        }

        ctx.request_repaint();
    }

    /// Generate a countdown notification message plus urgency category
    pub(super) fn notification_message_for_state(
        state: CountdownWarningState,
        event_time: DateTime<Local>,
        now: DateTime<Local>,
    ) -> (String, NotificationUrgency) {
        let remaining = event_time.signed_duration_since(now);

        match state {
            CountdownWarningState::Critical => {
                let minutes = remaining.num_minutes();
                let message = if minutes > 0 {
                    format!(
                        "Starting in {} minute{}",
                        minutes,
                        if minutes == 1 { "" } else { "s" }
                    )
                } else {
                    "Starting very soon!".to_string()
                };
                (message, NotificationUrgency::Critical)
            }
            CountdownWarningState::Imminent => {
                let hours = remaining.num_hours();
                let minutes = remaining.num_minutes() % 60;
                let message = if hours > 0 {
                    format!(
                        "Starting in {} hour{} {} minute{}",
                        hours,
                        if hours == 1 { "" } else { "s" },
                        minutes,
                        if minutes == 1 { "" } else { "s" }
                    )
                } else {
                    format!(
                        "Starting in {} minute{}",
                        minutes,
                        if minutes == 1 { "" } else { "s" }
                    )
                };
                (message, NotificationUrgency::Critical)
            }
            CountdownWarningState::Approaching => {
                let hours = remaining.num_hours();
                let message = format!(
                    "Starting in {} hour{}",
                    hours,
                    if hours == 1 { "" } else { "s" }
                );
                (message, NotificationUrgency::Normal)
            }
            CountdownWarningState::Starting => (
                "Event is starting now!".to_string(),
                NotificationUrgency::Critical,
            ),
            CountdownWarningState::Normal => {
                ("Event approaching".to_string(), NotificationUrgency::Normal)
            }
        }
    }
}
