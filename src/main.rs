// Rust Calendar Application
// Main entry point - using egui/eframe

mod models;
mod services;
mod ui_egui;
mod utils;

use eframe::egui;

fn main() -> eframe::Result<()> {
    // Initialize logging
    env_logger::init();

    log::info!("Starting Rust Calendar Application (egui version)");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Rust Calendar")
            .with_active(true)
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Rust Calendar",
        options,
        Box::new(|cc| Ok(Box::new(ui_egui::CalendarApp::new(cc)))),
    )
}
