// Theme service for managing custom themes
// Handles CRUD operations for custom theme configurations

use anyhow::Result;
use crate::services::database::Database;
use crate::ui_egui::theme::CalendarTheme;
use egui::Color32;

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
        let custom_themes = stmt.query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        
        // Always include built-in themes at the start
        let mut all_themes = vec!["Light".to_string(), "Dark".to_string()];
        
        // Add custom themes, but skip if they're named "Light" or "Dark" (shouldn't happen, but be safe)
        for theme in custom_themes {
            if theme != "Light" && theme != "Dark" {
                all_themes.push(theme);
            }
        }
        
        Ok(all_themes)
    }
    
    /// Get a theme by name (handles built-in and custom themes)
    pub fn get_theme(&self, name: &str) -> Result<CalendarTheme> {
        // Handle built-in themes (case-insensitive)
        match name.to_lowercase().as_str() {
            "light" => return Ok(CalendarTheme::light()),
            "dark" => return Ok(CalendarTheme::dark()),
            _ => {}
        }
        
        // Load custom theme from database
        let conn = self.db.connection();
        let theme = conn.query_row(
            "SELECT name, is_dark, app_background, calendar_background, weekend_background,
                    today_background, today_border, day_background, day_border,
                    text_primary, text_secondary
             FROM custom_themes WHERE name = ?1",
            [name],
            |row| {
                Ok(Self::row_to_theme(row)?)
            },
        )?;
        Ok(theme)
    }
    
    /// Save or update a theme
    pub fn save_theme(&self, theme: &CalendarTheme, name: &str) -> Result<()> {
        let conn = self.db.connection();
        
        let is_dark = theme.is_dark;
        
        conn.execute(
            "INSERT OR REPLACE INTO custom_themes 
             (name, is_dark, app_background, calendar_background, weekend_background,
              today_background, today_border, day_background, day_border,
              text_primary, text_secondary, updated_at)
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
            return Err(anyhow::anyhow!("Cannot delete built-in themes"));
        }
        
        let conn = self.db.connection();
        conn.execute("DELETE FROM custom_themes WHERE name = ?1", [name])?;
        Ok(())
    }
    
    /// Convert a database row to CalendarTheme
    fn row_to_theme(row: &rusqlite::Row) -> Result<CalendarTheme, rusqlite::Error> {
        let is_dark: i32 = row.get(1)?;
        
        Ok(CalendarTheme {
            is_dark: is_dark != 0,
            app_background: CalendarTheme::string_to_color(&row.get::<_, String>(2)?).unwrap_or(Color32::BLACK),
            calendar_background: CalendarTheme::string_to_color(&row.get::<_, String>(3)?).unwrap_or(Color32::WHITE),
            weekend_background: CalendarTheme::string_to_color(&row.get::<_, String>(4)?).unwrap_or(Color32::LIGHT_GRAY),
            today_background: CalendarTheme::string_to_color(&row.get::<_, String>(5)?).unwrap_or(Color32::LIGHT_BLUE),
            today_border: CalendarTheme::string_to_color(&row.get::<_, String>(6)?).unwrap_or(Color32::BLUE),
            day_background: CalendarTheme::string_to_color(&row.get::<_, String>(7)?).unwrap_or(Color32::WHITE),
            day_border: CalendarTheme::string_to_color(&row.get::<_, String>(8)?).unwrap_or(Color32::GRAY),
            text_primary: CalendarTheme::string_to_color(&row.get::<_, String>(9)?).unwrap_or(Color32::BLACK),
            text_secondary: CalendarTheme::string_to_color(&row.get::<_, String>(10)?).unwrap_or(Color32::GRAY),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
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
        
        // Save a custom theme
        let custom_theme = CalendarTheme::light();
        service.save_theme(&custom_theme, "TestTheme").unwrap();
        
        // Delete it
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
