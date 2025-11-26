//! Database repository for countdown cards.
//!
//! This module provides CRUD operations for countdown cards stored in SQLite.

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use rusqlite::{params, Connection, OptionalExtension, Row};

use super::models::{
    CountdownAutoDismissConfig, CountdownCardGeometry, CountdownCardId, CountdownCardState,
    CountdownCardVisuals, CountdownNotificationConfig, CountdownWarningState, RgbaColor,
    WarningThresholds,
};

/// Global countdown settings stored in the database
#[derive(Debug, Clone)]
pub struct CountdownGlobalSettings {
    pub next_card_id: u64,
    pub app_window_geometry: Option<CountdownCardGeometry>,
    pub visual_defaults: CountdownCardVisuals,
    pub notification_config: CountdownNotificationConfig,
    pub auto_dismiss_defaults: CountdownAutoDismissConfig,
}

impl Default for CountdownGlobalSettings {
    fn default() -> Self {
        Self {
            next_card_id: 1,
            app_window_geometry: None,
            visual_defaults: CountdownCardVisuals::default(),
            notification_config: CountdownNotificationConfig::default(),
            auto_dismiss_defaults: CountdownAutoDismissConfig::default(),
        }
    }
}

/// Repository for countdown card database operations
pub struct CountdownRepository<'a> {
    conn: &'a Connection,
}

