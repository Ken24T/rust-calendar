use std::time::{Duration, Instant};

use chrono::{DateTime, Local};

use super::models::{
    CountdownAutoDismissConfig, CountdownCardGeometry, CountdownCardId, CountdownCardState,
    CountdownCardVisuals, CountdownDisplayMode, CountdownNotificationConfig,
    CountdownPersistedState, RgbaColor, MAX_DAYS_FONT_SIZE, MIN_DAYS_FONT_SIZE,
};
use super::palette::apply_event_palette_if_needed;

/// Manages active countdown cards while the calendar app is running.
pub struct CountdownService {
    pub(super) cards: Vec<CountdownCardState>,
    pub(super) next_id: u64,
    pub(super) dirty: bool,
    pub(super) pending_geometry: Vec<(CountdownCardId, CountdownCardGeometry)>,
    pub(super) last_geometry_update: Option<Instant>,
    pub(super) visual_defaults: CountdownCardVisuals,
    pub(super) app_window_geometry: Option<CountdownCardGeometry>,
    pub(super) notification_config: CountdownNotificationConfig,
    pub(super) auto_dismiss_defaults: CountdownAutoDismissConfig,
    pub(super) display_mode: CountdownDisplayMode,
    pub(super) container_geometry: Option<CountdownCardGeometry>,
    pub(super) card_order: Vec<CountdownCardId>,
}

impl Default for CountdownService {
    fn default() -> Self {
        Self::new()
    }
}

impl CountdownService {
    pub fn new() -> Self {
        Self::from_snapshot(CountdownPersistedState::default())
    }

    pub fn from_snapshot(mut snapshot: CountdownPersistedState) -> Self {
        for card in &mut snapshot.cards {
            card.visuals.days_font_size = card
                .visuals
                .days_font_size
                .clamp(MIN_DAYS_FONT_SIZE, MAX_DAYS_FONT_SIZE);
            // Reset use_default_* flags to false (unchecked by default)
            card.visuals.use_default_title_bg = false;
            card.visuals.use_default_title_fg = false;
            card.visuals.use_default_body_bg = false;
            card.visuals.use_default_days_fg = false;
        }
        snapshot.visual_defaults.days_font_size = snapshot
            .visual_defaults
            .days_font_size
            .clamp(MIN_DAYS_FONT_SIZE, MAX_DAYS_FONT_SIZE);
        // Reset use_default_* flags to false for defaults too
        snapshot.visual_defaults.use_default_title_bg = false;
        snapshot.visual_defaults.use_default_title_fg = false;
        snapshot.visual_defaults.use_default_body_bg = false;
        snapshot.visual_defaults.use_default_days_fg = false;

        Self {
            cards: snapshot.cards,
            next_id: snapshot.next_id.max(1),
            dirty: false,
            pending_geometry: Vec::new(),
            last_geometry_update: None,
            visual_defaults: snapshot.visual_defaults,
            app_window_geometry: snapshot.app_window_geometry,
            notification_config: snapshot.notification_config,
            auto_dismiss_defaults: snapshot.auto_dismiss_defaults,
            display_mode: snapshot.display_mode,
            container_geometry: snapshot.container_geometry,
            card_order: snapshot.card_order,
        }
    }



    pub fn cards(&self) -> &[CountdownCardState] {
        &self.cards
    }

    pub fn visual_defaults(&self) -> &CountdownCardVisuals {
        &self.visual_defaults
    }

    /// Find a countdown card by its associated event ID
    pub fn find_card_by_event_id(&self, event_id: i64) -> Option<&CountdownCardState> {
        self.cards.iter().find(|card| card.event_id == Some(event_id))
    }

    /// Get a set of all event IDs that have associated countdown cards
    #[allow(dead_code)]
    pub fn event_ids_with_cards(&self) -> std::collections::HashSet<i64> {
        self.cards.iter().filter_map(|card| card.event_id).collect()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
        self.pending_geometry.clear();
        self.last_geometry_update = None;
    }

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

