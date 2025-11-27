use crate::services::countdown::CountdownCardGeometry;
use crate::ui_egui::dialogs::backup_manager::BackupManagerState;
use crate::ui_egui::dialogs::export_dialog::ExportDialogState;
use crate::ui_egui::dialogs::search_dialog::SearchDialogState;
use crate::ui_egui::dialogs::template_manager::TemplateManagerState;
use crate::ui_egui::dialogs::theme_creator::ThemeCreatorState;
use crate::ui_egui::dialogs::theme_dialog::ThemeDialogState;
use chrono::NaiveDate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    Day,
    Week,
    WorkWeek,
    Month,
}

/// State for the date picker popup used in navigation
#[derive(Default)]
pub struct DatePickerState {
    pub is_open: bool,
    /// The month currently being viewed in the picker (may differ from selected date)
    pub viewing_date: Option<NaiveDate>,
}

impl DatePickerState {
    pub fn open(&mut self, current_date: NaiveDate) {
        self.is_open = true;
        self.viewing_date = Some(current_date);
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.viewing_date = None;
    }
}

pub struct AppState {
    pub backup_manager_state: BackupManagerState,
    pub theme_dialog_state: ThemeDialogState,
    pub theme_creator_state: ThemeCreatorState,
    pub search_dialog_state: SearchDialogState,
    pub template_manager_state: TemplateManagerState,
    pub show_search_dialog: bool,
    pub show_about_dialog: bool,
    pub show_export_range_dialog: bool,
    pub export_dialog_state: ExportDialogState,
    pub pending_root_geometry: Option<CountdownCardGeometry>,
    pub date_picker_state: DatePickerState,
    /// Whether we've done the initial geometry sanitization on first frame
    pub geometry_sanitized: bool,
}

impl AppState {
    pub fn new(
        backup_manager_state: BackupManagerState,
        pending_root_geometry: Option<CountdownCardGeometry>,
    ) -> Self {
        Self {
            backup_manager_state,
            theme_dialog_state: ThemeDialogState::new(),
            theme_creator_state: ThemeCreatorState::new(),
            search_dialog_state: SearchDialogState::default(),
            template_manager_state: TemplateManagerState::default(),
            show_search_dialog: false,
            show_about_dialog: false,
            show_export_range_dialog: false,
            export_dialog_state: ExportDialogState::default(),
            pending_root_geometry,
            date_picker_state: DatePickerState::default(),
            geometry_sanitized: false,
        }
    }
}
