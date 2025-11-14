// Verification script to demonstrate settings persistence
use rust_calendar::services::database::Database;
use rust_calendar::services::settings::SettingsService;

fn main() {
    println!("=== Settings Persistence Verification ===\n");

    // Step 1: Initialize database (simulating first app launch)
    println!("Step 1: First app launch - initializing database...");
    {
        let db = Database::new("calendar.db").expect("Failed to create database");
        db.initialize_schema().expect("Failed to initialize schema");

        let service = SettingsService::new(&db);
        let settings = service.get().expect("Failed to get settings");

        println!("  Default settings loaded:");
        println!("    Theme: {}", settings.theme);
        println!("    Show My Day: {}", settings.show_my_day);
        println!("    Show Ribbon: {}", settings.show_ribbon);
        println!("    Current View: {}", settings.current_view);
    } // Database closed

    println!("\nStep 2: User changes preferences (theme to dark, enable My Day panel, switch to Week view)...");
    {
        let db = Database::new("calendar.db").expect("Failed to open database");
        let service = SettingsService::new(&db);

        let mut settings = service.get().expect("Failed to get settings");
        settings.theme = "dark".to_string();
        settings.show_my_day = true;
        settings.current_view = "Week".to_string();

        service.update(&settings).expect("Failed to save settings");
        println!("  Settings saved!");
    } // Database closed (simulating app close)

    println!("\nStep 3: App restart - loading settings...");
    {
        let db = Database::new("calendar.db").expect("Failed to open database");
        let service = SettingsService::new(&db);

        let settings = service.get().expect("Failed to load settings");

        println!("  Loaded settings:");
        println!("    Theme: {} ✓", settings.theme);
        println!("    Show My Day: {} ✓", settings.show_my_day);
        println!("    Show Ribbon: {}", settings.show_ribbon);
        println!("    Current View: {} ✓", settings.current_view);

        // Verify persistence
        assert_eq!(settings.theme, "dark", "Theme should persist");
        assert_eq!(settings.show_my_day, true, "My Day state should persist");
        assert_eq!(settings.current_view, "Week", "View should persist");
    }

    println!("\n✅ Verification complete! Settings successfully persist across app restarts.");
}