    #[allow(clippy::too_many_arguments)]
    pub fn create_card(
        &mut self,
        event_id: Option<i64>,
        event_title: impl Into<String>,
        start_at: DateTime<Local>,
        event_start: Option<DateTime<Local>>,
        event_end: Option<DateTime<Local>>,
        event_color: Option<RgbaColor>,
        event_body: Option<String>,
        default_width: f32,
        default_height: f32,
    ) -> CountdownCardId {
        const MIN_DIMENSION: f32 = 20.0;
        const MAX_DIMENSION: f32 = 600.0;
        const FALLBACK_WIDTH: f32 = 120.0;
        const FALLBACK_HEIGHT: f32 = 110.0;

        // Use provided dimensions if valid, otherwise use fallbacks
        let width = if default_width.is_finite() {
            default_width.clamp(MIN_DIMENSION, MAX_DIMENSION)
        } else {
            FALLBACK_WIDTH
        };
        let height = if default_height.is_finite() {
            default_height.clamp(MIN_DIMENSION, MAX_DIMENSION)
        } else {
            FALLBACK_HEIGHT
        };
        log::info!(
            "create_card received: default_width={}, default_height={}, using: width={}, height={}",
            default_width,
            default_height,
            width,
            height
        );

        let id = CountdownCardId(self.next_id);
        self.next_id += 1;
        let geometry = CountdownCardGeometry {
            x: 50.0,
            y: 50.0,
            width,
            height,
        };
        log::info!(
            "create_card: creating card with geometry: x={}, y={}, width={}, height={}",
            geometry.x,
            geometry.y,
            geometry.width,
            geometry.height
        );
        let mut card = CountdownCardState {
            id,
            event_id,
            event_title: event_title.into(),
            start_at,
            event_start,
            event_end,
            title_override: None,
            auto_title_override: false,
            geometry,
            visuals: self.visual_defaults.clone(),
            last_computed_days: None,
            comment: event_body,
            event_color,
            last_warning_state: None,
            last_notification_time: None,
            auto_dismiss: self.auto_dismiss_defaults.clone(),
        };
        apply_event_palette_if_needed(&mut card);
        self.cards.push(card);
        self.card_order.push(id);
        self.sort_cards_by_date();
        self.dirty = true;
        id
    }

    pub fn remove_card(&mut self, id: CountdownCardId) -> bool {
        if let Some(idx) = self.cards.iter().position(|card| card.id == id) {
            let card = self.cards.remove(idx);
            self.card_order.retain(|&card_id| card_id != id);
            log::info!(
                "remove_card: removed card {:?} for event {:?} ({})",
                id,
                card.event_id,
                card.event_title
            );
            self.dirty = true;
            return true;
        }
        log::warn!("remove_card: card {:?} not found", id);
        false
    }

