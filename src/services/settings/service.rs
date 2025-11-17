use crate::models::settings::Settings;
use crate::services::database::Database;
use anyhow::{anyhow, Context, Result};

use super::mapper::row_to_settings;

pub struct SettingsService<'a> {
    db: &'a Database,
}

impl<'a> SettingsService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get the current settings
    pub fn get(&self) -> Result<Settings> {
        let conn = self.db.connection();

        let settings = conn
            .query_row(
                "SELECT id, theme, first_day_of_week, time_format, date_format,
                    show_my_day, my_day_position_right, show_ribbon, current_view,
                    default_event_duration, first_day_of_work_week, last_day_of_work_week,
                    default_event_start_time, default_card_width, default_card_height
             FROM settings WHERE id = 1",
                [],
                |row| Ok(row_to_settings(row)?),
            )
            .context("Failed to load settings")?;

        Ok(settings)
    }

    /// Update settings
    pub fn update(&self, settings: &Settings) -> Result<()> {
        settings
            .validate_without_theme()
            .map_err(|e| anyhow!("Invalid settings: {}", e))?;

        let conn = self.db.connection();

        conn.execute(
            "UPDATE settings \
             SET theme = ?1, \
                 first_day_of_week = ?2, \
                 time_format = ?3, \
                 date_format = ?4, \
                 show_my_day = ?5, \
                 my_day_position_right = ?6, \
                 show_ribbon = ?7, \
                 current_view = ?8, \
                 default_event_duration = ?9, \
                 first_day_of_work_week = ?10, \
                 last_day_of_work_week = ?11, \
                 default_event_start_time = ?12, \
                 default_card_width = ?13, \
                 default_card_height = ?14, \
                 updated_at = CURRENT_TIMESTAMP \
             WHERE id = 1",
            (
                &settings.theme,
                settings.first_day_of_week,
                &settings.time_format,
                &settings.date_format,
                settings.show_my_day as i32,
                settings.my_day_position_right as i32,
                settings.show_ribbon as i32,
                &settings.current_view,
                settings.default_event_duration,
                settings.first_day_of_work_week,
                settings.last_day_of_work_week,
                &settings.default_event_start_time,
                settings.default_card_width,
                settings.default_card_height,
            ),
        )
        .context("Failed to update settings")?;

        Ok(())
    }

    /// Reset settings to defaults
    #[allow(dead_code)]
    pub fn reset(&self) -> Result<()> {
        let default_settings = Settings::default();
        self.update(&default_settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::Database;

    fn setup_test_db() -> Database {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        db
    }

    #[test]
    fn test_get_default_settings() {
        let db = setup_test_db();
        let service = SettingsService::new(&db);

        let settings = service.get().unwrap();
        assert_eq!(settings.theme, "light");
        assert_eq!(settings.first_day_of_week, 0);
    }

    #[test]
    fn test_update_settings() {
        let db = setup_test_db();
        let service = SettingsService::new(&db);

        let mut settings = service.get().unwrap();
        settings.theme = "dark".to_string();
        settings.first_day_of_week = 1;
        settings.show_my_day = true;

        let result = service.update(&settings);
        assert!(result.is_ok());

        let updated = service.get().unwrap();
        assert_eq!(updated.theme, "dark");
        assert_eq!(updated.first_day_of_week, 1);
        assert!(updated.show_my_day);
    }

    #[test]
    fn test_update_invalid_settings() {
        let db = setup_test_db();
        let service = SettingsService::new(&db);

        let mut settings = service.get().unwrap();
        settings.time_format = "invalid".to_string();

        let result = service.update(&settings);
        assert!(result.is_err());
    }

    #[test]
    fn test_reset_settings() {
        let db = setup_test_db();
        let service = SettingsService::new(&db);

        let mut settings = service.get().unwrap();
        settings.theme = "dark".to_string();
        settings.first_day_of_week = 1;
        service.update(&settings).unwrap();

        let result = service.reset();
        assert!(result.is_ok());

        let reset_settings = service.get().unwrap();
        let defaults = Settings::default();
        assert_eq!(reset_settings.theme, defaults.theme);
        assert_eq!(reset_settings.first_day_of_week, defaults.first_day_of_week);
    }

    #[test]
    fn test_update_all_boolean_fields() {
        let db = setup_test_db();
        let service = SettingsService::new(&db);

        let mut settings = service.get().unwrap();
        settings.show_my_day = true;
        settings.show_ribbon = true;

        service.update(&settings).unwrap();

        let updated = service.get().unwrap();
        assert!(updated.show_my_day);
        assert!(updated.show_ribbon);
    }

    #[test]
    fn test_update_current_view() {
        let db = setup_test_db();
        let service = SettingsService::new(&db);

        let mut settings = service.get().unwrap();
        settings.current_view = "Week".to_string();

        service.update(&settings).unwrap();

        let updated = service.get().unwrap();
        assert_eq!(updated.current_view, "Week");
    }
}
