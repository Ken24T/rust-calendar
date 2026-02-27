use std::{
    path::Path,
    time::{Duration, Instant},
};

use anyhow::Result;
use chrono::{DateTime, Local};
use rusqlite::Connection;

use super::models::{
    default_body_bg_color, default_days_fg_color, default_days_font_size, default_title_bg_color,
    default_title_fg_color, default_title_font_size, CountdownAutoDismissConfig,
    CountdownCardGeometry, CountdownCardId, CountdownCardState, CountdownCardVisuals,
    CountdownDisplayMode, CountdownNotificationConfig, CountdownPersistedState,
    CountdownWarningState, RgbaColor, MAX_DAYS_FONT_SIZE, MIN_DAYS_FONT_SIZE,
};
use super::palette::apply_event_palette_if_needed;
use super::persistence::{load_snapshot, save_snapshot};
use super::repository::{CountdownGlobalSettings, CountdownRepository};

/// Manages active countdown cards while the calendar app is running.
pub struct CountdownService {
    cards: Vec<CountdownCardState>,
    next_id: u64,
    dirty: bool,
    pending_geometry: Vec<(CountdownCardId, CountdownCardGeometry)>,
    last_geometry_update: Option<Instant>,
    visual_defaults: CountdownCardVisuals,
    app_window_geometry: Option<CountdownCardGeometry>,
    notification_config: CountdownNotificationConfig,
    auto_dismiss_defaults: CountdownAutoDismissConfig,
    display_mode: CountdownDisplayMode,
    container_geometry: Option<CountdownCardGeometry>,
    card_order: Vec<CountdownCardId>,
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

    /// Returns a snapshot of the current state for JSON serialization.
    /// Note: This is the legacy method. New code should use save_to_database.
    #[allow(dead_code)]
    pub fn snapshot(&self) -> CountdownPersistedState {
        CountdownPersistedState {
            next_id: self.next_id,
            cards: self.cards.clone(),
            visual_defaults: self.visual_defaults.clone(),
            app_window_geometry: self.app_window_geometry,
            notification_config: self.notification_config.clone(),
            auto_dismiss_defaults: self.auto_dismiss_defaults.clone(),
            display_mode: self.display_mode,
            container_geometry: self.container_geometry,
            card_order: self.card_order.clone(),
        }
    }

    #[allow(dead_code)]
    pub fn load_from_disk(path: &Path) -> Result<Self> {
        let snapshot = load_snapshot(path)?;
        Ok(Self::from_snapshot(snapshot))
    }

    /// Saves countdown state to a JSON file.
    /// Note: This is the legacy method. New code should use save_to_database.
    #[allow(dead_code)]
    pub fn save_to_disk(&self, path: &Path) -> Result<()> {
        let snapshot = self.snapshot();
        save_snapshot(path, &snapshot)
    }

    /// Load countdown cards and settings from the database.
    /// This is the preferred method for loading data.
    pub fn load_from_database(conn: &Connection) -> Result<Self> {
        let repo = CountdownRepository::new(conn);

        // Load global settings
        let settings = repo.get_global_settings()?;

        // Load all cards
        let mut cards = repo.get_all_cards()?;

        // Clamp font sizes
        for card in &mut cards {
            card.visuals.days_font_size = card
                .visuals
                .days_font_size
                .clamp(MIN_DAYS_FONT_SIZE, MAX_DAYS_FONT_SIZE);
        }

        let mut visual_defaults = settings.visual_defaults;
        visual_defaults.days_font_size = visual_defaults
            .days_font_size
            .clamp(MIN_DAYS_FONT_SIZE, MAX_DAYS_FONT_SIZE);

        log::info!(
            "Loaded countdown service from database: container_geometry={:?}, display_mode={:?}",
            settings.container_geometry, settings.display_mode
        );
        
        Ok(Self {
            cards,
            next_id: settings.next_card_id.max(1),
            dirty: false,
            pending_geometry: Vec::new(),
            last_geometry_update: None,
            visual_defaults,
            app_window_geometry: settings.app_window_geometry,
            notification_config: settings.notification_config,
            auto_dismiss_defaults: settings.auto_dismiss_defaults,
            display_mode: settings.display_mode,
            container_geometry: settings.container_geometry,
            card_order: settings.card_order,
        })
    }

