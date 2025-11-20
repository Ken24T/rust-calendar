use super::CalendarApp;
use crate::services::countdown::CountdownWarningState;
use crate::services::notification::NotificationUrgency;
use chrono::{DateTime, Local};

impl CalendarApp {
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
