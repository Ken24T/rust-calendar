use std::time::Instant;

use chrono::{DateTime, Local};

use super::models::{
    CountdownAutoDismissConfig, CountdownCardGeometry, CountdownCardId, CountdownCardState,
    CountdownCardVisuals, CountdownCategory, CountdownCategoryId, CountdownDisplayMode,
    CountdownNotificationConfig, CountdownPersistedState, RgbaColor, DEFAULT_CATEGORY_ID,
    MAX_DAYS_FONT_SIZE, MIN_DAYS_FONT_SIZE,
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
    pub(super) categories: Vec<CountdownCategory>,
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
            categories: snapshot.categories,
        }
    }

    pub fn cards(&self) -> &[CountdownCardState] {
        &self.cards
    }

    pub fn visual_defaults(&self) -> &CountdownCardVisuals {
        &self.visual_defaults
    }

    /// Resolve effective visual defaults for a specific category.
    ///
    /// If the category has `use_global_defaults = true`, the global visual
    /// defaults are returned. Otherwise, the category's own `visual_defaults`
    /// are returned. Falls back to global defaults when the category is not
    /// found.
    pub fn effective_visual_defaults_for(
        &self,
        category_id: CountdownCategoryId,
    ) -> CountdownCardVisuals {
        if let Some(cat) = self.categories.iter().find(|c| c.id == category_id) {
            if cat.use_global_defaults {
                self.visual_defaults.clone()
            } else {
                cat.visual_defaults.clone()
            }
        } else {
            self.visual_defaults.clone()
        }
    }

    /// Build a map of effective visual defaults for every category.
    ///
    /// This is useful when rendering multiple categories simultaneously (e.g.
    /// in container mode where cards from different categories share one
    /// window).
    pub fn effective_visual_defaults_map(
        &self,
    ) -> std::collections::HashMap<CountdownCategoryId, CountdownCardVisuals> {
        self.categories
            .iter()
            .map(|c| (c.id, self.effective_visual_defaults_for(c.id)))
            .collect()
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
        self.create_card_in_category(
            event_id,
            event_title,
            start_at,
            event_start,
            event_end,
            event_color,
            event_body,
            default_width,
            default_height,
            CountdownCategoryId(DEFAULT_CATEGORY_ID),
        )
    }

    /// Create a new countdown card assigned to a specific category.
    #[allow(clippy::too_many_arguments)]
    pub fn create_card_in_category(
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
        category_id: CountdownCategoryId,
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
            category_id,
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

    // ========== Category Management ==========

    /// Get all categories.
    #[allow(dead_code)]
    pub fn categories(&self) -> &[CountdownCategory] {
        &self.categories
    }

    /// Get a mutable reference to a category by ID.
    #[allow(dead_code)]
    pub fn category_mut(&mut self, id: CountdownCategoryId) -> Option<&mut CountdownCategory> {
        self.categories.iter_mut().find(|c| c.id == id)
    }

    /// Add a new category. Returns the category (with a temporary ID).
    /// The real ID is assigned by the database on save.
    #[allow(dead_code)]
    pub fn add_category(&mut self, mut category: CountdownCategory) -> &CountdownCategory {
        // Assign a temporary ID (negative to distinguish from DB-assigned)
        // The real ID will be set when saved to the database
        let max_id = self
            .categories
            .iter()
            .map(|c| c.id.0)
            .max()
            .unwrap_or(0);
        category.id = CountdownCategoryId(max_id + 1);
        self.categories.push(category);
        self.dirty = true;
        self.categories.last().unwrap()
    }

    /// Remove a category by ID. Cards in this category are reassigned to
    /// the default "General" category.
    #[allow(dead_code)]
    pub fn remove_category(&mut self, id: CountdownCategoryId) -> bool {
        if id.0 == DEFAULT_CATEGORY_ID {
            log::warn!("Cannot remove the default 'General' category");
            return false;
        }

        let existed = self.categories.iter().any(|c| c.id == id);
        if !existed {
            return false;
        }

        // Reassign cards to the default category
        let default_id = CountdownCategoryId(DEFAULT_CATEGORY_ID);
        for card in &mut self.cards {
            if card.category_id == id {
                card.category_id = default_id;
            }
        }

        self.categories.retain(|c| c.id != id);
        self.dirty = true;
        true
    }

    /// Change a card's category.
    #[allow(dead_code)]
    pub fn set_card_category(
        &mut self,
        card_id: CountdownCardId,
        category_id: CountdownCategoryId,
    ) -> bool {
        if let Some(card) = self.cards.iter_mut().find(|c| c.id == card_id) {
            card.category_id = category_id;
            self.dirty = true;
            return true;
        }
        false
    }

    /// Get cards belonging to a specific category.
    #[allow(dead_code)]
    pub fn cards_in_category(&self, category_id: CountdownCategoryId) -> Vec<&CountdownCardState> {
        self.cards
            .iter()
            .filter(|c| c.category_id == category_id)
            .collect()
    }

    /// Update the container geometry for a specific category.
    #[allow(dead_code)]
    pub fn update_category_container_geometry(
        &mut self,
        category_id: CountdownCategoryId,
        geometry: CountdownCardGeometry,
    ) {
        if let Some(cat) = self.categories.iter_mut().find(|c| c.id == category_id) {
            if cat.container_geometry != Some(geometry) {
                log::debug!(
                    "Category {:?} container geometry updated: {:?} -> {:?}",
                    category_id,
                    cat.container_geometry,
                    geometry
                );
                cat.container_geometry = Some(geometry);
                self.dirty = true;
            }
        }
    }

    /// Set the categories list directly (used when loading from database).
    #[allow(dead_code)]
    pub(super) fn set_categories(&mut self, categories: Vec<CountdownCategory>) {
        self.categories = categories;
    }

    /// Toggle the collapsed state of a category container.
    pub fn toggle_category_collapsed(&mut self, category_id: CountdownCategoryId) {
        if let Some(cat) = self.categories.iter_mut().find(|c| c.id == category_id) {
            cat.is_collapsed = !cat.is_collapsed;
            log::debug!(
                "Category {:?} collapsed toggled to {}",
                category_id,
                cat.is_collapsed
            );
            self.dirty = true;
        }
    }

    /// Set the sort mode for a category's container.
    pub fn set_category_sort_mode(
        &mut self,
        category_id: CountdownCategoryId,
        mode: super::models::ContainerSortMode,
    ) {
        if let Some(cat) = self.categories.iter_mut().find(|c| c.id == category_id) {
            if cat.sort_mode != mode {
                log::debug!(
                    "Category {:?} sort mode changed: {:?} -> {:?}",
                    category_id,
                    cat.sort_mode,
                    mode
                );
                cat.sort_mode = mode;
                // When switching to Date mode, re-sort now
                if mode == super::models::ContainerSortMode::Date {
                    self.sort_cards_by_date();
                }
                self.dirty = true;
            }
        }
    }

    /// Check whether a category is collapsed.
    #[allow(dead_code)]
    pub fn is_category_collapsed(&self, category_id: CountdownCategoryId) -> bool {
        self.categories
            .iter()
            .find(|c| c.id == category_id)
            .map(|c| c.is_collapsed)
            .unwrap_or(false)
    }

    /// Get the sort mode for a category.
    #[allow(dead_code)]
    pub fn category_sort_mode(
        &self,
        category_id: CountdownCategoryId,
    ) -> super::models::ContainerSortMode {
        self.categories
            .iter()
            .find(|c| c.id == category_id)
            .map(|c| c.sort_mode)
            .unwrap_or_default()
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
    fn effective_visual_defaults_uses_global_when_category_flag_set() {
        let mut service = CountdownService::new();
        // Default category (General) has use_global_defaults = true
        let defaults = service.effective_visual_defaults_for(CountdownCategoryId(DEFAULT_CATEGORY_ID));
        assert_eq!(defaults.title_bg_color, service.visual_defaults().title_bg_color);
    }

    #[test]
    fn effective_visual_defaults_uses_category_when_flag_unset() {
        let mut service = CountdownService::new();
        let custom_color = RgbaColor::new(255, 0, 0, 255);
        let mut cat = CountdownCategory {
            name: "Custom".to_string(),
            use_global_defaults: false,
            ..CountdownCategory::default()
        };
        cat.visual_defaults.title_bg_color = custom_color;
        let added = service.add_category(cat);
        let cat_id = added.id;

        let defaults = service.effective_visual_defaults_for(cat_id);
        assert_eq!(defaults.title_bg_color, custom_color);
        assert_ne!(defaults.title_bg_color, service.visual_defaults().title_bg_color);
    }

    #[test]
    fn effective_visual_defaults_map_covers_all_categories() {
        let mut service = CountdownService::new();
        let cat = CountdownCategory {
            name: "Extra".to_string(),
            ..CountdownCategory::default()
        };
        service.add_category(cat);

        let map = service.effective_visual_defaults_map();
        assert_eq!(map.len(), service.categories().len());
        for cat in service.categories() {
            assert!(map.contains_key(&cat.id));
        }
    }

}
