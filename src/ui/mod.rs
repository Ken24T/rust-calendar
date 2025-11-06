// UI module exports
// Main application and components

mod app;
pub mod theme;
pub mod messages;
pub mod view_type;
pub mod helpers;
pub mod dialogs;

pub mod views;
pub mod components;

// Re-export the main application
pub use app::CalendarApp;
pub use view_type::ViewType;
