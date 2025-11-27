use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Warning state for countdown cards based on time remaining
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CountdownWarningState {
    /// More than 1 day remaining
    Normal,
    /// 1 day or less, but more than 1 hour
    Approaching,
    /// 1 hour or less, but more than 5 minutes
    Imminent,
    /// 5 minutes or less, but event hasn't started
    Critical,
    /// Event has started (time <= 0)
    Starting,
}

impl Default for CountdownWarningState {
    fn default() -> Self {
        Self::Normal
    }
}

/// Configuration for countdown card notifications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CountdownNotificationConfig {
    pub enabled: bool,
    pub use_visual_warnings: bool,
    pub use_system_notifications: bool,
    pub warning_thresholds: WarningThresholds,
}

impl Default for CountdownNotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            use_visual_warnings: true,
            use_system_notifications: true,
            warning_thresholds: WarningThresholds::default(),
        }
    }
}

/// Thresholds for different warning states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WarningThresholds {
    /// Hours before event to enter "approaching" state (default: 24)
    pub approaching_hours: u32,
    /// Hours before event to enter "imminent" state (default: 1)
    pub imminent_hours: u32,
    /// Minutes before event to enter "critical" state (default: 5)
    pub critical_minutes: u32,
}

impl Default for WarningThresholds {
    fn default() -> Self {
        Self {
            approaching_hours: 24,
            imminent_hours: 1,
            critical_minutes: 5,
        }
    }
}

/// Configuration for auto-dismiss behavior
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CountdownAutoDismissConfig {
    pub enabled: bool,
    pub on_event_start: bool,
    pub on_event_end: bool,
    pub delay_seconds: u32,
}

impl Default for CountdownAutoDismissConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            on_event_start: true,
            on_event_end: false,
            delay_seconds: 10,
        }
    }
}

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

impl CountdownCardGeometry {
    /// Check if this geometry is plausible (has reasonable size and position)
    pub fn is_plausible(&self) -> bool {
        // Check for reasonable size
        self.width >= 20.0 && self.height >= 20.0 &&
        // Check for valid (non-NaN, non-infinite) values
        self.x.is_finite() && self.y.is_finite() &&
        self.width.is_finite() && self.height.is_finite() &&
        // Check for reasonable position (not absurdly far off-screen)
        // Allow positions within ±10000 pixels to support multi-monitor setups
        self.x.abs() < 10000.0 && self.y.abs() < 10000.0
    }
    
    /// Sanitize this geometry to ensure it's visible within the given screen bounds.
    /// If position is outside all monitors, move it to a default visible position.
    /// The monitors parameter is a list of (x, y, width, height) tuples representing
    /// the virtual desktop bounds (can be a single large region for multi-monitor).
    pub fn sanitize_for_monitors(&self, monitors: &[(f32, f32, f32, f32)], default_pos: (f32, f32)) -> Self {
        // If the geometry is plausible (valid finite values, reasonable range),
        // trust it - the user may have multiple monitors we don't know about
        if self.is_plausible() {
            return *self;
        }
        
        // Geometry is invalid (NaN, infinite, or absurdly positioned) - reset to default
        log::warn!(
            "Geometry {:?} is not plausible, resetting to default position {:?}",
            self, default_pos
        );
        
        // Use the first monitor bounds or fallback
        let (mx, my, _mw, _mh) = monitors.first()
            .copied()
            .unwrap_or((0.0, 0.0, 1920.0, 1080.0));
        
        Self {
            x: default_pos.0.max(mx),
            y: default_pos.1.max(my),
            width: self.width.max(100.0).min(800.0),
            height: self.height.max(100.0).min(600.0),
        }
    }
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
            use_default_title_bg: false,
            title_bg_color: default_title_bg_color(),
            use_default_title_fg: false,
            title_fg_color: default_title_fg_color(),
            title_font_size: default_title_font_size(),
            use_default_body_bg: false,
            body_bg_color: default_body_bg_color(),
            use_default_days_fg: false,
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

pub const MIN_DAYS_FONT_SIZE: f32 = 16.0;
pub const MAX_DAYS_FONT_SIZE: f32 = 512.0;

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
    #[serde(default)]
    pub auto_title_override: bool,
    pub geometry: CountdownCardGeometry,
    pub visuals: CountdownCardVisuals,
    pub last_computed_days: Option<i64>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub event_color: Option<RgbaColor>,
    /// Tracks the last warning state to detect transitions
    #[serde(default)]
    pub last_warning_state: Option<CountdownWarningState>,
    /// Last time a notification was sent for this card
    #[serde(default)]
    pub last_notification_time: Option<DateTime<Local>>,
    /// Auto-dismiss configuration for this card
    #[serde(default)]
    pub auto_dismiss: CountdownAutoDismissConfig,
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

    /// Calculate the current warning state based on time remaining and thresholds.
    pub fn warning_state(
        &self,
        now: DateTime<Local>,
        thresholds: &WarningThresholds,
    ) -> CountdownWarningState {
        let remaining = self.start_at.signed_duration_since(now);

        if remaining.num_seconds() <= 0 {
            CountdownWarningState::Starting
        } else if remaining.num_minutes() <= thresholds.critical_minutes as i64 {
            CountdownWarningState::Critical
        } else if remaining.num_hours() <= thresholds.imminent_hours as i64 {
            CountdownWarningState::Imminent
        } else if remaining.num_hours() <= thresholds.approaching_hours as i64 {
            CountdownWarningState::Approaching
        } else {
            CountdownWarningState::Normal
        }
    }

