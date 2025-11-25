use crate::services::database::Database;
use crate::ui_egui::theme::{CalendarTheme, ThemePreset};
use anyhow::{anyhow, Result};
use std::path::Path;

use super::mapper::row_to_theme;

pub struct ThemeService<'a> {
    db: &'a Database,
}

impl<'a> ThemeService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get all available theme names (including built-in presets and custom themes)
    pub fn list_themes(&self) -> Result<Vec<String>> {
        let conn = self.db.connection();
        let mut stmt = conn.prepare("SELECT name FROM custom_themes ORDER BY name")?;
        let custom_themes = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        // Start with all preset theme names
        let mut all_themes: Vec<String> = ThemePreset::all()
            .iter()
            .map(|p| p.name().to_string())
            .collect();
        
        // Add custom themes that aren't preset names
        for theme in custom_themes {
            if !CalendarTheme::is_builtin(&theme) {
                all_themes.push(theme);
            }
        }

        Ok(all_themes)
    }

    /// Get only custom theme names (excluding presets)
    #[allow(dead_code)]
    pub fn list_custom_themes(&self) -> Result<Vec<String>> {
        let conn = self.db.connection();
        let mut stmt = conn.prepare("SELECT name FROM custom_themes ORDER BY name")?;
        let themes = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .filter(|name| !CalendarTheme::is_builtin(name))
            .collect();

        Ok(themes)
    }

    /// Get a theme by name (handles built-in presets and custom themes)
    pub fn get_theme(&self, name: &str) -> Result<CalendarTheme> {
        // Check if it's a preset theme first
        if let Some(theme) = CalendarTheme::from_preset_name(name) {
            return Ok(theme);
        }

        // Otherwise look in the database
        let conn = self.db.connection();
        let theme = conn.query_row(
            "SELECT name, is_dark, app_background, calendar_background, weekend_background,
                    today_background, today_border, day_background, day_border,
                    text_primary, text_secondary, header_background, header_text
             FROM custom_themes WHERE name = ?1",
            [name],
            |row| Ok(row_to_theme(row)?),
        )?;
        Ok(theme)
    }

    /// Get a theme with its preview colors for display in dialogs
    #[allow(dead_code)]
    pub fn get_theme_with_preview(&self, name: &str) -> Result<(CalendarTheme, [egui::Color32; 4])> {
        let theme = self.get_theme(name)?;
        let preview = theme.preview_colors();
        Ok((theme, preview))
    }

    /// Save or update a theme
    pub fn save_theme(&self, theme: &CalendarTheme, name: &str) -> Result<()> {
        let conn = self.db.connection();
        let is_dark = theme.is_dark;

        conn.execute(
            "INSERT OR REPLACE INTO custom_themes \
             (name, is_dark, app_background, calendar_background, weekend_background,\
              today_background, today_border, day_background, day_border,\
              text_primary, text_secondary, header_background, header_text, updated_at)\
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, CURRENT_TIMESTAMP)",
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
                CalendarTheme::color_to_string(theme.header_background),
                CalendarTheme::color_to_string(theme.header_text),
            ),
        )?;

        Ok(())
    }

    /// Delete a theme by name (cannot delete built-in preset themes)
    pub fn delete_theme(&self, name: &str) -> Result<()> {
        if CalendarTheme::is_builtin(name) {
            return Err(anyhow!("Cannot delete built-in themes"));
        }

        let conn = self.db.connection();
        conn.execute("DELETE FROM custom_themes WHERE name = ?1", [name])?;
        Ok(())
    }

    /// Export a theme to a TOML file
    pub fn export_theme(&self, name: &str, path: &Path) -> Result<()> {
        let theme = self.get_theme(name)?;
        let toml = theme.to_toml();
        std::fs::write(path, toml)?;
        Ok(())
    }

    /// Import a theme from a TOML file
    pub fn import_theme(&self, path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(path)?;
        let theme = CalendarTheme::from_toml(&content)
            .map_err(|e| anyhow!("Failed to parse theme: {}", e))?;
        
        let name = theme.name.clone();
        
        // Don't allow importing with a preset name
        if CalendarTheme::is_builtin(&name) {
            return Err(anyhow!("Cannot import a theme with a built-in theme name. Please rename the theme."));
        }
        
        self.save_theme(&theme, &name)?;
        Ok(name)
    }

    /// Duplicate an existing theme with a new name
    pub fn duplicate_theme(&self, source_name: &str, new_name: &str) -> Result<()> {
        if CalendarTheme::is_builtin(new_name) {
            return Err(anyhow!("Cannot use a built-in theme name"));
        }
        
        let mut theme = self.get_theme(source_name)?;
        theme.name = new_name.to_string();
        self.save_theme(&theme, new_name)?;
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

        let result = service.delete_theme("Nord");
        assert!(result.is_err());
    }

    #[test]
    fn test_preset_themes_available() {
        let db = setup_test_db();
        let service = ThemeService::new(&db);

        // All presets should be retrievable
        let theme = service.get_theme("Nord").unwrap();
        assert!(theme.is_dark);
        
        let theme = service.get_theme("Solarized Light").unwrap();
        assert!(!theme.is_dark);
    }

    #[test]
    fn test_duplicate_theme() {
        let db = setup_test_db();
        let service = ThemeService::new(&db);

        service.duplicate_theme("Nord", "My Nord").unwrap();
        
        let themes = service.list_themes().unwrap();
        assert!(themes.contains(&"My Nord".to_string()));
        
        let theme = service.get_theme("My Nord").unwrap();
        assert!(theme.is_dark);
    }
}
