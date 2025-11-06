// Integration tests for database and settings persistence
use rust_calendar::services::database::Database;
use rust_calendar::services::settings::SettingsService;
use rust_calendar::models::settings::Settings;
use std::path::PathBuf;

#[test]
fn test_settings_persistence() {
    // Create a temporary database file
    let test_db_path = PathBuf::from("test_integration.db");
    
    // Clean up any existing test database
    if test_db_path.exists() {
        std::fs::remove_file(&test_db_path).ok();
    }
    
    // Initialize database
    let db = Database::new(test_db_path.to_str().unwrap()).expect("Failed to create database");
    db.initialize_schema().expect("Failed to initialize schema");
    
    let settings_service = SettingsService::new(&db);
    
    // Get default settings
    let mut settings = settings_service.get().expect("Failed to get settings");
    assert_eq!(settings.theme, "light");
    assert_eq!(settings.show_my_day, false);
    assert_eq!(settings.show_ribbon, false);
    assert_eq!(settings.current_view, "Month");
    
    // Update settings to simulate UI changes
    settings.theme = "dark".to_string();
    settings.show_my_day = true;
    settings.show_ribbon = true;
    settings.current_view = "Week".to_string();
    
    settings_service.update(&settings).expect("Failed to update settings");
    
    // Verify persistence by reading again
    let loaded_settings = settings_service.get().expect("Failed to load settings");
    assert_eq!(loaded_settings.theme, "dark");
    assert_eq!(loaded_settings.show_my_day, true);
    assert_eq!(loaded_settings.show_ribbon, true);
    assert_eq!(loaded_settings.current_view, "Week");
    
    // Clean up
    std::fs::remove_file(&test_db_path).ok();
}

#[test]
fn test_app_lifecycle_simulation() {
    // Create a temporary database file
    let test_db_path = PathBuf::from("test_lifecycle.db");
    
    // Clean up any existing test database
    if test_db_path.exists() {
        std::fs::remove_file(&test_db_path).ok();
    }
    
    // Simulate first app launch
    {
        let db = Database::new(test_db_path.to_str().unwrap()).expect("Failed to create database");
        db.initialize_schema().expect("Failed to initialize schema");
        
        let settings_service = SettingsService::new(&db);
        let mut settings = settings_service.get().expect("Failed to get settings");
        
        // User changes theme to dark
        settings.theme = "dark".to_string();
        settings_service.update(&settings).expect("Failed to save theme");
    } // Database connection closed
    
    // Simulate second app launch - settings should persist
    {
        let db = Database::new(test_db_path.to_str().unwrap()).expect("Failed to open database");
        let settings_service = SettingsService::new(&db);
        let settings = settings_service.get().expect("Failed to load settings");
        
        // Verify theme persisted across app restarts
        assert_eq!(settings.theme, "dark", "Theme should persist across app restarts");
    }
    
    // Clean up
    std::fs::remove_file(&test_db_path).ok();
}

#[test]
fn test_view_type_persistence() {
    let test_db_path = PathBuf::from("test_view_types.db");
    
    if test_db_path.exists() {
        std::fs::remove_file(&test_db_path).ok();
    }
    
    let db = Database::new(test_db_path.to_str().unwrap()).expect("Failed to create database");
    db.initialize_schema().expect("Failed to initialize schema");
    
    let settings_service = SettingsService::new(&db);
    
    // Test each view type
    let view_types = vec!["Day", "WorkWeek", "Week", "Month", "Quarter"];
    
    for view_type in view_types {
        let mut settings = settings_service.get().expect("Failed to get settings");
        settings.current_view = view_type.to_string();
        settings_service.update(&settings).expect("Failed to update view");
        
        let loaded = settings_service.get().expect("Failed to load settings");
        assert_eq!(loaded.current_view, view_type, "View type '{}' should persist", view_type);
    }
    
    std::fs::remove_file(&test_db_path).ok();
}