    /// Check if this card should be auto-dismissed based on current time.
    pub fn should_auto_dismiss(&self, now: DateTime<Local>) -> bool {
        if !self.auto_dismiss.enabled {
            return false;
        }

        let remaining = self.start_at.signed_duration_since(now);
        let seconds_past_start = -remaining.num_seconds();

        if self.auto_dismiss.on_event_start
            && seconds_past_start >= self.auto_dismiss.delay_seconds as i64
        {
            return true;
        }

        // TODO: Implement on_event_end when we have event end times

        false
    }
}

/// Display mode for countdown cards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CountdownDisplayMode {
    /// Each card in its own separate window
    IndividualWindows,
    /// All cards in a single container window
    Container,
}

impl Default for CountdownDisplayMode {
    fn default() -> Self {
        Self::IndividualWindows
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
    /// Global notification configuration
    #[serde(default)]
    pub notification_config: CountdownNotificationConfig,
    /// Default auto-dismiss configuration for new cards
    #[serde(default)]
    pub auto_dismiss_defaults: CountdownAutoDismissConfig,
    /// Display mode for countdown cards (Individual Windows or Container)
    #[serde(default)]
    pub display_mode: CountdownDisplayMode,
    /// Container window geometry (position and size)
    #[serde(default)]
    pub container_geometry: Option<CountdownCardGeometry>,
    /// Manual card ordering for container mode (list of card IDs)
    #[serde(default)]
    pub card_order: Vec<CountdownCardId>,
}

pub(crate) fn default_visuals() -> CountdownCardVisuals {
    CountdownCardVisuals::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_countdown_display_mode_default() {
        let mode = CountdownDisplayMode::default();
        assert_eq!(mode, CountdownDisplayMode::IndividualWindows);
    }

    #[test]
    fn test_countdown_display_mode_serialization() {
        // Test IndividualWindows serialization
        let individual = CountdownDisplayMode::IndividualWindows;
        let json = serde_json::to_string(&individual).unwrap();
        let deserialized: CountdownDisplayMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, CountdownDisplayMode::IndividualWindows);

        // Test Container serialization
        let container = CountdownDisplayMode::Container;
        let json = serde_json::to_string(&container).unwrap();
        let deserialized: CountdownDisplayMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, CountdownDisplayMode::Container);
    }

    #[test]
    fn test_persisted_state_defaults_to_individual_windows() {
        let state = CountdownPersistedState::default();
        assert_eq!(state.display_mode, CountdownDisplayMode::IndividualWindows);
    }

    #[test]
    fn test_persisted_state_serialization_with_display_mode() {
        let mut state = CountdownPersistedState::default();
        state.display_mode = CountdownDisplayMode::Container;
        state.next_id = 42;

        // Serialize
        let json = serde_json::to_string(&state).unwrap();

        // Deserialize
        let deserialized: CountdownPersistedState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.display_mode, CountdownDisplayMode::Container);
        assert_eq!(deserialized.next_id, 42);
    }

    #[test]
    fn test_persisted_state_backward_compatibility() {
        // Test that default() provides IndividualWindows
        let state = CountdownPersistedState::default();
        assert_eq!(state.display_mode, CountdownDisplayMode::IndividualWindows);
    }

    #[test]
    fn test_container_geometry_defaults_to_none() {
        let state = CountdownPersistedState::default();
        assert_eq!(state.container_geometry, None);
    }

    #[test]
    fn test_card_order_defaults_to_empty() {
        let state = CountdownPersistedState::default();
        assert!(state.card_order.is_empty());
    }

    #[test]
    fn test_container_geometry_serialization() {
        let mut state = CountdownPersistedState::default();
        state.container_geometry = Some(CountdownCardGeometry {
            x: 100.0,
            y: 200.0,
            width: 800.0,
            height: 600.0,
        });

        // Serialize
        let json = serde_json::to_string(&state).unwrap();

        // Deserialize
        let deserialized: CountdownPersistedState = serde_json::from_str(&json).unwrap();
        assert!(deserialized.container_geometry.is_some());
        let geom = deserialized.container_geometry.unwrap();
        assert_eq!(geom.x, 100.0);
        assert_eq!(geom.y, 200.0);
        assert_eq!(geom.width, 800.0);
        assert_eq!(geom.height, 600.0);
    }

    #[test]
    fn test_card_order_serialization() {
        let mut state = CountdownPersistedState::default();
        state.card_order = vec![
            CountdownCardId(1),
            CountdownCardId(3),
            CountdownCardId(2),
        ];

        // Serialize
        let json = serde_json::to_string(&state).unwrap();

        // Deserialize
        let deserialized: CountdownPersistedState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.card_order.len(), 3);
        assert_eq!(deserialized.card_order[0], CountdownCardId(1));
        assert_eq!(deserialized.card_order[1], CountdownCardId(3));
        assert_eq!(deserialized.card_order[2], CountdownCardId(2));
    }
}
