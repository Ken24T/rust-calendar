use std::time::{Duration, Instant};

use super::models::{CountdownCardGeometry, CountdownCardId, CountdownDisplayMode};
use super::service::CountdownService;

impl CountdownService {
    pub fn queue_geometry_update(
        &mut self,
        id: CountdownCardId,
        geometry: CountdownCardGeometry,
    ) -> bool {
        if let Some(entry) = self
            .pending_geometry
            .iter_mut()
            .find(|(pending_id, _)| *pending_id == id)
        {
            if entry.1 == geometry {
                return false;
            }
            entry.1 = geometry;
            self.last_geometry_update = Some(Instant::now());
            return true;
        }

        self.pending_geometry.push((id, geometry));
        self.last_geometry_update = Some(Instant::now());
        true
    }

    pub fn flush_geometry_updates(&mut self) {
        // Only flush if enough time has passed since last update (debounce)
        const DEBOUNCE_MS: u64 = 180;
        if self.pending_geometry.is_empty() {
            return;
        }
        if let Some(last) = self.last_geometry_update {
            if last.elapsed() < Duration::from_millis(DEBOUNCE_MS) {
                return;
            }
        }
        let updates = std::mem::take(&mut self.pending_geometry);
        for (id, geometry) in updates {
            self.update_geometry(id, geometry);
        }
        self.last_geometry_update = None;
    }

