//! Global settings operations for the countdown repository.
//!
//! Extracted from `repository.rs` — handles get/update of countdown global settings.

use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension};

use super::models::{
    CountdownAutoDismissConfig, CountdownCardGeometry, CountdownCardId, CountdownCardVisuals,
    CountdownDisplayMode, CountdownNotificationConfig, RgbaColor,
    WarningThresholds,
};
use super::repository::CountdownRepository;

/// Global countdown settings stored in the database
#[derive(Debug, Clone)]
pub struct CountdownGlobalSettings {
    pub next_card_id: u64,
    pub app_window_geometry: Option<CountdownCardGeometry>,
    pub visual_defaults: CountdownCardVisuals,
    pub notification_config: CountdownNotificationConfig,
    pub auto_dismiss_defaults: CountdownAutoDismissConfig,
    pub display_mode: CountdownDisplayMode,
    pub container_geometry: Option<CountdownCardGeometry>,
    pub card_order: Vec<CountdownCardId>,
}

impl Default for CountdownGlobalSettings {
    fn default() -> Self {
        Self {
            next_card_id: 1,
            app_window_geometry: None,
            visual_defaults: CountdownCardVisuals::default(),
            notification_config: CountdownNotificationConfig::default(),
            auto_dismiss_defaults: CountdownAutoDismissConfig::default(),
            display_mode: CountdownDisplayMode::default(),
            container_geometry: None,
            card_order: Vec::new(),
        }
    }
}

impl<'a> CountdownRepository<'a> {
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
                    auto_dismiss_delay_seconds,
                    display_mode,
                    container_geometry_x, container_geometry_y, container_geometry_width, container_geometry_height,
                    card_order
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

                // Parse container geometry (indices 34-37)
                let container_x: Option<f32> = row.get(34)?;
                let container_y: Option<f32> = row.get(35)?;
                let container_width: Option<f32> = row.get(36)?;
                let container_height: Option<f32> = row.get(37)?;

                let container_geometry =
                    if container_x.is_some() && container_y.is_some() {
                        Some(CountdownCardGeometry {
                            x: container_x.unwrap_or(0.0),
                            y: container_y.unwrap_or(0.0),
                            width: container_width.unwrap_or(400.0),
                            height: container_height.unwrap_or(300.0),
                        })
                    } else {
                        None
                    };

                // Parse display_mode (index 33)
                let display_mode_str: Option<String> = row.get(33)?;
                let display_mode = display_mode_str
                    .as_deref()
                    .map(|s| match s {
                        "IndividualWindows" => CountdownDisplayMode::IndividualWindows,
                        "Container" => CountdownDisplayMode::Container,
                        "CategoryContainers" => CountdownDisplayMode::CategoryContainers,
                        _ => CountdownDisplayMode::default(),
                    })
                    .unwrap_or_default();

                // Parse card_order (index 38)
                let card_order_str: Option<String> = row.get(38)?;
                let card_order: Vec<CountdownCardId> = card_order_str
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        s.split(',')
                            .filter_map(|id| id.trim().parse::<u64>().ok())
                            .map(CountdownCardId)
                            .collect()
                    })
                    .unwrap_or_default();

                Ok(CountdownGlobalSettings {
                    next_card_id: row.get::<_, i64>(0)? as u64,
                    app_window_geometry,
                    visual_defaults: CountdownCardVisuals {
                        accent_color: None,
                        always_on_top: false,
                        use_default_title_bg: false,
                        title_bg_color: RgbaColor::new(
                            row.get(5)?,
                            row.get(6)?,
                            row.get(7)?,
                            row.get(8)?,
                        ),
                        use_default_title_fg: false,
                        title_fg_color: RgbaColor::new(
                            row.get(9)?,
                            row.get(10)?,
                            row.get(11)?,
                            row.get(12)?,
                        ),
                        title_font_size: row.get(13)?,
                        use_default_body_bg: false,
                        body_bg_color: RgbaColor::new(
                            row.get(14)?,
                            row.get(15)?,
                            row.get(16)?,
                            row.get(17)?,
                        ),
                        use_default_days_fg: false,
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
                    display_mode,
                    container_geometry,
                    card_order,
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

        let (cont_x, cont_y, cont_w, cont_h) = settings
            .container_geometry
            .map(|g| (Some(g.x), Some(g.y), Some(g.width), Some(g.height)))
            .unwrap_or((None, None, None, None));

        let display_mode_str = match settings.display_mode {
            CountdownDisplayMode::IndividualWindows => "IndividualWindows",
            CountdownDisplayMode::Container => "Container",
            CountdownDisplayMode::CategoryContainers => "CategoryContainers",
        };

        let card_order_str = settings
            .card_order
            .iter()
            .map(|id| id.0.to_string())
            .collect::<Vec<_>>()
            .join(",");

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
                display_mode = ?34,
                container_geometry_x = ?35, container_geometry_y = ?36, container_geometry_width = ?37, container_geometry_height = ?38,
                card_order = ?39,
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
                display_mode_str,
                cont_x,
                cont_y,
                cont_w,
                cont_h,
                card_order_str,
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