impl<'a> CountdownRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ========== Card CRUD Operations ==========

    /// Get all countdown cards from the database
    pub fn get_all_cards(&self) -> Result<Vec<CountdownCardState>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_id, event_title, start_at, title_override, auto_title_override,
                    comment, event_color,
                    geometry_x, geometry_y, geometry_width, geometry_height,
                    accent_color, always_on_top, compact_mode,
                    use_default_title_bg, title_bg_r, title_bg_g, title_bg_b, title_bg_a,
                    use_default_title_fg, title_fg_r, title_fg_g, title_fg_b, title_fg_a,
                    title_font_size,
                    use_default_body_bg, body_bg_r, body_bg_g, body_bg_b, body_bg_a,
                    use_default_days_fg, days_fg_r, days_fg_g, days_fg_b, days_fg_a,
                    days_font_size,
                    auto_dismiss_enabled, auto_dismiss_on_event_start, auto_dismiss_on_event_end,
                    auto_dismiss_delay_seconds,
                    last_computed_days, last_warning_state, last_notification_time
             FROM countdown_cards
             ORDER BY id",
        )?;

        let cards = stmt
            .query_map([], |row| Ok(row_to_card_state(row)?))?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to fetch countdown cards")?;

        Ok(cards)
    }

    /// Get a single card by ID
    #[allow(dead_code)]
    pub fn get_card(&self, id: CountdownCardId) -> Result<Option<CountdownCardState>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_id, event_title, start_at, title_override, auto_title_override,
                    comment, event_color,
                    geometry_x, geometry_y, geometry_width, geometry_height,
                    accent_color, always_on_top, compact_mode,
                    use_default_title_bg, title_bg_r, title_bg_g, title_bg_b, title_bg_a,
                    use_default_title_fg, title_fg_r, title_fg_g, title_fg_b, title_fg_a,
                    title_font_size,
                    use_default_body_bg, body_bg_r, body_bg_g, body_bg_b, body_bg_a,
                    use_default_days_fg, days_fg_r, days_fg_g, days_fg_b, days_fg_a,
                    days_font_size,
                    auto_dismiss_enabled, auto_dismiss_on_event_start, auto_dismiss_on_event_end,
                    auto_dismiss_delay_seconds,
                    last_computed_days, last_warning_state, last_notification_time
             FROM countdown_cards WHERE id = ?",
        )?;

        stmt.query_row([id.0 as i64], |row| Ok(row_to_card_state(row)?))
            .optional()
            .context("Failed to fetch countdown card")
    }

    /// Get a card by its associated event ID
    #[allow(dead_code)]
    pub fn get_card_by_event_id(&self, event_id: i64) -> Result<Option<CountdownCardState>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_id, event_title, start_at, title_override, auto_title_override,
                    comment, event_color,
                    geometry_x, geometry_y, geometry_width, geometry_height,
                    accent_color, always_on_top, compact_mode,
                    use_default_title_bg, title_bg_r, title_bg_g, title_bg_b, title_bg_a,
                    use_default_title_fg, title_fg_r, title_fg_g, title_fg_b, title_fg_a,
                    title_font_size,
                    use_default_body_bg, body_bg_r, body_bg_g, body_bg_b, body_bg_a,
                    use_default_days_fg, days_fg_r, days_fg_g, days_fg_b, days_fg_a,
                    days_font_size,
                    auto_dismiss_enabled, auto_dismiss_on_event_start, auto_dismiss_on_event_end,
                    auto_dismiss_delay_seconds,
                    last_computed_days, last_warning_state, last_notification_time
             FROM countdown_cards WHERE event_id = ?",
        )?;

        stmt.query_row([event_id], |row| Ok(row_to_card_state(row)?))
            .optional()
            .context("Failed to fetch countdown card by event_id")
    }

    /// Insert a new countdown card
    pub fn insert_card(&self, card: &CountdownCardState) -> Result<()> {
        let start_at_str = card.start_at.to_rfc3339();
        let event_color_str = card.event_color.map(|c| format!("{},{},{},{}", c.r, c.g, c.b, c.a));
        let last_warning_str = card.last_warning_state.map(warning_state_to_string);
        let last_notif_str = card.last_notification_time.map(|t| t.to_rfc3339());

        self.conn.execute(
            "INSERT INTO countdown_cards (
                id, event_id, event_title, start_at, title_override, auto_title_override,
                comment, event_color,
                geometry_x, geometry_y, geometry_width, geometry_height,
                accent_color, always_on_top, compact_mode,
                use_default_title_bg, title_bg_r, title_bg_g, title_bg_b, title_bg_a,
                use_default_title_fg, title_fg_r, title_fg_g, title_fg_b, title_fg_a,
                title_font_size,
                use_default_body_bg, body_bg_r, body_bg_g, body_bg_b, body_bg_a,
                use_default_days_fg, days_fg_r, days_fg_g, days_fg_b, days_fg_a,
                days_font_size,
                auto_dismiss_enabled, auto_dismiss_on_event_start, auto_dismiss_on_event_end,
                auto_dismiss_delay_seconds,
                last_computed_days, last_warning_state, last_notification_time
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                ?9, ?10, ?11, ?12,
                ?13, ?14, ?15,
                ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25,
                ?26,
                ?27, ?28, ?29, ?30, ?31,
                ?32, ?33, ?34, ?35, ?36,
                ?37,
                ?38, ?39, ?40, ?41,
                ?42, ?43, ?44
            )",
            params![
                card.id.0 as i64,
                card.event_id,
                card.event_title,
                start_at_str,
                card.title_override,
                card.auto_title_override,
                card.comment,
                event_color_str,
                card.geometry.x,
                card.geometry.y,
                card.geometry.width,
                card.geometry.height,
                card.visuals.accent_color,
                card.visuals.always_on_top,
                card.visuals.compact_mode,
                card.visuals.use_default_title_bg,
                card.visuals.title_bg_color.r,
                card.visuals.title_bg_color.g,
                card.visuals.title_bg_color.b,
                card.visuals.title_bg_color.a,
                card.visuals.use_default_title_fg,
                card.visuals.title_fg_color.r,
                card.visuals.title_fg_color.g,
                card.visuals.title_fg_color.b,
                card.visuals.title_fg_color.a,
                card.visuals.title_font_size,
                card.visuals.use_default_body_bg,
                card.visuals.body_bg_color.r,
                card.visuals.body_bg_color.g,
                card.visuals.body_bg_color.b,
                card.visuals.body_bg_color.a,
                card.visuals.use_default_days_fg,
                card.visuals.days_fg_color.r,
                card.visuals.days_fg_color.g,
                card.visuals.days_fg_color.b,
                card.visuals.days_fg_color.a,
                card.visuals.days_font_size,
                card.auto_dismiss.enabled,
                card.auto_dismiss.on_event_start,
                card.auto_dismiss.on_event_end,
                card.auto_dismiss.delay_seconds,
                card.last_computed_days,
                last_warning_str,
                last_notif_str,
            ],
        ).context("Failed to insert countdown card")?;

        Ok(())
    }

    /// Update an existing countdown card
    pub fn update_card(&self, card: &CountdownCardState) -> Result<bool> {
        let start_at_str = card.start_at.to_rfc3339();
        let event_color_str = card.event_color.map(|c| format!("{},{},{},{}", c.r, c.g, c.b, c.a));
        let last_warning_str = card.last_warning_state.map(warning_state_to_string);
        let last_notif_str = card.last_notification_time.map(|t| t.to_rfc3339());

        let rows = self.conn.execute(
            "UPDATE countdown_cards SET
                event_id = ?2, event_title = ?3, start_at = ?4, title_override = ?5,
                auto_title_override = ?6, comment = ?7, event_color = ?8,
                geometry_x = ?9, geometry_y = ?10, geometry_width = ?11, geometry_height = ?12,
                accent_color = ?13, always_on_top = ?14, compact_mode = ?15,
                use_default_title_bg = ?16, title_bg_r = ?17, title_bg_g = ?18, title_bg_b = ?19, title_bg_a = ?20,
                use_default_title_fg = ?21, title_fg_r = ?22, title_fg_g = ?23, title_fg_b = ?24, title_fg_a = ?25,
                title_font_size = ?26,
                use_default_body_bg = ?27, body_bg_r = ?28, body_bg_g = ?29, body_bg_b = ?30, body_bg_a = ?31,
                use_default_days_fg = ?32, days_fg_r = ?33, days_fg_g = ?34, days_fg_b = ?35, days_fg_a = ?36,
                days_font_size = ?37,
                auto_dismiss_enabled = ?38, auto_dismiss_on_event_start = ?39, auto_dismiss_on_event_end = ?40,
                auto_dismiss_delay_seconds = ?41,
                last_computed_days = ?42, last_warning_state = ?43, last_notification_time = ?44,
                updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![
                card.id.0 as i64,
                card.event_id,
                card.event_title,
                start_at_str,
                card.title_override,
                card.auto_title_override,
                card.comment,
                event_color_str,
                card.geometry.x,
                card.geometry.y,
                card.geometry.width,
                card.geometry.height,
                card.visuals.accent_color,
                card.visuals.always_on_top,
                card.visuals.compact_mode,
                card.visuals.use_default_title_bg,
                card.visuals.title_bg_color.r,
                card.visuals.title_bg_color.g,
                card.visuals.title_bg_color.b,
                card.visuals.title_bg_color.a,
                card.visuals.use_default_title_fg,
                card.visuals.title_fg_color.r,
                card.visuals.title_fg_color.g,
                card.visuals.title_fg_color.b,
                card.visuals.title_fg_color.a,
                card.visuals.title_font_size,
                card.visuals.use_default_body_bg,
                card.visuals.body_bg_color.r,
                card.visuals.body_bg_color.g,
                card.visuals.body_bg_color.b,
                card.visuals.body_bg_color.a,
                card.visuals.use_default_days_fg,
                card.visuals.days_fg_color.r,
                card.visuals.days_fg_color.g,
                card.visuals.days_fg_color.b,
                card.visuals.days_fg_color.a,
                card.visuals.days_font_size,
                card.auto_dismiss.enabled,
                card.auto_dismiss.on_event_start,
                card.auto_dismiss.on_event_end,
                card.auto_dismiss.delay_seconds,
                card.last_computed_days,
                last_warning_str,
                last_notif_str,
            ],
        ).context("Failed to update countdown card")?;

        Ok(rows > 0)
    }

    /// Delete a countdown card by ID
    pub fn delete_card(&self, id: CountdownCardId) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM countdown_cards WHERE id = ?", [id.0 as i64])
            .context("Failed to delete countdown card")?;
        Ok(rows > 0)
    }

    /// Delete all cards for a given event ID
    /// Note: This is redundant with ON DELETE CASCADE, but useful for explicit cleanup
    #[allow(dead_code)]
    pub fn delete_cards_for_event(&self, event_id: i64) -> Result<usize> {
        let rows = self
            .conn
            .execute(
                "DELETE FROM countdown_cards WHERE event_id = ?",
                [event_id],
            )
            .context("Failed to delete countdown cards for event")?;
        Ok(rows)
    }

    // ========== Global Settings Operations ==========

    /// Get global countdown settings
    pub fn get_global_settings(&self) -> Result<CountdownGlobalSettings> {
        let mut stmt = self.conn.prepare(
            "SELECT next_card_id,
                    app_window_x, app_window_y, app_window_width, app_window_height,
                    default_title_bg_r, default_title_bg_g, default_title_bg_b, default_title_bg_a,
                    default_title_fg_r, default_title_fg_g, default_title_fg_b, default_title_fg_a,
                    default_title_font_size,
                    default_body_bg_r, default_body_bg_g, default_body_bg_b, default_body_bg_a,
                    default_days_fg_r, default_days_fg_g, default_days_fg_b, default_days_fg_a,
                    default_days_font_size,
                    notifications_enabled, use_visual_warnings, use_system_notifications,
                    approaching_hours, imminent_hours, critical_minutes,
                    auto_dismiss_enabled, auto_dismiss_on_event_start, auto_dismiss_on_event_end,
                    auto_dismiss_delay_seconds
             FROM countdown_settings WHERE id = 1",
        )?;

        let settings = stmt
            .query_row([], |row| {
                let app_window_x: Option<f32> = row.get(1)?;
                let app_window_y: Option<f32> = row.get(2)?;
                let app_window_width: Option<f32> = row.get(3)?;
                let app_window_height: Option<f32> = row.get(4)?;

                let app_window_geometry =
                    if app_window_x.is_some() && app_window_y.is_some() {
                        Some(CountdownCardGeometry {
                            x: app_window_x.unwrap_or(0.0),
                            y: app_window_y.unwrap_or(0.0),
                            width: app_window_width.unwrap_or(800.0),
                            height: app_window_height.unwrap_or(600.0),
                        })
                    } else {
                        None
                    };

                Ok(CountdownGlobalSettings {
                    next_card_id: row.get::<_, i64>(0)? as u64,
                    app_window_geometry,
                    visual_defaults: CountdownCardVisuals {
                        accent_color: None,
                        always_on_top: false,
                        compact_mode: false,
                        use_default_title_bg: true,
                        title_bg_color: RgbaColor::new(
                            row.get(5)?,
                            row.get(6)?,
                            row.get(7)?,
                            row.get(8)?,
                        ),
                        use_default_title_fg: true,
                        title_fg_color: RgbaColor::new(
                            row.get(9)?,
                            row.get(10)?,
                            row.get(11)?,
                            row.get(12)?,
                        ),
                        title_font_size: row.get(13)?,
                        use_default_body_bg: true,
                        body_bg_color: RgbaColor::new(
                            row.get(14)?,
                            row.get(15)?,
                            row.get(16)?,
                            row.get(17)?,
                        ),
                        use_default_days_fg: true,
                        days_fg_color: RgbaColor::new(
                            row.get(18)?,
                            row.get(19)?,
                            row.get(20)?,
                            row.get(21)?,
                        ),
                        days_font_size: row.get(22)?,
                    },
                    notification_config: CountdownNotificationConfig {
                        enabled: row.get(23)?,
                        use_visual_warnings: row.get(24)?,
                        use_system_notifications: row.get(25)?,
                        warning_thresholds: WarningThresholds {
                            approaching_hours: row.get(26)?,
                            imminent_hours: row.get(27)?,
                            critical_minutes: row.get(28)?,
                        },
                    },
                    auto_dismiss_defaults: CountdownAutoDismissConfig {
                        enabled: row.get(29)?,
                        on_event_start: row.get(30)?,
                        on_event_end: row.get(31)?,
                        delay_seconds: row.get(32)?,
                    },
                })
            })
            .optional()
            .context("Failed to fetch countdown settings")?;

        Ok(settings.unwrap_or_default())
    }

    /// Update global countdown settings
    pub fn update_global_settings(&self, settings: &CountdownGlobalSettings) -> Result<()> {
        let (app_x, app_y, app_w, app_h) = settings
            .app_window_geometry
            .map(|g| (Some(g.x), Some(g.y), Some(g.width), Some(g.height)))
            .unwrap_or((None, None, None, None));

        self.conn.execute(
            "UPDATE countdown_settings SET
                next_card_id = ?1,
                app_window_x = ?2, app_window_y = ?3, app_window_width = ?4, app_window_height = ?5,
                default_title_bg_r = ?6, default_title_bg_g = ?7, default_title_bg_b = ?8, default_title_bg_a = ?9,
                default_title_fg_r = ?10, default_title_fg_g = ?11, default_title_fg_b = ?12, default_title_fg_a = ?13,
                default_title_font_size = ?14,
                default_body_bg_r = ?15, default_body_bg_g = ?16, default_body_bg_b = ?17, default_body_bg_a = ?18,
                default_days_fg_r = ?19, default_days_fg_g = ?20, default_days_fg_b = ?21, default_days_fg_a = ?22,
                default_days_font_size = ?23,
                notifications_enabled = ?24, use_visual_warnings = ?25, use_system_notifications = ?26,
                approaching_hours = ?27, imminent_hours = ?28, critical_minutes = ?29,
                auto_dismiss_enabled = ?30, auto_dismiss_on_event_start = ?31, auto_dismiss_on_event_end = ?32,
                auto_dismiss_delay_seconds = ?33,
                updated_at = CURRENT_TIMESTAMP
             WHERE id = 1",
            params![
                settings.next_card_id as i64,
                app_x,
                app_y,
                app_w,
                app_h,
                settings.visual_defaults.title_bg_color.r,
                settings.visual_defaults.title_bg_color.g,
                settings.visual_defaults.title_bg_color.b,
                settings.visual_defaults.title_bg_color.a,
                settings.visual_defaults.title_fg_color.r,
                settings.visual_defaults.title_fg_color.g,
                settings.visual_defaults.title_fg_color.b,
                settings.visual_defaults.title_fg_color.a,
                settings.visual_defaults.title_font_size,
                settings.visual_defaults.body_bg_color.r,
                settings.visual_defaults.body_bg_color.g,
                settings.visual_defaults.body_bg_color.b,
                settings.visual_defaults.body_bg_color.a,
                settings.visual_defaults.days_fg_color.r,
                settings.visual_defaults.days_fg_color.g,
                settings.visual_defaults.days_fg_color.b,
                settings.visual_defaults.days_fg_color.a,
                settings.visual_defaults.days_font_size,
                settings.notification_config.enabled,
                settings.notification_config.use_visual_warnings,
                settings.notification_config.use_system_notifications,
                settings.notification_config.warning_thresholds.approaching_hours,
                settings.notification_config.warning_thresholds.imminent_hours,
                settings.notification_config.warning_thresholds.critical_minutes,
                settings.auto_dismiss_defaults.enabled,
                settings.auto_dismiss_defaults.on_event_start,
                settings.auto_dismiss_defaults.on_event_end,
                settings.auto_dismiss_defaults.delay_seconds,
            ],
        )
        .context("Failed to update countdown settings")?;

        Ok(())
    }

    /// Update just the next_card_id
    #[allow(dead_code)]
    pub fn update_next_card_id(&self, next_id: u64) -> Result<()> {
        self.conn
            .execute(
                "UPDATE countdown_settings SET next_card_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 1",
                [next_id as i64],
            )
            .context("Failed to update next_card_id")?;
        Ok(())
    }
}