    pub fn update_geometry(
        &mut self,
        id: CountdownCardId,
        geometry: CountdownCardGeometry,
    ) -> bool {
        if let Some(card) = self.cards.iter_mut().find(|card| card.id == id) {
            card.geometry = geometry;
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn app_window_geometry(&self) -> Option<CountdownCardGeometry> {
        self.app_window_geometry
    }

    pub fn update_app_window_geometry(&mut self, geometry: CountdownCardGeometry) {
        if self.app_window_geometry != Some(geometry) {
            self.app_window_geometry = Some(geometry);
            self.dirty = true;
        }
    }

    // Display mode and container methods
    pub fn display_mode(&self) -> CountdownDisplayMode {
        self.display_mode
    }

    pub fn set_display_mode(&mut self, mode: CountdownDisplayMode) {
        if self.display_mode != mode {
            self.display_mode = mode;
            self.dirty = true;
        }
    }

    pub fn container_geometry(&self) -> Option<CountdownCardGeometry> {
        self.container_geometry
    }

    pub fn update_container_geometry(&mut self, geometry: CountdownCardGeometry) {
        if self.container_geometry != Some(geometry) {
            log::debug!(
                "Container geometry updated: {:?} -> {:?}",
                self.container_geometry, geometry
            );
            self.container_geometry = Some(geometry);
            self.dirty = true;
        }
    }

    /// Reset all card and container positions to safe defaults on the primary monitor.
    /// This is useful when cards get "lost" on disconnected monitors or in invalid positions.
    pub fn reset_all_positions(&mut self) {
        const DEFAULT_X: f32 = 100.0;
        const DEFAULT_Y: f32 = 100.0;
        const DEFAULT_WIDTH: f32 = 300.0;
        const DEFAULT_HEIGHT: f32 = 200.0;
        const CARD_WIDTH: f32 = 120.0;
        const CARD_HEIGHT: f32 = 110.0;
        const CARD_SPACING: f32 = 20.0;

        log::info!("Resetting all countdown card and container positions to defaults");

        // Reset container geometry to a safe default position
        self.container_geometry = Some(CountdownCardGeometry {
            x: DEFAULT_X,
            y: DEFAULT_Y,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
        });

        // Reset each card's geometry to stacked positions
        for (index, card) in self.cards.iter_mut().enumerate() {
            let offset = index as f32 * CARD_SPACING;
            card.geometry = CountdownCardGeometry {
                x: DEFAULT_X + offset,
                y: DEFAULT_Y + offset,
                width: CARD_WIDTH,
                height: CARD_HEIGHT,
            };
            log::debug!("Reset card {:?} position to ({}, {})", card.id, card.geometry.x, card.geometry.y);
        }

        // Clear pending geometry updates since we're resetting
        self.pending_geometry.clear();
        self.dirty = true;
    }

    pub fn card_order(&self) -> &[CountdownCardId] {
        &self.card_order
    }

    pub fn reorder_cards(&mut self, new_order: Vec<CountdownCardId>) {
        if self.card_order != new_order {
            self.card_order = new_order;
            self.dirty = true;
        }
    }

    /// Sort cards by their target date (start_at) and update card_order
    pub fn sort_cards_by_date(&mut self) {
        // Get cards sorted by start_at
        let mut sorted_ids: Vec<(CountdownCardId, chrono::DateTime<chrono::Local>)> = self
            .cards
            .iter()
            .map(|c| (c.id, c.start_at))
            .collect();
        sorted_ids.sort_by_key(|(_, start_at)| *start_at);

        let new_order: Vec<CountdownCardId> = sorted_ids.into_iter().map(|(id, _)| id).collect();
        self.reorder_cards(new_order);
    }

    /// Sanitize all card and container geometries to ensure they're visible on available monitors.
    /// Call this on startup after loading from database.
    ///
    /// The monitors parameter is a list of (x, y, width, height) tuples representing
    /// available monitor bounds. If empty, a default 1920x1080 monitor at (0,0) is assumed.
    pub fn sanitize_all_geometries(&mut self, monitors: &[(f32, f32, f32, f32)]) {
        let default_offset = 50.0;
        let mut any_changed = false;

        // Sanitize individual card geometries
        for (card_index, card) in self.cards.iter_mut().enumerate() {
            // Stagger default positions for multiple cards
            let default_x = 100.0 + (card_index as f32 * default_offset);
            let default_y = 100.0 + (card_index as f32 * default_offset);
            let default_pos = (default_x, default_y);

            let sanitized = card.geometry.sanitize_for_monitors(monitors, default_pos);
            if sanitized != card.geometry {
                log::info!(
                    "Sanitized geometry for card {:?} '{}' from {:?} to {:?}",
                    card.id,
                    card.effective_title(),
                    card.geometry,
                    sanitized
                );
                card.geometry = sanitized;
                any_changed = true;
            }
        }

        // Sanitize container geometry
        if let Some(container_geom) = self.container_geometry {
            let sanitized = container_geom.sanitize_for_monitors(monitors, (100.0, 100.0));
            if sanitized != container_geom {
                log::info!(
                    "Sanitized container geometry from {:?} to {:?}",
                    container_geom,
                    sanitized
                );
                self.container_geometry = Some(sanitized);
                any_changed = true;
            }
        }

        // Sanitize app window geometry
        if let Some(app_geom) = self.app_window_geometry {
            let sanitized = app_geom.sanitize_for_monitors(monitors, (100.0, 100.0));
            if sanitized != app_geom {
                log::info!(
                    "Sanitized app window geometry from {:?} to {:?}",
                    app_geom,
                    sanitized
                );
                self.app_window_geometry = Some(sanitized);
                any_changed = true;
            }
        }

        if any_changed {
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_mode_defaults_to_individual_windows() {
        let service = CountdownService::new();
        assert_eq!(service.display_mode(), CountdownDisplayMode::IndividualWindows);
    }

    #[test]
    fn test_set_display_mode_updates_and_marks_dirty() {
        let mut service = CountdownService::new();
        service.mark_clean();
        assert!(!service.is_dirty());

        service.set_display_mode(CountdownDisplayMode::Container);
        assert_eq!(service.display_mode(), CountdownDisplayMode::Container);
        assert!(service.is_dirty());
    }

    #[test]
    fn test_set_same_display_mode_does_not_mark_dirty() {
        let mut service = CountdownService::new();
        service.mark_clean();
        assert!(!service.is_dirty());

        service.set_display_mode(CountdownDisplayMode::IndividualWindows);
        assert!(!service.is_dirty());
    }

    #[test]
    fn test_container_geometry_defaults_to_none() {
        let service = CountdownService::new();
        assert_eq!(service.container_geometry(), None);
    }

    #[test]
    fn test_update_container_geometry() {
        let mut service = CountdownService::new();
        let geom = CountdownCardGeometry {
            x: 100.0,
            y: 200.0,
            width: 800.0,
            height: 600.0,
        };

        service.mark_clean();
        service.update_container_geometry(geom);

        assert_eq!(service.container_geometry(), Some(geom));
        assert!(service.is_dirty());
    }

    #[test]
    fn test_card_order_defaults_to_empty() {
        let service = CountdownService::new();
        assert!(service.card_order().is_empty());
    }

    #[test]
    fn test_reorder_cards() {
        let mut service = CountdownService::new();
        let new_order = vec![
            CountdownCardId(3),
            CountdownCardId(1),
            CountdownCardId(2),
        ];

        service.mark_clean();
        service.reorder_cards(new_order.clone());

        assert_eq!(service.card_order(), &new_order);
        assert!(service.is_dirty());
    }

    #[test]
    fn test_reorder_same_order_does_not_mark_dirty() {
        let mut service = CountdownService::new();
        let order = vec![CountdownCardId(1)];

        service.reorder_cards(order.clone());
        service.mark_clean();

        service.reorder_cards(order);
        assert!(!service.is_dirty());
    }
}
