//! Per-card and default visual setters for countdown cards.
//!
//! These methods manage colour, font-size, and "use default" flag
//! mutations on [`CountdownService`] and its cards.

use super::models::{
    default_body_bg_color, default_days_fg_color, default_days_font_size, default_title_bg_color,
    default_title_fg_color, default_title_font_size, CountdownCardId, CountdownCardState,
    CountdownCardVisuals, RgbaColor, MAX_DAYS_FONT_SIZE, MIN_DAYS_FONT_SIZE,
};
use super::palette::apply_event_palette_if_needed;
use super::service::CountdownService;

// ---------------------------------------------------------------------------
// Per-card visual setters
// ---------------------------------------------------------------------------

impl CountdownService {
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

    pub fn apply_visual_defaults(&mut self, id: CountdownCardId) -> bool {
        if let Some(card) = self.cards.iter_mut().find(|card| card.id == id) {
            card.visuals = self.visual_defaults.clone();
            self.dirty = true;
            return true;
        }
        false
    }

    /// Returns the current visual defaults (alias: [`visual_defaults`](Self::visual_defaults)).
    pub fn defaults(&self) -> &CountdownCardVisuals {
        &self.visual_defaults
    }
}

// ---------------------------------------------------------------------------
// Global / default visual setters
// ---------------------------------------------------------------------------

impl CountdownService {
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
}

// ---------------------------------------------------------------------------
// Private helpers (used by per-card setters above)
// ---------------------------------------------------------------------------

impl CountdownService {
    pub(super) fn update_visual_flag<F>(&mut self, id: CountdownCardId, mut update: F) -> bool
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

    pub(super) fn update_card_state<F>(&mut self, id: CountdownCardId, mut update: F) -> bool
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Local};

    #[test]
    fn set_title_bg_color_clears_default_flag() {
        let mut svc = CountdownService::new();
        let t = Local::now() + Duration::days(5);
        let id = svc.create_card(Some(1), "Evt", t, None, None, None, None, 120.0, 110.0);

        let accent = RgbaColor::new(200, 50, 80, 255);
        assert!(svc.set_title_bg_color(id, accent));
        let card = svc.cards().iter().find(|c| c.id == id).unwrap();
        assert_eq!(card.visuals.title_bg_color, accent);
        assert!(!card.visuals.use_default_title_bg);
    }

    #[test]
    fn set_default_body_bg_round_trips() {
        let mut svc = CountdownService::new();
        let color = RgbaColor::new(10, 20, 30, 255);
        svc.set_default_body_bg_color(color);
        assert_eq!(svc.visual_defaults().body_bg_color, color);
        assert!(svc.is_dirty());
    }

    #[test]
    fn reset_default_days_font_size_uses_constant() {
        let mut svc = CountdownService::new();
        svc.set_default_days_font_size(99.0);
        svc.reset_default_days_font_size();
        assert!((svc.visual_defaults().days_font_size - default_days_font_size()).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_visual_defaults_replaces_card_visuals() {
        let mut svc = CountdownService::new();
        let t = Local::now() + Duration::days(3);
        let id = svc.create_card(Some(1), "Card", t, None, None, None, None, 120.0, 110.0);

        let custom = RgbaColor::new(255, 0, 0, 255);
        svc.set_title_bg_color(id, custom);

        assert!(svc.apply_visual_defaults(id));
        let card = svc.cards().iter().find(|c| c.id == id).unwrap();
        assert_eq!(card.visuals, *svc.visual_defaults());
    }

    #[test]
    fn font_size_clamped_to_bounds() {
        let mut svc = CountdownService::new();
        let t = Local::now() + Duration::days(1);
        let id = svc.create_card(Some(1), "F", t, None, None, None, None, 120.0, 110.0);

        assert!(svc.set_days_font_size(id, 999.0));
        let card = svc.cards().iter().find(|c| c.id == id).unwrap();
        assert!(card.visuals.days_font_size <= MAX_DAYS_FONT_SIZE);

        assert!(svc.set_title_font_size(id, 1.0));
        let card = svc.cards().iter().find(|c| c.id == id).unwrap();
        assert!(card.visuals.title_font_size >= 10.0);
    }

    #[test]
    fn defaults_and_visual_defaults_are_identical() {
        let svc = CountdownService::new();
        assert!(std::ptr::eq(svc.defaults(), svc.visual_defaults()));
    }
}
