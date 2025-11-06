// Rust Calendar Application
// Main entry point

mod models;
mod services;
mod ui;
mod utils;

use iced::{Application, Settings};

fn main() -> iced::Result {
    // Initialize logging
    env_logger::init();
    
    log::info!("Starting Rust Calendar Application");
    
    // Run the iced application
    ui::CalendarApp::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(1200.0, 800.0),
            min_size: Some(iced::Size::new(800.0, 600.0)),
            ..Default::default()
        },
        default_font: iced::Font::with_name("Segoe UI Emoji"),
        ..Default::default()
    })
}
