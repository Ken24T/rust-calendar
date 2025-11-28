//! Category model for organizing events.
//!
//! Categories allow users to group events by type (Work, Personal, etc.)
//! with associated colors and optional icons for visual identification.

use serde::{Deserialize, Serialize};

/// A category for organizing events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    /// Unique identifier (database primary key)
    pub id: Option<i64>,
    /// Display name of the category (must be unique)
    pub name: String,
    /// Hex color code for the category (e.g., "#3B82F6")
    pub color: String,
    /// Optional emoji or icon for the category
    pub icon: Option<String>,
    /// Whether this is a system/default category (cannot be deleted)
    pub is_system: bool,
}

impl Category {
    /// Create a new category with the given name and color.
    pub fn new(name: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            color: color.into(),
            icon: None,
            is_system: false,
        }
    }

    /// Create a new category with an icon.
    pub fn with_icon(name: impl Into<String>, color: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            color: color.into(),
            icon: Some(icon.into()),
            is_system: false,
        }
    }

    /// Create a system category (cannot be deleted by user).
    pub fn system(name: impl Into<String>, color: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            color: color.into(),
            icon: Some(icon.into()),
            is_system: true,
        }
    }

    /// Validate the category data.
    pub fn validate(&self) -> Result<(), CategoryValidationError> {
        // Name validation
        let name = self.name.trim();
        if name.is_empty() {
            return Err(CategoryValidationError::EmptyName);
        }
        if name.len() > 50 {
            return Err(CategoryValidationError::NameTooLong);
        }

        // Color validation (must be valid hex)
        if !is_valid_hex_color(&self.color) {
            return Err(CategoryValidationError::InvalidColor);
        }

        // Icon validation (if present, must be reasonable length)
        if let Some(ref icon) = self.icon {
            if icon.len() > 10 {
                return Err(CategoryValidationError::IconTooLong);
            }
        }

        Ok(())
    }

    /// Get the display string (icon + name) for UI.
    pub fn display_name(&self) -> String {
        match &self.icon {
            Some(icon) => format!("{} {}", icon, self.name),
            None => self.name.clone(),
        }
    }
}

/// Validation errors for Category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryValidationError {
    EmptyName,
    NameTooLong,
    InvalidColor,
    IconTooLong,
}

impl std::fmt::Display for CategoryValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "Category name cannot be empty"),
            Self::NameTooLong => write!(f, "Category name must be 50 characters or less"),
            Self::InvalidColor => write!(f, "Invalid color format (use hex like #FF0000)"),
            Self::IconTooLong => write!(f, "Icon must be 10 characters or less"),
        }
    }
}

impl std::error::Error for CategoryValidationError {}

/// Check if a string is a valid hex color code.
fn is_valid_hex_color(color: &str) -> bool {
    let color = color.trim();
    if !color.starts_with('#') {
        return false;
    }
    let hex = &color[1..];
    // Accept 3, 6, or 8 character hex codes
    matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Default categories that ship with the application.
pub fn default_categories() -> Vec<Category> {
    vec![
        Category::system("Work", "#3B82F6", "💼"),
        Category::system("Personal", "#10B981", "🏠"),
        Category::system("Birthday", "#F59E0B", "🎂"),
        Category::system("Holiday", "#EF4444", "🎉"),
        Category::system("Meeting", "#8B5CF6", "👥"),
        Category::system("Deadline", "#DC2626", "⏰"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_new() {
        let cat = Category::new("Work", "#3B82F6");
        assert_eq!(cat.name, "Work");
        assert_eq!(cat.color, "#3B82F6");
        assert!(cat.icon.is_none());
        assert!(!cat.is_system);
        assert!(cat.id.is_none());
    }

    #[test]
    fn test_category_with_icon() {
        let cat = Category::with_icon("Work", "#3B82F6", "💼");
        assert_eq!(cat.name, "Work");
        assert_eq!(cat.icon, Some("💼".to_string()));
        assert!(!cat.is_system);
    }

    #[test]
    fn test_category_system() {
        let cat = Category::system("Work", "#3B82F6", "💼");
        assert!(cat.is_system);
    }

    #[test]
    fn test_display_name_with_icon() {
        let cat = Category::with_icon("Work", "#3B82F6", "💼");
        assert_eq!(cat.display_name(), "💼 Work");
    }

    #[test]
    fn test_display_name_without_icon() {
        let cat = Category::new("Work", "#3B82F6");
        assert_eq!(cat.display_name(), "Work");
    }

    #[test]
    fn test_validate_valid_category() {
        let cat = Category::new("Work", "#3B82F6");
        assert!(cat.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let cat = Category::new("", "#3B82F6");
        assert_eq!(cat.validate(), Err(CategoryValidationError::EmptyName));
    }

    #[test]
    fn test_validate_whitespace_name() {
        let cat = Category::new("   ", "#3B82F6");
        assert_eq!(cat.validate(), Err(CategoryValidationError::EmptyName));
    }

    #[test]
    fn test_validate_name_too_long() {
        let cat = Category::new("a".repeat(51), "#3B82F6");
        assert_eq!(cat.validate(), Err(CategoryValidationError::NameTooLong));
    }

    #[test]
    fn test_validate_invalid_color_no_hash() {
        let cat = Category::new("Work", "3B82F6");
        assert_eq!(cat.validate(), Err(CategoryValidationError::InvalidColor));
    }

    #[test]
    fn test_validate_invalid_color_wrong_length() {
        let cat = Category::new("Work", "#3B82");
        assert_eq!(cat.validate(), Err(CategoryValidationError::InvalidColor));
    }

    #[test]
    fn test_validate_invalid_color_non_hex() {
        let cat = Category::new("Work", "#GGGGGG");
        assert_eq!(cat.validate(), Err(CategoryValidationError::InvalidColor));
    }

    #[test]
    fn test_validate_valid_short_hex() {
        let cat = Category::new("Work", "#FFF");
        assert!(cat.validate().is_ok());
    }

    #[test]
    fn test_validate_valid_rgba_hex() {
        let cat = Category::new("Work", "#FF0000FF");
        assert!(cat.validate().is_ok());
    }

    #[test]
    fn test_default_categories() {
        let defaults = default_categories();
        assert_eq!(defaults.len(), 6);
        
        // All should be system categories
        for cat in &defaults {
            assert!(cat.is_system);
            assert!(cat.icon.is_some());
            assert!(cat.validate().is_ok());
        }
        
        // Check specific categories exist
        let names: Vec<&str> = defaults.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"Work"));
        assert!(names.contains(&"Personal"));
        assert!(names.contains(&"Birthday"));
    }

    #[test]
    fn test_is_valid_hex_color() {
        assert!(is_valid_hex_color("#FFF"));
        assert!(is_valid_hex_color("#FFFFFF"));
        assert!(is_valid_hex_color("#FF0000FF"));
        assert!(is_valid_hex_color("#abc"));
        assert!(is_valid_hex_color("#AbCdEf"));
        
        assert!(!is_valid_hex_color("FFF"));
        assert!(!is_valid_hex_color("#FF"));
        assert!(!is_valid_hex_color("#FFFF"));
        assert!(!is_valid_hex_color("#GGG"));
        assert!(!is_valid_hex_color(""));
    }
}
