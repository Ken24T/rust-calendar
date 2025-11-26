use std::{
    path::Path,
    time::{Duration, Instant},
};

use anyhow::Result;
use chrono::{DateTime, Local};

use super::models::{
    default_body_bg_color, default_days_fg_color, default_days_font_size, default_title_bg_color,
    default_title_fg_color, default_title_font_size, CountdownAutoDismissConfig,
    CountdownCardGeometry, CountdownCardId, CountdownCardState, CountdownCardVisuals,
    CountdownNotificationConfig, CountdownPersistedState, CountdownWarningState, RgbaColor,
    MAX_DAYS_FONT_SIZE, MIN_DAYS_FONT_SIZE,
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
    notification_config: CountdownNotificationConfig,
    auto_dismiss_defaults: CountdownAutoDismissConfig,
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
        }
        snapshot.visual_defaults.days_font_size = snapshot
            .visual_defaults
            .days_font_size
            .clamp(MIN_DAYS_FONT_SIZE, MAX_DAYS_FONT_SIZE);

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
        }
    }

    pub fn snapshot(&self) -> CountdownPersistedState {
        CountdownPersistedState {
            next_id: self.next_id,
            cards: self.cards.clone(),
            visual_defaults: self.visual_defaults.clone(),
            app_window_geometry: self.app_window_geometry,
            notification_config: self.notification_config.clone(),
            auto_dismiss_defaults: self.auto_dismiss_defaults.clone(),
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

    pub fn create_card(
        &mut self,
        event_id: Option<i64>,
        event_title: impl Into<String>,
        start_at: DateTime<Local>,
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
        self.dirty = true;
        id
    }

    pub fn remove_card(&mut self, id: CountdownCardId) -> bool {
        if let Some(idx) = self.cards.iter().position(|card| card.id == id) {
            let card = self.cards.remove(idx);
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

    /// Remove all countdown cards that reference the given event IDs.
    /// Used to clean up orphaned cards when their events are deleted.
    pub fn remove_cards_for_events(&mut self, event_ids: &[i64]) -> usize {
        let initial_count = self.cards.len();
        self.cards.retain(|card| {
            if let Some(event_id) = card.event_id {
                if event_ids.contains(&event_id) {
                    log::info!(
                        "Removing orphaned countdown card {:?} for deleted event {}",
                        card.id,
                        event_id
                    );
                    return false;
                }
            }
            true
        });
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

    pub fn set_compact_mode(&mut self, id: CountdownCardId, compact_mode: bool) -> bool {
        self.update_visual_flag(id, |visuals| visuals.compact_mode = compact_mode)
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

#[derive(Clone, Copy)]
struct EventPalette {
    title_bg: RgbaColor,
    title_fg: RgbaColor,
    body_bg: RgbaColor,
    days_fg: RgbaColor,
}

fn event_palette_for(card: &CountdownCardState) -> Option<EventPalette> {
    card.event_color.map(EventPalette::from_base)
}

fn apply_event_palette_if_needed(card: &mut CountdownCardState) {
    let Some(palette) = event_palette_for(card) else {
        return;
    };

    if !card.visuals.use_default_title_bg {
        card.visuals.title_bg_color = palette.title_bg;
    }
    if !card.visuals.use_default_title_fg {
        card.visuals.title_fg_color = palette.title_fg;
    }
    if !card.visuals.use_default_body_bg {
        card.visuals.body_bg_color = palette.body_bg;
    }
    if !card.visuals.use_default_days_fg {
        card.visuals.days_fg_color = palette.days_fg;
    }
}

impl EventPalette {
    fn from_base(base: RgbaColor) -> Self {
        let title_bg = darken_color(base, 0.18);
        let body_bg = lighten_color(base, 0.12);
        let title_fg = readable_text_color(title_bg);
        let days_fg = readable_text_color(body_bg);
        Self {
            title_bg,
            title_fg,
            body_bg,
            days_fg,
        }
    }
}

fn readable_text_color(bg: RgbaColor) -> RgbaColor {
    const LIGHT: RgbaColor = RgbaColor::new(255, 255, 255, 255);
    const DARK: RgbaColor = RgbaColor::new(20, 28, 45, 255);
    if relative_luminance(bg) > 0.5 {
        DARK
    } else {
        LIGHT
    }
}

fn lighten_color(color: RgbaColor, factor: f32) -> RgbaColor {
    mix_colors(color, RgbaColor::new(255, 255, 255, color.a), factor)
}

fn darken_color(color: RgbaColor, factor: f32) -> RgbaColor {
    mix_colors(color, RgbaColor::new(0, 0, 0, color.a), factor)
}

fn mix_colors(base: RgbaColor, target: RgbaColor, factor: f32) -> RgbaColor {
    let weight = factor.clamp(0.0, 1.0);
    let mix = |start: u8, end: u8| -> u8 {
        let start_f = start as f32;
        let end_f = end as f32;
        ((start_f + (end_f - start_f) * weight).round()).clamp(0.0, 255.0) as u8
    };
    RgbaColor::new(
        mix(base.r, target.r),
        mix(base.g, target.g),
        mix(base.b, target.b),
        base.a,
    )
}

fn relative_luminance(color: RgbaColor) -> f32 {
    fn srgb_component(value: u8) -> f32 {
        let channel = value as f32 / 255.0;
        if channel <= 0.03928 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    let r = srgb_component(color.r);
    let g = srgb_component(color.g);
    let b = srgb_component(color.b);
    0.2126 * r + 0.7152 * g + 0.0722 * b
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
        service.create_card(None, "Persist", target_start, None, None, 120.0, 110.0);
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
            service.create_card(Some(1), "A", target_start, Some(accent), None, 120.0, 110.0);
        let card_b =
            service.create_card(Some(2), "B", target_start, Some(accent), None, 120.0, 110.0);

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
}