    /// Save all countdown cards and settings to the database.
    /// This method syncs the in-memory state with the database.
    pub fn save_to_database(&mut self, conn: &Connection) -> Result<()> {
        let repo = CountdownRepository::new(conn);

        // Update global settings
        let settings = CountdownGlobalSettings {
            next_card_id: self.next_id,
            app_window_geometry: self.app_window_geometry,
            visual_defaults: self.visual_defaults.clone(),
            notification_config: self.notification_config.clone(),
            auto_dismiss_defaults: self.auto_dismiss_defaults.clone(),
            display_mode: self.display_mode,
            container_geometry: self.container_geometry,
            card_order: self.card_order.clone(),
        };
        repo.update_global_settings(&settings)?;

        // Get existing card IDs from database
        let existing_cards = repo.get_all_cards()?;
        let existing_ids: std::collections::HashSet<u64> =
            existing_cards.iter().map(|c| c.id.0).collect();

        // Get current card IDs
        let current_ids: std::collections::HashSet<u64> =
            self.cards.iter().map(|c| c.id.0).collect();

        // Delete cards that no longer exist in memory
        for id in existing_ids.difference(&current_ids) {
            repo.delete_card(CountdownCardId(*id))?;
        }

        // Insert or update current cards
        let mut failed_inserts = Vec::new();
        for card in &self.cards {
            if existing_ids.contains(&card.id.0) {
                repo.update_card(card)?;
            } else {
                // Try to insert - may fail if event was deleted (FK constraint)
                if let Err(e) = repo.insert_card(card) {
                    log::warn!(
                        "Failed to insert card {:?} (event_id={:?}): {}. Card will be removed.",
                        card.id,
                        card.event_id,
                        e
                    );
                    failed_inserts.push(card.id);
                }
            }
        }

        // Remove cards that failed to insert (likely due to deleted events)
        for id in failed_inserts {
            self.cards.retain(|c| c.id != id);
        }

        self.dirty = false;
        Ok(())
    }

    /// Save a single card to the database (for incremental updates).
    #[allow(dead_code)]
    pub fn save_card_to_database(&self, conn: &Connection, id: CountdownCardId) -> Result<bool> {
        if let Some(card) = self.cards.iter().find(|c| c.id == id) {
            let repo = CountdownRepository::new(conn);
            // Try update first, if it fails (row doesn't exist), insert
            if !repo.update_card(card)? {
                repo.insert_card(card)?;
            }
            return Ok(true);
        }
        Ok(false)
    }

    /// Delete a card from the database.
    #[allow(dead_code)]
    pub fn delete_card_from_database(&self, conn: &Connection, id: CountdownCardId) -> Result<bool> {
        let repo = CountdownRepository::new(conn);
        repo.delete_card(id)
    }

