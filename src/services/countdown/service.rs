use std::{
    path::Path,
    time::{Duration, Instant},
};

use anyhow::Result;
use chrono::{DateTime, Local};

use super::models::{
    default_body_bg_color, default_days_fg_color, default_days_font_size, default_title_bg_color,
    default_title_fg_color, default_title_font_size, CountdownCardGeometry, CountdownCardId,
    CountdownCardState, CountdownCardVisuals, CountdownPersistedState, RgbaColor,
};
use super::persistence::{load_snapshot, save_snapshot};

/// Manages active countdown cards while the calendar app is running.
pub struct CountdownService {
    cards: Vec<CountdownCardState>,
    next_id: u64,
    dirty: bool,
    pending_geometry: Vec<(CountdownCardId, CountdownCardGeometry)>,
    last_geometry_update: Option<Instant>,
    visual_defaults: CountdownCardVisuals,
    app_window_geometry: Option<CountdownCardGeometry>,
}

impl CountdownService {
    pub fn new() -> Self {
        Self::from_snapshot(CountdownPersistedState::default())
    }

    pub fn from_snapshot(snapshot: CountdownPersistedState) -> Self {
        Self {
            cards: snapshot.cards,
            next_id: snapshot.next_id.max(1),
            dirty: false,
            pending_geometry: Vec::new(),
            last_geometry_update: None,
            visual_defaults: snapshot.visual_defaults,
            app_window_geometry: snapshot.app_window_geometry,
        }
    }

    pub fn snapshot(&self) -> CountdownPersistedState {
        CountdownPersistedState {
            next_id: self.next_id,
            cards: self.cards.clone(),
            visual_defaults: self.visual_defaults.clone(),
            app_window_geometry: self.app_window_geometry,
        }
    }

    pub fn load_from_disk(path: &Path) -> Result<Self> {
        let snapshot = load_snapshot(path)?;
        Ok(Self::from_snapshot(snapshot))
    }

    pub fn save_to_disk(&self, path: &Path) -> Result<()> {
        let snapshot = self.snapshot();
        save_snapshot(path, &snapshot)
    }

    pub fn cards(&self) -> &[CountdownCardState] {
        &self.cards
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

    pub fn create_card(
        &mut self,
        event_id: Option<i64>,
        event_title: impl Into<String>,
        start_at: DateTime<Local>,
    ) -> CountdownCardId {
        let id = CountdownCardId(self.next_id);
        self.next_id += 1;
        let card = CountdownCardState {
            id,
            event_id,
            event_title: event_title.into(),
            start_at,
            title_override: None,
            geometry: CountdownCardGeometry {
                x: 50.0,
                y: 50.0,
                width: 120.0,
                height: 110.0,
            },
            visuals: self.visual_defaults.clone(),
            last_computed_days: None,
            comment: None,
        };
        self.cards.push(card);
        self.dirty = true;
        id
    }

    pub fn remove_card(&mut self, id: CountdownCardId) -> bool {
        if let Some(idx) = self.cards.iter().position(|card| card.id == id) {
            self.cards.remove(idx);
            self.dirty = true;
            return true;
        }
        false
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
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn set_always_on_top(&mut self, id: CountdownCardId, always_on_top: bool) -> bool {
        self.update_visual_flag(id, |visuals| visuals.always_on_top = always_on_top)
    }

    pub fn set_compact_mode(&mut self, id: CountdownCardId, compact_mode: bool) -> bool {
        self.update_visual_flag(id, |visuals| visuals.compact_mode = compact_mode)
    }

    pub fn set_title_bg_color(&mut self, id: CountdownCardId, color: RgbaColor) -> bool {
        self.update_visual_flag(id, |visuals| visuals.title_bg_color = color)
    }

    pub fn set_title_fg_color(&mut self, id: CountdownCardId, color: RgbaColor) -> bool {
        self.update_visual_flag(id, |visuals| visuals.title_fg_color = color)
    }

    pub fn set_body_bg_color(&mut self, id: CountdownCardId, color: RgbaColor) -> bool {
        self.update_visual_flag(id, |visuals| visuals.body_bg_color = color)
    }

    pub fn set_days_fg_color(&mut self, id: CountdownCardId, color: RgbaColor) -> bool {
        self.update_visual_flag(id, |visuals| visuals.days_fg_color = color)
    }

    pub fn set_days_font_size(&mut self, id: CountdownCardId, size: f32) -> bool {
        self.update_visual_flag(id, |visuals| visuals.days_font_size = size.max(16.0))
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
        self.visual_defaults.days_font_size = size.max(16.0);
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
        let card_id = service.create_card(Some(42), "Sample Event", target_start);

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
        service.create_card(None, "Persist", target_start);
        service.save_to_disk(&file_path).unwrap();

        let loaded = CountdownService::load_from_disk(&file_path).unwrap();
        assert_eq!(loaded.cards().len(), 1);
        assert_eq!(loaded.cards()[0].event_title, "Persist");
    }
}
