// UI module exports
// Main application and components

mod app;
pub mod theme;
pub mod messages;

pub mod views;
pub mod components;

// Re-export the main application
pub use app::CalendarApp;
