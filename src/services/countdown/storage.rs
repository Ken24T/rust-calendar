//! Persistence (disk and database) for countdown cards and settings.
//!
//! Handles JSON snapshot load/save (legacy), SQLite database CRUD,
//! and one-time JSON→database migration.

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use super::models::{
    CountdownCardId, CountdownPersistedState, MIN_DAYS_FONT_SIZE,
    MAX_DAYS_FONT_SIZE,
};
use super::persistence::{load_snapshot, save_snapshot};
use super::repository::{CountdownGlobalSettings, CountdownRepository};
use super::service::CountdownService;

impl CountdownService {
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
            categories: self.categories.clone(),
            templates: self.templates.clone(),
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

        // Load categories
        let categories = repo.get_all_categories()?;

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
            "Loaded countdown service from database: container_geometry={:?}, display_mode={:?}, categories={}",
            settings.container_geometry, settings.display_mode, categories.len()
        );

        // Load templates
        let templates = repo.get_all_templates()?;

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
            categories,
            templates,
        })
    }

    /// Save all countdown cards and settings to the database.
    /// This method syncs the in-memory state with the database.
    pub fn save_to_database(&mut self, conn: &Connection) -> Result<()> {
        let repo = CountdownRepository::new(conn);

        // Sync templates first (categories reference templates via FK)
        let existing_templates = repo.get_all_templates()?;
        let existing_tmpl_ids: std::collections::HashSet<i64> =
            existing_templates.iter().map(|t| t.id.0).collect();
        let current_tmpl_ids: std::collections::HashSet<i64> =
            self.templates.iter().map(|t| t.id.0).collect();

        // Delete removed templates (clears category references)
        for id in existing_tmpl_ids.difference(&current_tmpl_ids) {
            use super::models::CountdownCardTemplateId;
            if let Err(e) = repo.delete_template(CountdownCardTemplateId(*id)) {
                log::warn!("Failed to delete template {}: {}", id, e);
            }
        }

        // Insert or update current templates
        for template in &mut self.templates {
            if existing_tmpl_ids.contains(&template.id.0) {
                repo.update_template(template)?;
            } else {
                let new_id = repo.insert_template(template)?;
                if new_id != template.id {
                    // Update category references to use the new template ID
                    for cat in &mut self.categories {
                        if cat.template_id == Some(template.id) {
                            cat.template_id = Some(new_id);
                        }
                    }
                    template.id = new_id;
                }
            }
        }

        // Sync categories (cards reference categories via FK)
        let existing_categories = repo.get_all_categories()?;
        let existing_cat_ids: std::collections::HashSet<i64> =
            existing_categories.iter().map(|c| c.id.0).collect();
        let current_cat_ids: std::collections::HashSet<i64> =
            self.categories.iter().map(|c| c.id.0).collect();

        // Delete removed categories (reassigns cards to General)
        for id in existing_cat_ids.difference(&current_cat_ids) {
            use super::models::CountdownCategoryId;
            if let Err(e) = repo.delete_category(CountdownCategoryId(*id)) {
                log::warn!("Failed to delete category {}: {}", id, e);
            }
        }

        // Insert or update current categories, collecting ID remaps
        let mut id_remaps: Vec<(super::models::CountdownCategoryId, super::models::CountdownCategoryId)> = Vec::new();
        for category in &self.categories {
            if existing_cat_ids.contains(&category.id.0) {
                repo.update_category(category)?;
            } else {
                let new_id = repo.insert_category(category)?;
                if new_id != category.id {
                    id_remaps.push((category.id, new_id));
                }
            }
        }

        // Apply any ID remaps from newly-inserted categories
        for (old_id, new_id) in &id_remaps {
            for card in &mut self.cards {
                if card.category_id == *old_id {
                    card.category_id = *new_id;
                }
            }
            if let Some(cat) = self.categories.iter_mut().find(|c| c.id == *old_id) {
                cat.id = *new_id;
            }
        }

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
    pub fn delete_card_from_database(
        &self,
        conn: &Connection,
        id: CountdownCardId,
    ) -> Result<bool> {
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
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Local};
    use tempfile::tempdir;

    use super::super::service::CountdownService;

    #[test]
    fn persist_and_reload_cards() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("countdowns.json");
        let mut service = CountdownService::new();
        let target_start = Local::now() + Duration::days(10);
        service.create_card(
            None, "Persist", target_start, None, None, None, None, 120.0, 110.0,
        );
        service.save_to_disk(&file_path).unwrap();

        let loaded = CountdownService::load_from_disk(&file_path).unwrap();
        assert_eq!(loaded.cards().len(), 1);
        assert_eq!(loaded.cards()[0].event_title, "Persist");
    }
}
