// Theme service for managing custom themes
// Handles CRUD operations for custom theme configurations

use anyhow::{Context, Result};
use crate::services::database::Database;
use crate::ui::theme::CalendarTheme;
use iced::Color;

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
        all_themes.extend(custom_themes);
        
        Ok(all_themes)
    }
    
    /// Get a theme by name (handles built-in and custom themes)
    pub fn get_theme(&self, name: &str) -> Result<CalendarTheme> {
        // Handle built-in themes
        match name {
            "Light" => return Ok(CalendarTheme::light()),
            "Dark" => return Ok(CalendarTheme::dark()),
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
        
        let is_dark = matches!(theme.base, iced::Theme::Dark);
        
        conn.execute(
            "INSERT OR REPLACE INTO custom_themes 
             (name, is_dark, app_background, calendar_background, weekend_background,
              today_background, today_border, day_background, day_border,
              text_primary, text_secondary, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, CURRENT_TIMESTAMP)",
            (
                name,
                is_dark as i32,
                Self::color_to_string(theme.app_background),
                Self::color_to_string(theme.calendar_background),
                Self::color_to_string(theme.weekend_background),
                Self::color_to_string(theme.today_background),
                Self::color_to_string(theme.today_border),
                Self::color_to_string(theme.day_background),
                Self::color_to_string(theme.day_border),
                Self::color_to_string(theme.text_primary),
                Self::color_to_string(theme.text_secondary),
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
        let base = if is_dark != 0 {
            iced::Theme::Dark
        } else {
            iced::Theme::Light
        };
        
        Ok(CalendarTheme {
            base,
            app_background: Self::string_to_color(&row.get::<_, String>(2)?),
            calendar_background: Self::string_to_color(&row.get::<_, String>(3)?),
            weekend_background: Self::string_to_color(&row.get::<_, String>(4)?),
            today_background: Self::string_to_color(&row.get::<_, String>(5)?),
            today_border: Self::string_to_color(&row.get::<_, String>(6)?),
            day_background: Self::string_to_color(&row.get::<_, String>(7)?),
            day_border: Self::string_to_color(&row.get::<_, String>(8)?),
            text_primary: Self::string_to_color(&row.get::<_, String>(9)?),
            text_secondary: Self::string_to_color(&row.get::<_, String>(10)?),
        })
    }
    
    /// Convert Color to string format "r,g,b"
    fn color_to_string(color: Color) -> String {
        format!("{},{},{}", color.r, color.g, color.b)
    }
    
    /// Convert string format "r,g,b" to Color
    fn string_to_color(s: &str) -> Color {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() == 3 {
            let r: f32 = parts[0].parse().unwrap_or(0.0);
            let g: f32 = parts[1].parse().unwrap_or(0.0);
            let b: f32 = parts[2].parse().unwrap_or(0.0);
            Color::from_rgb(r, g, b)
        } else {
            Color::BLACK
        }
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
        assert_eq!(theme.base, iced::Theme::Light);
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