    /// Migrate data from JSON file to database.
    /// Call this once during app startup if JSON file exists.
    pub fn migrate_json_to_database(json_path: &Path, conn: &Connection) -> Result<bool> {
        if !json_path.exists() {
            log::info!("No JSON countdown file to migrate");
            return Ok(false);
        }

        log::info!("Migrating countdown cards from JSON to database...");

        // Load from JSON
        let snapshot = load_snapshot(json_path)?;

        let repo = CountdownRepository::new(conn);

        // Update global settings
        let settings = CountdownGlobalSettings {
            next_card_id: snapshot.next_id.max(1),
            app_window_geometry: snapshot.app_window_geometry,
            visual_defaults: snapshot.visual_defaults,
            notification_config: snapshot.notification_config,
            auto_dismiss_defaults: snapshot.auto_dismiss_defaults,
            display_mode: snapshot.display_mode,
            container_geometry: snapshot.container_geometry,
            card_order: snapshot.card_order,
        };
        repo.update_global_settings(&settings)?;

        // Insert all cards, skipping any that fail (e.g., due to deleted events)
        let mut migrated_count = 0;
        let mut skipped_count = 0;
        for card in &snapshot.cards {
            match repo.insert_card(card) {
                Ok(_) => migrated_count += 1,
                Err(e) => {
                    log::warn!(
                        "Skipping card {:?} during migration (event may have been deleted): {}",
                        card.id,
                        e
                    );
                    skipped_count += 1;
                }
            }
        }

        log::info!(
            "Migrated {} countdown cards from JSON to database ({} skipped)",
            migrated_count,
            skipped_count
        );

        // Rename the JSON file to indicate migration completed
        let backup_path = json_path.with_extension("json.migrated");
        if let Err(e) = std::fs::rename(json_path, &backup_path) {
            log::warn!(
                "Failed to rename migrated JSON file: {}. Please delete {} manually.",
                e,
                json_path.display()
            );
        } else {
            log::info!("Renamed migrated JSON file to {}", backup_path.display());
        }

        Ok(true)
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

    pub fn set_always_on_top(&mut self, id: CountdownCardId, always_on_top: bool) -> bool {
        self.update_visual_flag(id, |visuals| visuals.always_on_top = always_on_top)
    }

    pub fn set_title_bg_color(&mut self, id: CountdownCardId, color: RgbaColor) -> bool {
        self.update_visual_flag(id, |visuals| {
            visuals.title_bg_color = color;
            visuals.use_default_title_bg = false;
        })
    }

    pub fn set_title_fg_color(&mut self, id: CountdownCardId, color: RgbaColor) -> bool {
        self.update_visual_flag(id, |visuals| {
            visuals.title_fg_color = color;
            visuals.use_default_title_fg = false;
        })
    }

    pub fn set_body_bg_color(&mut self, id: CountdownCardId, color: RgbaColor) -> bool {
        self.update_visual_flag(id, |visuals| {
            visuals.body_bg_color = color;
            visuals.use_default_body_bg = false;
        })
    }

    pub fn set_days_fg_color(&mut self, id: CountdownCardId, color: RgbaColor) -> bool {
        self.update_visual_flag(id, |visuals| {
            visuals.days_fg_color = color;
            visuals.use_default_days_fg = false;
        })
    }

    pub fn set_use_default_title_bg(&mut self, id: CountdownCardId, use_default: bool) -> bool {
        let fallback = self.visual_defaults.title_bg_color;
        let updated = self.update_card_state(id, |card| {
            card.visuals.use_default_title_bg = use_default;
            if use_default {
                card.visuals.title_bg_color = fallback;
            } else {
                apply_event_palette_if_needed(card);
            }
        });
        if updated {
            self.visual_defaults.use_default_title_bg = use_default;
        }
        updated
    }

    pub fn set_use_default_title_fg(&mut self, id: CountdownCardId, use_default: bool) -> bool {
        let fallback = self.visual_defaults.title_fg_color;
        let updated = self.update_card_state(id, |card| {
            card.visuals.use_default_title_fg = use_default;
            if use_default {
                card.visuals.title_fg_color = fallback;
            } else {
                apply_event_palette_if_needed(card);
            }
        });
        if updated {
            self.visual_defaults.use_default_title_fg = use_default;
        }
        updated
    }

    pub fn set_use_default_body_bg(&mut self, id: CountdownCardId, use_default: bool) -> bool {
        let fallback = self.visual_defaults.body_bg_color;
        let updated = self.update_card_state(id, |card| {
            card.visuals.use_default_body_bg = use_default;
            if use_default {
                card.visuals.body_bg_color = fallback;
            } else {
                apply_event_palette_if_needed(card);
            }
        });
        if updated {
            self.visual_defaults.use_default_body_bg = use_default;
        }
        updated
    }

    pub fn set_use_default_days_fg(&mut self, id: CountdownCardId, use_default: bool) -> bool {
        let fallback = self.visual_defaults.days_fg_color;
        let updated = self.update_card_state(id, |card| {
            card.visuals.use_default_days_fg = use_default;
            if use_default {
                card.visuals.days_fg_color = fallback;
            } else {
                apply_event_palette_if_needed(card);
            }
        });
        if updated {
            self.visual_defaults.use_default_days_fg = use_default;
        }
        updated
    }

    pub fn set_days_font_size(&mut self, id: CountdownCardId, size: f32) -> bool {
        self.update_visual_flag(id, |visuals| {
            visuals.days_font_size = size.clamp(MIN_DAYS_FONT_SIZE, MAX_DAYS_FONT_SIZE)
        })
    }

    pub fn set_title_font_size(&mut self, id: CountdownCardId, size: f32) -> bool {
        self.update_visual_flag(id, |visuals| {
            visuals.title_font_size = size.clamp(10.0, 64.0)
        })
    }

    pub fn set_comment(&mut self, id: CountdownCardId, comment: Option<String>) -> bool {
        if let Some(card) = self.cards.iter_mut().find(|card| card.id == id) {
            card.comment = comment;
            self.dirty = true;
            return true;
        }
        false
    }

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

    pub fn apply_visual_defaults(&mut self, id: CountdownCardId) -> bool {
        if let Some(card) = self.cards.iter_mut().find(|card| card.id == id) {
            card.visuals = self.visual_defaults.clone();
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn defaults(&self) -> &CountdownCardVisuals {
        &self.visual_defaults
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

    pub fn set_default_title_bg_color(&mut self, color: RgbaColor) {
        self.visual_defaults.title_bg_color = color;
        self.dirty = true;
    }

    pub fn reset_default_title_bg_color(&mut self) {
        self.set_default_title_bg_color(default_title_bg_color());
    }

    pub fn set_default_title_fg_color(&mut self, color: RgbaColor) {
        self.visual_defaults.title_fg_color = color;
        self.dirty = true;
    }

    pub fn reset_default_title_fg_color(&mut self) {
        self.set_default_title_fg_color(default_title_fg_color());
    }

    pub fn set_default_body_bg_color(&mut self, color: RgbaColor) {
        self.visual_defaults.body_bg_color = color;
        self.dirty = true;
    }

    pub fn reset_default_body_bg_color(&mut self) {
        self.set_default_body_bg_color(default_body_bg_color());
    }

    pub fn set_default_days_fg_color(&mut self, color: RgbaColor) {
        self.visual_defaults.days_fg_color = color;
        self.dirty = true;
    }

    pub fn reset_default_days_fg_color(&mut self) {
        self.set_default_days_fg_color(default_days_fg_color());
    }

    pub fn set_default_days_font_size(&mut self, size: f32) {
        self.visual_defaults.days_font_size = size.clamp(MIN_DAYS_FONT_SIZE, MAX_DAYS_FONT_SIZE);
        self.dirty = true;
    }

    pub fn reset_default_days_font_size(&mut self) {
        self.set_default_days_font_size(default_days_font_size());
    }

    pub fn set_default_title_font_size(&mut self, size: f32) {
        self.visual_defaults.title_font_size = size.clamp(10.0, 64.0);
        self.dirty = true;
    }

    pub fn reset_default_title_font_size(&mut self) {
        self.set_default_title_font_size(default_title_font_size());
    }

    fn update_visual_flag<F>(&mut self, id: CountdownCardId, mut update: F) -> bool
    where
        F: FnMut(&mut CountdownCardVisuals),
    {
        if let Some(card) = self.cards.iter_mut().find(|card| card.id == id) {
            update(&mut card.visuals);
            self.dirty = true;
            return true;
        }
        false
    }

    fn update_card_state<F>(&mut self, id: CountdownCardId, mut update: F) -> bool
    where
        F: FnMut(&mut CountdownCardState),
    {
        if let Some(card) = self.cards.iter_mut().find(|card| card.id == id) {
            update(card);
            self.dirty = true;
            return true;
        }
        false
    }

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

    /// Returns notification config for external use
    pub fn notification_config(&self) -> &CountdownNotificationConfig {
        &self.notification_config
    }

    /// Returns mutable notification config for settings updates
    #[allow(dead_code)]
    pub fn notification_config_mut(&mut self) -> &mut CountdownNotificationConfig {
        self.dirty = true;
        &mut self.notification_config
    }

    /// Returns auto-dismiss defaults for external use
    #[allow(dead_code)]
    pub fn auto_dismiss_defaults(&self) -> &CountdownAutoDismissConfig {
        &self.auto_dismiss_defaults
    }

    /// Returns mutable auto-dismiss defaults for settings updates
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
                    (None, _) => true,                              // Initial state and it's urgent
                    (Some(old), new) if new as u8 > old as u8 => true, // Urgency increased
                    _ => false, // Urgency decreased or stayed same
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
    use super::*;
    use chrono::Duration;
    use tempfile::tempdir;

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
    fn persist_and_reload_cards() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("countdowns.json");
        let mut service = CountdownService::new();
        let target_start = Local::now() + Duration::days(10);
        service.create_card(None, "Persist", target_start, None, None, None, None, 120.0, 110.0);
        service.save_to_disk(&file_path).unwrap();

        let loaded = CountdownService::load_from_disk(&file_path).unwrap();
        assert_eq!(loaded.cards().len(), 1);
        assert_eq!(loaded.cards()[0].event_title, "Persist");
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
