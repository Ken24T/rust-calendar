use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::{self, Error as SerdeError};
use std::{fs, path::Path, time::{Duration, Instant}};

use anyhow::{Context, Result};

/// Unique identifier for countdown cards. We start with a monotonic u64 so we
/// can serialize it easily and evolve to UUIDs later if needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CountdownCardId(pub u64);

/// Geometry data we persist for each card so they reopen at the same spot.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct CountdownCardGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Visual preferences that persist per card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountdownCardVisuals {
    pub accent_color: Option<String>,
    pub always_on_top: bool,
    pub compact_mode: bool,
}

impl Default for CountdownCardVisuals {
    fn default() -> Self {
        Self {
            accent_color: None,
            always_on_top: false,
            compact_mode: false,
        }
    }
}

/// Core persisted information for each countdown card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountdownCardState {
    pub id: CountdownCardId,
    pub event_id: Option<i64>,
    pub event_title: String,
    pub start_at: DateTime<Local>,
    pub title_override: Option<String>,
    pub geometry: CountdownCardGeometry,
    pub visuals: CountdownCardVisuals,
    pub last_computed_days: Option<i64>,
}

impl CountdownCardState {
    /// Returns the title that should be rendered.
    pub fn effective_title(&self) -> &str {
        self.title_override.as_deref().unwrap_or(&self.event_title)
    }

    /// Updates the cached `last_computed_days` value.
    pub fn record_days_remaining(&mut self, days: i64) {
        self.last_computed_days = Some(days);
    }

    /// Calculate days remaining relative to now.
    pub fn compute_days_remaining(&self, now: DateTime<Local>) -> i64 {
        let start_date = self.start_at.date_naive();
        let today = now.date_naive();
        (start_date - today).num_days().max(0)
    }
}

/// Serializable container for persisting card state between sessions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CountdownPersistedState {
    pub next_id: u64,
    pub cards: Vec<CountdownCardState>,
}

/// Manages active countdown cards while the calendar app is running.
pub struct CountdownService {
    cards: Vec<CountdownCardState>,
    next_id: u64,
    dirty: bool,
    pending_geometry: Vec<(CountdownCardId, CountdownCardGeometry)>,
    last_geometry_update: Option<Instant>,
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
        }
    }

    pub fn snapshot(&self) -> CountdownPersistedState {
        CountdownPersistedState {
            next_id: self.next_id,
            cards: self.cards.clone(),
        }
    }

    pub fn load_from_disk(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let data = fs::read_to_string(path)
            .with_context(|| format!("failed to read countdowns from {}", path.display()))?;
        let snapshot: CountdownPersistedState =
            serde_json::from_str(&data).map_err(|err| map_deser_error(err, path))?;
        Ok(Self::from_snapshot(snapshot))
    }

    pub fn save_to_disk(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create dir {}", parent.display()))?;
        }
        let snapshot = self.snapshot();
        let data = serde_json::to_string_pretty(&snapshot)?;
        fs::write(path, data)
            .with_context(|| format!("failed to write countdowns to {}", path.display()))
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
    ) {
        self.pending_geometry.push((id, geometry));
        self.last_geometry_update = Some(Instant::now());
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
        let mut card = CountdownCardState {
            id,
            event_id,
            event_title: event_title.into(),
            start_at,
            title_override: None,
            geometry: CountdownCardGeometry {
                x: 50.0,
                y: 50.0,
                width: 138.0,
                height: 128.0,
            },
            visuals: CountdownCardVisuals::default(),
            last_computed_days: None,
        };
        let days = card.compute_days_remaining(Local::now());
        card.record_days_remaining(days);
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

fn map_deser_error(err: SerdeError, path: &Path) -> anyhow::Error {
    anyhow::Error::new(err).context(format!(
        "failed to deserialize countdowns from {}",
        path.display()
    ))
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
