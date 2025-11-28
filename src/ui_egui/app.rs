#[path = "app/context.rs"]
mod context;
mod confirm;
mod countdown;
mod dialogs;
mod geometry;
mod imports;
mod lifecycle;
mod menu;
mod navigation;
mod notifications;
mod shortcuts;
mod sidebar;
mod state;
mod status_bar;
mod toast;
mod views;

use self::context::AppContext;
use self::countdown::CountdownUiState;
use self::state::{AppState, ViewType};
use self::toast::ToastManager;
use self::confirm::ConfirmDialogState;
use crate::models::settings::Settings;
use crate::ui_egui::commands::UndoManager;
use crate::ui_egui::event_dialog::EventDialogState;
use crate::ui_egui::theme::CalendarTheme;
use crate::ui_egui::views::AutoFocusRequest;
use chrono::NaiveDate;

pub struct CalendarApp {
    /// Shared access to leaked database and supporting services
    context: AppContext,
    /// Core application settings/stateful data
    settings: Settings,
    current_view: ViewType,
    current_date: NaiveDate,
    show_event_dialog: bool,
    show_settings_dialog: bool,
    /// Ribbon toggle mirrors persisted settings
    show_ribbon: bool,
    /// Currently applied theme colors
    active_theme: CalendarTheme,
    /// Event dialog state management
    event_dialog_state: Option<EventDialogState>,
    event_dialog_date: Option<NaiveDate>,
    event_dialog_time: Option<chrono::NaiveTime>,
    event_dialog_recurrence: Option<String>,
    event_to_edit: Option<i64>,
    pending_focus: Option<AutoFocusRequest>,
    /// Countdown window and dialogs
    countdown_ui: CountdownUiState,
    /// Aggregated dialog/control state
    state: AppState,
    /// Toast notification manager
    toast_manager: ToastManager,
    /// Confirmation dialog state
    confirm_dialog: ConfirmDialogState,
    /// Active category filter (None = show all categories)
    active_category_filter: Option<String>,
    /// Undo/Redo manager for event operations
    undo_manager: UndoManager,
}

impl eframe::App for CalendarApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_update(ctx, frame);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.handle_exit(gl);
    }
}
