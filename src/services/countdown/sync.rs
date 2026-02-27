//! Event-synchronisation helpers for countdown cards.
//!
//! When an event's title, comment, colour, or date changes in the main
//! calendar, these methods propagate the update to every countdown card
//! that references the same event.

use chrono::{DateTime, Local};

use super::models::{CountdownCardId, RgbaColor};
use super::palette::apply_event_palette_if_needed;
use super::service::CountdownService;

impl CountdownService {
    /// Synchronize stored card titles with the latest event label.
    /// Only updates cards that are still using the automatic event title (no override).
    pub fn sync_title_for_event(&mut self, event_id: i64, title: impl Into<String>) {
        let title = title.into();
        let mut changed = false;
        for card in self
            .cards
            .iter_mut()
            .filter(|card| card.event_id == Some(event_id) && card.title_override.is_none())
        {
            if card.event_title != title {
                card.event_title = title.clone();
                changed = true;
            }
        }

        if changed {
            self.dirty = true;
        }
    }

    pub fn sync_comment_for_event(&mut self, event_id: i64, comment: Option<String>) {
        let mut changed = false;
        for card in self
            .cards
            .iter_mut()
            .filter(|card| card.event_id == Some(event_id))
        {
            let needs_update = match (&card.comment, &comment) {
                (Some(existing), Some(target)) => existing != target,
                (None, None) => false,
                _ => true,
            };
            if needs_update {
                card.comment = comment.clone();
                changed = true;
            }
        }

        if changed {
            self.dirty = true;
        }
    }

    #[allow(dead_code)]
    pub fn sync_title_override_for_event(&mut self, event_id: i64, label: Option<String>) {
        let mut changed = false;
        for card in self
            .cards
            .iter_mut()
            .filter(|card| card.event_id == Some(event_id))
        {
            let managed = card.auto_title_override || card.title_override.is_none();
            if !managed {
                continue;
            }

            if card.title_override != label {
                card.title_override = label.clone();
                card.auto_title_override = label.is_some();
                changed = true;
            }
        }

        if changed {
            self.dirty = true;
        }
    }

    /// Synchronize the event color for all countdown cards linked to an event.
    /// This updates the stored event_color which is used when "Use default color" is enabled.
    pub fn sync_event_color_for_event(&mut self, event_id: i64, event_color: Option<RgbaColor>) {
        let mut changed = false;
        for card in self
            .cards
            .iter_mut()
            .filter(|card| card.event_id == Some(event_id))
        {
            if card.event_color != event_color {
                card.event_color = event_color;
                // Re-apply the palette if the card is using default colors
                apply_event_palette_if_needed(card);
                changed = true;
            }
        }

        if changed {
            self.dirty = true;
        }
    }

    /// Synchronize the start time for all countdown cards linked to an event.
    /// This updates the countdown target date when the event date changes.
    pub fn sync_start_at_for_event(&mut self, event_id: i64, start_at: DateTime<Local>) {
        let mut changed = false;
        let now = Local::now();
        for card in self
            .cards
            .iter_mut()
            .filter(|card| card.event_id == Some(event_id))
        {
            if card.start_at != start_at {
                card.start_at = start_at;
                let days = card.compute_days_remaining(now);
                card.record_days_remaining(days);
                changed = true;
            }
        }

        if changed {
            self.dirty = true;
            // Re-sort cards by date when an event's date changes
            self.sort_cards_by_date();
        }
    }

    pub fn set_start_at(&mut self, id: CountdownCardId, start_at: DateTime<Local>) -> bool {
        if let Some(card) = self.cards.iter_mut().find(|card| card.id == id) {
            card.start_at = start_at;
            let days = card.compute_days_remaining(Local::now());
            card.record_days_remaining(days);
            self.dirty = true;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Local};

    use super::super::models::RgbaColor;
    use super::super::service::CountdownService;

    #[test]
    fn sync_title_updates_matching_cards() {
        let mut svc = CountdownService::new();
        let t = Local::now() + Duration::days(5);
        svc.create_card(Some(10), "Old Name", t, None, None, None, None, 120.0, 110.0);
        svc.mark_clean();

        svc.sync_title_for_event(10, "New Name");
        assert_eq!(svc.cards()[0].event_title, "New Name");
        assert!(svc.is_dirty());
    }

    #[test]
    fn sync_title_skips_overridden_cards() {
        let mut svc = CountdownService::new();
        let t = Local::now() + Duration::days(5);
        let id = svc.create_card(Some(10), "Auto", t, None, None, None, None, 120.0, 110.0);
        svc.set_title_override(id, Some("Custom".into()));
        svc.mark_clean();

        svc.sync_title_for_event(10, "Updated");
        // Title should NOT change because it has a manual override
        assert_eq!(svc.cards()[0].event_title, "Auto");
        assert!(!svc.is_dirty());
    }

    #[test]
    fn sync_event_color_reapplies_palette() {
        let mut svc = CountdownService::new();
        let t = Local::now() + Duration::days(5);
        let accent = RgbaColor::new(100, 50, 200, 255);
        svc.create_card(Some(10), "Evt", t, None, None, Some(accent), None, 120.0, 110.0);
        svc.mark_clean();

        let new_color = RgbaColor::new(200, 100, 50, 255);
        svc.sync_event_color_for_event(10, Some(new_color));
        assert_eq!(svc.cards()[0].event_color, Some(new_color));
        assert!(svc.is_dirty());
    }

    #[test]
    fn sync_start_at_recomputes_days() {
        let mut svc = CountdownService::new();
        let t = Local::now() + Duration::days(5);
        svc.create_card(Some(10), "Evt", t, None, None, None, None, 120.0, 110.0);
        svc.mark_clean();

        let new_start = Local::now() + Duration::days(20);
        svc.sync_start_at_for_event(10, new_start);
        assert_eq!(svc.cards()[0].start_at, new_start);
        assert!(svc.is_dirty());
    }
}
