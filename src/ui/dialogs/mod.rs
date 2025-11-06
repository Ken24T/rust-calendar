//! Dialog modules for the calendar application

pub mod date_picker;
pub mod theme_picker;
pub mod theme_manager;

pub use date_picker::create_date_picker_dialog;
pub use theme_picker::create_theme_picker_dialog;
pub use theme_manager::create_theme_manager_dialog;
