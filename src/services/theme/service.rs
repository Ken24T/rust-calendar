use crate::services::database::Database;
use crate::ui_egui::theme::CalendarTheme;
use anyhow::{anyhow, Result};

use super::mapper::row_to_theme;

pub struct ThemeService<'a> {
    db: &'a Database,
}

impl<'a> ThemeService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get all available theme names (including built-in Light and Dark)
    pub fn list_themes(&self) -> Result<Vec<String>> {
        let conn = self.db.connection();
        let mut stmt = conn.prepare("SELECT name FROM custom_themes ORDER BY name")?;
        let custom_themes = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        let mut all_themes = vec!["Light".to_string(), "Dark".to_string()];
        for theme in custom_themes {
            if theme != "Light" && theme != "Dark" {
                all_themes.push(theme);
            }
        }

        Ok(all_themes)
    }

    /// Get a theme by name (handles built-in and custom themes)
    pub fn get_theme(&self, name: &str) -> Result<CalendarTheme> {
        match name.to_lowercase().as_str() {
            "light" => return Ok(CalendarTheme::light()),
            "dark" => return Ok(CalendarTheme::dark()),
            _ => {}
        }

        let conn = self.db.connection();
        let theme = conn.query_row(
            "SELECT name, is_dark, app_background, calendar_background, weekend_background,
                    today_background, today_border, day_background, day_border,
                    text_primary, text_secondary
             FROM custom_themes WHERE name = ?1",
            [name],
            |row| Ok(row_to_theme(row)?),
        )?;
        Ok(theme)
    }

    /// Save or update a theme
    pub fn save_theme(&self, theme: &CalendarTheme, name: &str) -> Result<()> {
        let conn = self.db.connection();
        let is_dark = theme.is_dark;

        conn.execute(
            "INSERT OR REPLACE INTO custom_themes \
             (name, is_dark, app_background, calendar_background, weekend_background,\
              today_background, today_border, day_background, day_border,\
              text_primary, text_secondary, updated_at)\
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, CURRENT_TIMESTAMP)",
            (
                name,
                is_dark as i32,
                CalendarTheme::color_to_string(theme.app_background),
                CalendarTheme::color_to_string(theme.calendar_background),
                CalendarTheme::color_to_string(theme.weekend_background),
                CalendarTheme::color_to_string(theme.today_background),
                CalendarTheme::color_to_string(theme.today_border),
                CalendarTheme::color_to_string(theme.day_background),
                CalendarTheme::color_to_string(theme.day_border),
                CalendarTheme::color_to_string(theme.text_primary),
                CalendarTheme::color_to_string(theme.text_secondary),
            ),
        )?;

        Ok(())
    }

    /// Delete a theme by name (cannot delete built-in Light/Dark themes)
    pub fn delete_theme(&self, name: &str) -> Result<()> {
        if name == "Light" || name == "Dark" {
            return Err(anyhow!("Cannot delete built-in themes"));
        }

        let conn = self.db.connection();
        conn.execute("DELETE FROM custom_themes WHERE name = ?1", [name])?;
        Ok(())
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
    fn test_list_themes() {
        let db = setup_test_db();
        let service = ThemeService::new(&db);

        let themes = service.list_themes().unwrap();
        assert!(themes.contains(&"Light".to_string()));
        assert!(themes.contains(&"Dark".to_string()));
    }

    #[test]
    fn test_get_theme() {
        let db = setup_test_db();
        let service = ThemeService::new(&db);

        let theme = service.get_theme("Light").unwrap();
        assert!(!theme.is_dark);
    }

    #[test]
    fn test_save_custom_theme() {
        let db = setup_test_db();
        let service = ThemeService::new(&db);

        let custom_theme = CalendarTheme::light();
        let result = service.save_theme(&custom_theme, "MyCustomTheme");
        assert!(result.is_ok());

        let themes = service.list_themes().unwrap();
        assert!(themes.contains(&"MyCustomTheme".to_string()));
    }

    #[test]
    fn test_delete_custom_theme() {
        let db = setup_test_db();
        let service = ThemeService::new(&db);

        let custom_theme = CalendarTheme::light();
        service.save_theme(&custom_theme, "TestTheme").unwrap();

        let result = service.delete_theme("TestTheme");
        assert!(result.is_ok());

        let themes = service.list_themes().unwrap();
        assert!(!themes.contains(&"TestTheme".to_string()));
    }

    #[test]
    fn test_cannot_delete_builtin_themes() {
        let db = setup_test_db();
        let service = ThemeService::new(&db);

        let result = service.delete_theme("Light");
        assert!(result.is_err());

        let result = service.delete_theme("Dark");
        assert!(result.is_err());
    }
}
