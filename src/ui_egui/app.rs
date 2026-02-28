#[path = "app/context.rs"]
mod context;
mod confirm;
mod countdown;
mod dialogs;
mod geometry;
mod imports;
mod lifecycle;
mod menu;
mod menu_export;
mod menu_help;
mod navigation;
mod notifications;
mod shortcuts;
mod sidebar;
mod sync_guard;
mod sync_scheduler;
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
use crate::services::calendar_sync::scheduler::CalendarSyncScheduler;
use crate::ui_egui::commands::UndoManager;
use crate::ui_egui::event_dialog::EventDialogState;
use crate::ui_egui::theme::CalendarTheme;
use crate::ui_egui::views::AutoFocusRequest;
use chrono::NaiveDate;
use std::sync::{Arc, Mutex};

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
    /// When true, only show events imported via calendar sync mappings
    show_synced_events_only: bool,
    /// Optional selected calendar source when synced-only filtering is enabled
    selected_synced_source_id: Option<i64>,
    /// Latest scheduler sync status text for status bar display
    calendar_sync_status_message: Option<String>,
    /// Whether the latest scheduler status indicates an error condition
    calendar_sync_status_is_error: bool,
    /// Next scheduled sync delay from the latest scheduler tick
    calendar_sync_next_due_in: Option<std::time::Duration>,
    /// Wall-clock due time for next scheduler run
    calendar_sync_poll_due_at: Option<std::time::Instant>,
    /// Receiver for in-flight background scheduler result
    calendar_sync_result_rx: Option<
        std::sync::mpsc::Receiver<
            Result<crate::services::calendar_sync::scheduler::SchedulerTickResult, String>,
        >,
    >,
    /// True while a background scheduled sync run is active
    calendar_sync_in_progress: bool,
    /// Undo/Redo manager for event operations
    undo_manager: UndoManager,
    /// Background scheduler for periodic calendar source sync
    calendar_sync_scheduler: Arc<Mutex<CalendarSyncScheduler>>,
}

impl eframe::App for CalendarApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_update(ctx, frame);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.handle_exit(gl);
    }
}
