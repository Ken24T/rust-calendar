//! Dialog modules for the calendar application

pub mod date_picker;
pub mod event_dialog;
pub mod theme_picker;
pub mod theme_manager;
pub mod theme_creator;
pub mod settings;
pub mod color_picker;

pub use date_picker::create_date_picker_dialog;
pub use event_dialog::{create_event_dialog, EventDialogState};
pub use theme_picker::create_theme_picker_dialog;
pub use theme_manager::create_theme_manager_dialog;
pub use settings::create_settings_dialog;
pub use color_picker::create_color_picker_dialog;