    /// Remove all countdown cards associated with a given event ID.
    /// Call this when deleting an event to keep the in-memory state in sync.
    pub fn remove_cards_for_event(&mut self, event_id: i64) -> usize {
        let initial_count = self.cards.len();
        // Collect IDs to remove for card_order cleanup
        let ids_to_remove: Vec<CountdownCardId> = self.cards
            .iter()
            .filter(|card| card.event_id == Some(event_id))
            .map(|card| card.id)
            .collect();
        
        self.cards.retain(|card| {
            if card.event_id == Some(event_id) {
                log::info!(
                    "remove_cards_for_event: removing card {:?} for deleted event {}",
                    card.id,
                    event_id
                );
                false
            } else {
                true
            }
        });
        
        // Remove from card_order
        for id in &ids_to_remove {
            self.card_order.retain(|&card_id| card_id != *id);
        }
        
        let removed = initial_count - self.cards.len();
        if removed > 0 {
            self.dirty = true;
        }
        removed
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

    pub fn set_title_override(&mut self, id: CountdownCardId, title: Option<String>) -> bool {
        if let Some(card) = self.cards.iter_mut().find(|card| card.id == id) {
            card.title_override = title;
            card.auto_title_override = false;
            self.dirty = true;
            return true;
        }
        false
    }

    #[allow(dead_code)]
    pub fn set_auto_title_override(&mut self, id: CountdownCardId, title: Option<String>) -> bool {
        if let Some(card) = self.cards.iter_mut().find(|card| card.id == id) {
            card.title_override = title;
            card.auto_title_override = card.title_override.is_some();
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
    use chrono::Duration;

    #[test]
    fn create_refresh_and_remove_card() {
        let mut service = CountdownService::new();
        let target_start = Local::now() + Duration::days(34);
        let card_id = service.create_card(
            Some(42),
            "Sample Event",
            target_start,
            None,
            None,
            None,
            None,
            120.0,
            110.0,
        );

        assert_eq!(service.cards().len(), 1);
        assert_eq!(service.cards()[0].effective_title(), "Sample Event");

        let changes = service.refresh_days_remaining(Local::now());
        assert_eq!(changes.len(), 1);

        assert!(service.set_title_override(card_id, Some("Custom".into())));
        assert_eq!(service.cards()[0].effective_title(), "Custom");

        assert!(service.remove_card(card_id));
        assert!(service.cards().is_empty());
    }

    #[test]
    fn toggling_default_colors_uses_event_palette() {
        let mut service = CountdownService::new();
        let target_start = Local::now() + Duration::days(5);
        let accent = RgbaColor::new(180, 60, 100, 255);
        let card_id = service.create_card(
            Some(1),
            "Palette",
            target_start,
            None,
            None,
            Some(accent),
            None,
            120.0,
            110.0,
        );

        assert!(service.set_use_default_body_bg(card_id, false));
        assert!(service.set_use_default_days_fg(card_id, false));
        let card = service.cards().iter().find(|c| c.id == card_id).unwrap();
        assert!(!card.visuals.use_default_body_bg);
        assert!(!card.visuals.use_default_days_fg);
        assert_ne!(card.visuals.body_bg_color, service.defaults().body_bg_color);
        assert_ne!(card.visuals.days_fg_color, service.defaults().days_fg_color);

        assert!(service.set_use_default_body_bg(card_id, true));
        assert!(service.set_use_default_days_fg(card_id, true));
        let card = service.cards().iter().find(|c| c.id == card_id).unwrap();
        assert_eq!(card.visuals.body_bg_color, service.defaults().body_bg_color);
        assert_eq!(card.visuals.days_fg_color, service.defaults().days_fg_color);
    }

    #[test]
    fn toggling_default_state_is_per_card() {
        let mut service = CountdownService::new();
        let target_start = Local::now() + Duration::days(3);
        let accent = RgbaColor::new(10, 150, 200, 255);
        let card_a =
            service.create_card(Some(1), "A", target_start, None, None, Some(accent), None, 120.0, 110.0);
        let card_b =
            service.create_card(Some(2), "B", target_start, None, None, Some(accent), None, 120.0, 110.0);

        assert!(service.set_use_default_title_bg(card_a, false));
        assert!(service.set_use_default_title_bg(card_b, true));

        let card = service.cards().iter().find(|c| c.id == card_a).unwrap();
        assert!(!card.visuals.use_default_title_bg);

        let other = service.cards().iter().find(|c| c.id == card_b).unwrap();
        assert!(other.visuals.use_default_title_bg);
    }

    #[test]
    fn new_cards_inherit_default_checkbox_state() {
        let mut service = CountdownService::new();
        let base_time = Local::now() + Duration::days(2);
        let accent = RgbaColor::new(120, 40, 200, 255);
        let first = service.create_card(
            Some(1),
            "First",
            base_time,
            None,
            None,
            Some(accent),
            None,
            120.0,
            110.0,
        );
        assert!(service.set_use_default_body_bg(first, false));

        let second = service.create_card(
            Some(2),
            "Second",
            base_time + Duration::days(3),
            None,
            None,
            Some(accent),
            None,
            120.0,
            110.0,
        );
        let second_card = service.cards().iter().find(|c| c.id == second).unwrap();
        assert!(!second_card.visuals.use_default_body_bg);
        assert_ne!(
            second_card.visuals.body_bg_color,
            service.defaults().body_bg_color
        );
    }

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
