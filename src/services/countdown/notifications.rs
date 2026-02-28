//! Notification triggers and auto-dismiss logic for countdown cards.
//!
//! Monitors warning-state transitions to fire desktop notifications
//! and automatically removes cards whose events have already started.

use chrono::{DateTime, Local};

use super::models::{
    CountdownAutoDismissConfig, CountdownCardId, CountdownNotificationConfig,
    CountdownWarningState,
};
use super::service::CountdownService;

impl CountdownService {
    /// Recomputes days remaining for every card, returning the ones that
    /// changed so the UI can re-render or animate them.
    pub fn refresh_days_remaining(&mut self, now: DateTime<Local>) -> Vec<(CountdownCardId, i64)> {
        let mut changed = Vec::new();
        for card in &mut self.cards {
            let computed = card.compute_days_remaining(now);
            if card.last_computed_days != Some(computed) {
                card.record_days_remaining(computed);
                changed.push((card.id, computed));
                self.dirty = true;
            }
        }
        changed
    }

    /// Returns notification config for external use.
    pub fn notification_config(&self) -> &CountdownNotificationConfig {
        &self.notification_config
    }

    /// Returns mutable notification config for settings updates.
    #[allow(dead_code)]
    pub fn notification_config_mut(&mut self) -> &mut CountdownNotificationConfig {
        self.dirty = true;
        &mut self.notification_config
    }

    /// Returns auto-dismiss defaults for external use.
    #[allow(dead_code)]
    pub fn auto_dismiss_defaults(&self) -> &CountdownAutoDismissConfig {
        &self.auto_dismiss_defaults
    }

    /// Returns mutable auto-dismiss defaults for settings updates.
    #[allow(dead_code)]
    pub fn auto_dismiss_defaults_mut(&mut self) -> &mut CountdownAutoDismissConfig {
        self.dirty = true;
        &mut self.auto_dismiss_defaults
    }

    /// Check for warning state transitions that should trigger notifications.
    /// Returns tuples of (card_id, old_state, new_state) for cards that changed state.
    pub fn check_notification_triggers(
        &mut self,
        now: DateTime<Local>,
    ) -> Vec<(
        CountdownCardId,
        Option<CountdownWarningState>,
        CountdownWarningState,
    )> {
        if !self.notification_config.enabled {
            return Vec::new();
        }

        let mut transitions = Vec::new();
        let thresholds = &self.notification_config.warning_thresholds;

        for card in &mut self.cards {
            let new_state = card.warning_state(now, thresholds);
            let old_state = card.last_warning_state;

            // Detect state transition (including first time calculation)
            if old_state != Some(new_state) {
                // Only notify on state increases in urgency, not decreases
                // (e.g., notify when going from Normal->Approaching, but not Approaching->Normal)
                let should_notify = match (old_state, new_state) {
                    (None, CountdownWarningState::Normal) => false, // Initial state, not urgent
                    (None, _) => true, // Initial state and it's urgent
                    (Some(old), new) if new as u8 > old as u8 => true, // Urgency increased
                    _ => false,        // Urgency decreased or stayed same
                };

                if should_notify {
                    transitions.push((card.id, old_state, new_state));
                    card.last_notification_time = Some(now);
                }

                // Always update the last known state
                card.last_warning_state = Some(new_state);
                self.dirty = true;
            }
        }

        transitions
    }

    /// Check for cards that should be auto-dismissed.
    /// Returns IDs of cards that were dismissed.
    pub fn check_auto_dismiss(&mut self, now: DateTime<Local>) -> Vec<CountdownCardId> {
        let mut to_dismiss = Vec::new();

        // Collect IDs of cards to dismiss (can't remove while iterating)
        for card in &self.cards {
            if card.should_auto_dismiss(now) {
                log::info!(
                    "Auto-dismiss triggered for card {:?} ({}): event started {} seconds ago",
                    card.id,
                    card.event_title,
                    (now - card.start_at).num_seconds()
                );
                to_dismiss.push(card.id);
            }
        }

        // Remove dismissed cards
        for id in &to_dismiss {
            self.remove_card(*id);
        }

        to_dismiss
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Local};

    use super::super::service::CountdownService;

    #[test]
    fn refresh_days_remaining_detects_changes() {
        let mut svc = CountdownService::new();
        let t = Local::now() + Duration::days(10);
        svc.create_card(None, "Evt", t, None, None, None, None, 120.0, 110.0);
        svc.mark_clean();

        let changes = svc.refresh_days_remaining(Local::now());
        // First refresh always detects a change (last_computed_days starts as None)
        assert!(!changes.is_empty());
        assert!(svc.is_dirty());

        svc.mark_clean();
        let changes2 = svc.refresh_days_remaining(Local::now());
        // Second refresh with same time should have no changes
        assert!(changes2.is_empty());
        assert!(!svc.is_dirty());
    }
}
