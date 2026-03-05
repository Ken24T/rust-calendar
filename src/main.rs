// Rust Calendar Application
// Main entry point - using egui/eframe

// Hide console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod services;
mod ui_egui;
mod utils;

use eframe::egui;

fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../assets/icons/663353.png");

    // Decode PNG manually
    let decoder = png::Decoder::new(&icon_bytes[..]);
    let mut reader = match decoder.read_info() {
        Ok(reader) => reader,
        Err(err) => {
            log::warn!("Failed to read app icon PNG info: {}", err);
            return None;
        }
    };
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = match reader.next_frame(&mut buf) {
        Ok(info) => info,
        Err(err) => {
            log::warn!("Failed to decode app icon PNG: {}", err);
            return None;
        }
    };

    Some(egui::IconData {
        rgba: buf,
        width: info.width,
        height: info.height,
    })
}

fn main() -> eframe::Result<()> {
    // Initialize logging
    env_logger::init();

    log::info!("Starting Rust Calendar Application (egui version)");

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 800.0])
        .with_min_inner_size([800.0, 600.0])
        .with_title("Rust Calendar")
        .with_active(true)
        .with_drag_and_drop(true);

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Rust Calendar",
        options,
        Box::new(|cc| Ok(Box::new(ui_egui::CalendarApp::new(cc)))),
    )
}
