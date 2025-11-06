// Rust Calendar Application
// Main entry point

mod models;
mod services;
mod ui;
mod utils;

use iced::{Application, Settings};
use std::path::PathBuf;

fn main() -> iced::Result {
    // Initialize logging
    env_logger::init();
    
    log::info!("Starting Rust Calendar Application");
    
    // Get database path
    let db_path = get_database_path();
    
    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("Warning: Failed to create database directory: {}", e);
        }
    }
    
    log::info!("Using database at: {}", db_path.display());
    
    // Run the iced application
    ui::CalendarApp::run(Settings {
        flags: db_path.to_string_lossy().to_string(),
        window: iced::window::Settings {
            size: iced::Size::new(1200.0, 800.0),
            min_size: Some(iced::Size::new(800.0, 600.0)),
            ..Default::default()
        },
        default_font: iced::Font::with_name("Segoe UI Emoji"),
        ..Default::default()
    })
}

/// Get the path to the database file
fn get_database_path() -> PathBuf {
    // Use local directory for now
    // TODO: Use proper app data directory when directories crate is added
    PathBuf::from("calendar.db")
}
