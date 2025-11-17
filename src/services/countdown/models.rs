use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Unique identifier for countdown cards. We start with a monotonic u64 so we
/// can serialize it easily and evolve to UUIDs later if needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CountdownCardId(pub u64);

/// Geometry data we persist for each card so they reopen at the same spot.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub struct CountdownCardGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Visual preferences that persist per card.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CountdownCardVisuals {
    pub accent_color: Option<String>,
    pub always_on_top: bool,
    pub compact_mode: bool,
    #[serde(default = "default_use_default_title_bg")]
    pub use_default_title_bg: bool,
    #[serde(default = "default_title_bg_color")]
    pub title_bg_color: RgbaColor,
    #[serde(default = "default_use_default_title_fg")]
    pub use_default_title_fg: bool,
    #[serde(default = "default_title_fg_color")]
    pub title_fg_color: RgbaColor,
    #[serde(default = "default_title_font_size")]
    pub title_font_size: f32,
    #[serde(default = "default_use_default_body_bg")]
    pub use_default_body_bg: bool,
    #[serde(default = "default_body_bg_color")]
    pub body_bg_color: RgbaColor,
    #[serde(default = "default_use_default_days_fg")]
    pub use_default_days_fg: bool,
    #[serde(default = "default_days_fg_color")]
    pub days_fg_color: RgbaColor,
    #[serde(default = "default_days_font_size")]
    pub days_font_size: f32,
}

impl Default for CountdownCardVisuals {
    fn default() -> Self {
        Self {
            accent_color: None,
            always_on_top: false,
            compact_mode: false,
            use_default_title_bg: true,
            title_bg_color: default_title_bg_color(),
            use_default_title_fg: true,
            title_fg_color: default_title_fg_color(),
            title_font_size: default_title_font_size(),
            use_default_body_bg: true,
            body_bg_color: default_body_bg_color(),
            use_default_days_fg: true,
            days_fg_color: default_days_fg_color(),
            days_font_size: default_days_font_size(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RgbaColor {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex_str(value: &str) -> Option<Self> {
        let trimmed = value.trim();
        let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
        if hex.len() != 6 && hex.len() != 8 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).ok()?
        } else {
            255
        };

        Some(RgbaColor::new(r, g, b, a))
    }
}

impl Default for RgbaColor {
    fn default() -> Self {
        RgbaColor::new(0, 0, 0, 255)
    }
}

pub(crate) const fn default_title_bg_color() -> RgbaColor {
    RgbaColor::new(10, 34, 145, 255)
}

pub(crate) const fn default_title_fg_color() -> RgbaColor {
    RgbaColor::new(255, 255, 255, 255)
}

pub(crate) const fn default_title_font_size() -> f32 {
    20.0
}

pub(crate) const fn default_body_bg_color() -> RgbaColor {
    RgbaColor::new(103, 176, 255, 255)
}

pub(crate) const fn default_days_fg_color() -> RgbaColor {
    RgbaColor::new(15, 32, 70, 255)
}

pub(crate) const fn default_days_font_size() -> f32 {
    80.0
}

const fn default_use_default_title_bg() -> bool {
    true
}

const fn default_use_default_title_fg() -> bool {
    true
}

const fn default_use_default_body_bg() -> bool {
    true
}

const fn default_use_default_days_fg() -> bool {
    true
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
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub event_color: Option<RgbaColor>,
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
    #[serde(default = "default_visuals")]
    pub visual_defaults: CountdownCardVisuals,
    #[serde(default)]
    pub app_window_geometry: Option<CountdownCardGeometry>,
}

pub(crate) fn default_visuals() -> CountdownCardVisuals {
    CountdownCardVisuals::default()
}