// ========== Helper Functions ==========

fn row_to_card_state(row: &Row<'_>) -> rusqlite::Result<CountdownCardState> {
    let id: i64 = row.get(0)?;
    let event_id: Option<i64> = row.get(1)?;
    let event_title: String = row.get(2)?;
    let start_at_str: String = row.get(3)?;
    let title_override: Option<String> = row.get(4)?;
    let auto_title_override: bool = row.get(5)?;
    let comment: Option<String> = row.get(6)?;
    let event_color_str: Option<String> = row.get(7)?;

    let start_at = DateTime::parse_from_rfc3339(&start_at_str)
        .map(|dt| dt.with_timezone(&Local))
        .unwrap_or_else(|_| Local::now());

    let event_color = event_color_str.and_then(|s| {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() == 4 {
            Some(RgbaColor::new(
                parts[0].parse().unwrap_or(0),
                parts[1].parse().unwrap_or(0),
                parts[2].parse().unwrap_or(0),
                parts[3].parse().unwrap_or(255),
            ))
        } else {
            None
        }
    });

    let geometry = CountdownCardGeometry {
        x: row.get(8)?,
        y: row.get(9)?,
        width: row.get(10)?,
        height: row.get(11)?,
    };

    let visuals = CountdownCardVisuals {
        accent_color: row.get(12)?,
        always_on_top: row.get(13)?,
        compact_mode: row.get(14)?,
        use_default_title_bg: row.get(15)?,
        title_bg_color: RgbaColor::new(row.get(16)?, row.get(17)?, row.get(18)?, row.get(19)?),
        use_default_title_fg: row.get(20)?,
        title_fg_color: RgbaColor::new(row.get(21)?, row.get(22)?, row.get(23)?, row.get(24)?),
        title_font_size: row.get(25)?,
        use_default_body_bg: row.get(26)?,
        body_bg_color: RgbaColor::new(row.get(27)?, row.get(28)?, row.get(29)?, row.get(30)?),
        use_default_days_fg: row.get(31)?,
        days_fg_color: RgbaColor::new(row.get(32)?, row.get(33)?, row.get(34)?, row.get(35)?),
        days_font_size: row.get(36)?,
    };

    let auto_dismiss = CountdownAutoDismissConfig {
        enabled: row.get(37)?,
        on_event_start: row.get(38)?,
        on_event_end: row.get(39)?,
        delay_seconds: row.get(40)?,
    };

    let last_computed_days: Option<i64> = row.get(41)?;
    let last_warning_str: Option<String> = row.get(42)?;
    let last_notification_str: Option<String> = row.get(43)?;

    let last_warning_state = last_warning_str.and_then(|s| string_to_warning_state(&s));
    let last_notification_time = last_notification_str.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Local))
            .ok()
    });

    Ok(CountdownCardState {
        id: CountdownCardId(id as u64),
        event_id,
        event_title,
        start_at,
        title_override,
        auto_title_override,
        geometry,
        visuals,
        last_computed_days,
        comment,
        event_color,
        last_warning_state,
        last_notification_time,
        auto_dismiss,
    })
}

fn warning_state_to_string(state: CountdownWarningState) -> String {
    match state {
        CountdownWarningState::Normal => "Normal".to_string(),
        CountdownWarningState::Approaching => "Approaching".to_string(),
        CountdownWarningState::Imminent => "Imminent".to_string(),
        CountdownWarningState::Critical => "Critical".to_string(),
        CountdownWarningState::Starting => "Starting".to_string(),
    }
}

fn string_to_warning_state(s: &str) -> Option<CountdownWarningState> {
    match s {
        "Normal" => Some(CountdownWarningState::Normal),
        "Approaching" => Some(CountdownWarningState::Approaching),
        "Imminent" => Some(CountdownWarningState::Imminent),
        "Critical" => Some(CountdownWarningState::Critical),
        "Starting" => Some(CountdownWarningState::Starting),
        _ => None,
    }
}
