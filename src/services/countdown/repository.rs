//! Database repository for countdown cards.
//!
//! This module provides CRUD operations for countdown cards stored in SQLite.

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use rusqlite::{params, Connection, OptionalExtension, Row};

use super::models::{
    CountdownAutoDismissConfig, CountdownCardGeometry, CountdownCardId, CountdownCardState,
    CountdownCardVisuals, CountdownWarningState,
    RgbaColor,
};

pub use super::repository_settings::CountdownGlobalSettings;

/// Repository for countdown card database operations
pub struct CountdownRepository<'a> {
    pub(super) conn: &'a Connection,
}

impl<'a> CountdownRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ========== Card CRUD Operations ==========

    /// Get all countdown cards from the database
    pub fn get_all_cards(&self) -> Result<Vec<CountdownCardState>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_id, event_title, start_at, event_start, event_end,
                    title_override, auto_title_override,
                    comment, event_color,
                    geometry_x, geometry_y, geometry_width, geometry_height,
                    accent_color, always_on_top,
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
            .query_map([], row_to_card_state)?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to fetch countdown cards")?;

        Ok(cards)
    }

    /// Get a single card by ID
    #[allow(dead_code)]
    pub fn get_card(&self, id: CountdownCardId) -> Result<Option<CountdownCardState>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_id, event_title, start_at, event_start, event_end,
                    title_override, auto_title_override,
                    comment, event_color,
                    geometry_x, geometry_y, geometry_width, geometry_height,
                    accent_color, always_on_top,
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

        stmt.query_row([id.0 as i64], row_to_card_state)
            .optional()
            .context("Failed to fetch countdown card")
    }

    /// Get a card by its associated event ID
    #[allow(dead_code)]
    pub fn get_card_by_event_id(&self, event_id: i64) -> Result<Option<CountdownCardState>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, event_id, event_title, start_at, event_start, event_end,
                    title_override, auto_title_override,
                    comment, event_color,
                    geometry_x, geometry_y, geometry_width, geometry_height,
                    accent_color, always_on_top,
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

        stmt.query_row([event_id], row_to_card_state)
            .optional()
            .context("Failed to fetch countdown card by event_id")
    }

    /// Insert a new countdown card
    pub fn insert_card(&self, card: &CountdownCardState) -> Result<()> {
        let start_at_str = card.start_at.to_rfc3339();
        let event_start_str = card.event_start.map(|t| t.to_rfc3339());
        let event_end_str = card.event_end.map(|t| t.to_rfc3339());
        let event_color_str = card.event_color.map(|c| format!("{},{},{},{}", c.r, c.g, c.b, c.a));
        let last_warning_str = card.last_warning_state.map(warning_state_to_string);
        let last_notif_str = card.last_notification_time.map(|t| t.to_rfc3339());

        self.conn.execute(
            "INSERT INTO countdown_cards (
                id, event_id, event_title, start_at, event_start, event_end,
                title_override, auto_title_override,
                comment, event_color,
                geometry_x, geometry_y, geometry_width, geometry_height,
                accent_color, always_on_top,
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
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14,
                ?15, ?16,
                ?17, ?18, ?19, ?20, ?21,
                ?22, ?23, ?24, ?25, ?26,
                ?27,
                ?28, ?29, ?30, ?31, ?32,
                ?33, ?34, ?35, ?36, ?37,
                ?38,
                ?39, ?40, ?41, ?42,
                ?43, ?44, ?45
            )",
            params![
                card.id.0 as i64,
                card.event_id,
                card.event_title,
                start_at_str,
                event_start_str,
                event_end_str,
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
        let event_start_str = card.event_start.map(|t| t.to_rfc3339());
        let event_end_str = card.event_end.map(|t| t.to_rfc3339());
        let event_color_str = card.event_color.map(|c| format!("{},{},{},{}", c.r, c.g, c.b, c.a));
        let last_warning_str = card.last_warning_state.map(warning_state_to_string);
        let last_notif_str = card.last_notification_time.map(|t| t.to_rfc3339());

        let rows = self.conn.execute(
            "UPDATE countdown_cards SET
                event_id = ?2, event_title = ?3, start_at = ?4, event_start = ?5, event_end = ?6,
                title_override = ?7, auto_title_override = ?8, comment = ?9, event_color = ?10,
                geometry_x = ?11, geometry_y = ?12, geometry_width = ?13, geometry_height = ?14,
                accent_color = ?15, always_on_top = ?16,
                use_default_title_bg = ?17, title_bg_r = ?18, title_bg_g = ?19, title_bg_b = ?20, title_bg_a = ?21,
                use_default_title_fg = ?22, title_fg_r = ?23, title_fg_g = ?24, title_fg_b = ?25, title_fg_a = ?26,
                title_font_size = ?27,
                use_default_body_bg = ?28, body_bg_r = ?29, body_bg_g = ?30, body_bg_b = ?31, body_bg_a = ?32,
                use_default_days_fg = ?33, days_fg_r = ?34, days_fg_g = ?35, days_fg_b = ?36, days_fg_a = ?37,
                days_font_size = ?38,
                auto_dismiss_enabled = ?39, auto_dismiss_on_event_start = ?40, auto_dismiss_on_event_end = ?41,
                auto_dismiss_delay_seconds = ?42,
                last_computed_days = ?43, last_warning_state = ?44, last_notification_time = ?45,
                updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![
                card.id.0 as i64,
                card.event_id,
                card.event_title,
                start_at_str,
                event_start_str,
                event_end_str,
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
}

// ========== Helper Functions ==========

fn row_to_card_state(row: &Row<'_>) -> rusqlite::Result<CountdownCardState> {
    let id: i64 = row.get(0)?;
    let event_id: Option<i64> = row.get(1)?;
    let event_title: String = row.get(2)?;
    let start_at_str: String = row.get(3)?;
    let event_start_str: Option<String> = row.get(4)?;
    let event_end_str: Option<String> = row.get(5)?;
    let title_override: Option<String> = row.get(6)?;
    let auto_title_override: bool = row.get(7)?;
    let comment: Option<String> = row.get(8)?;
    let event_color_str: Option<String> = row.get(9)?;

    let start_at = DateTime::parse_from_rfc3339(&start_at_str)
        .map(|dt| dt.with_timezone(&Local))
        .unwrap_or_else(|_| Local::now());

    let event_start = event_start_str.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Local))
            .ok()
    });

    let event_end = event_end_str.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Local))
            .ok()
    });

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
        x: row.get(10)?,
        y: row.get(11)?,
        width: row.get(12)?,
        height: row.get(13)?,
    };

    let visuals = CountdownCardVisuals {
        accent_color: row.get(14)?,
        always_on_top: row.get(15)?,
        use_default_title_bg: row.get(16)?,
        title_bg_color: RgbaColor::new(row.get(17)?, row.get(18)?, row.get(19)?, row.get(20)?),
        use_default_title_fg: row.get(21)?,
        title_fg_color: RgbaColor::new(row.get(22)?, row.get(23)?, row.get(24)?, row.get(25)?),
        title_font_size: row.get(26)?,
        use_default_body_bg: row.get(27)?,
        body_bg_color: RgbaColor::new(row.get(28)?, row.get(29)?, row.get(30)?, row.get(31)?),
        use_default_days_fg: row.get(32)?,
        days_fg_color: RgbaColor::new(row.get(33)?, row.get(34)?, row.get(35)?, row.get(36)?),
        days_font_size: row.get(37)?,
    };

    let auto_dismiss = CountdownAutoDismissConfig {
        enabled: row.get(38)?,
        on_event_start: row.get(39)?,
        on_event_end: row.get(40)?,
        delay_seconds: row.get(41)?,
    };

    let last_computed_days: Option<i64> = row.get(42)?;
    let last_warning_str: Option<String> = row.get(43)?;
    let last_notification_str: Option<String> = row.get(44)?;

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
        event_start,
        event_end,
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
