// Rust Calendar Application
// Main entry point - using egui/eframe

// Hide console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod services;
mod ui_egui;
mod utils;

use eframe::egui;

fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../assets/icons/663353.png");

    // Decode PNG manually
    let decoder = png::Decoder::new(&icon_bytes[..]);
    let mut reader = decoder.read_info().expect("Failed to read PNG info");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("Failed to decode PNG");

    egui::IconData {
        rgba: buf,
        width: info.width,
        height: info.height,
    }
}

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
            .with_drag_and_drop(true)
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Rust Calendar",
        options,
        Box::new(|cc| Ok(Box::new(ui_egui::CalendarApp::new(cc)))),
    )
}
