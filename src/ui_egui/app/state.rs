use crate::services::countdown::CountdownCardGeometry;
use crate::ui_egui::dialogs::backup_manager::BackupManagerState;
use crate::ui_egui::dialogs::theme_creator::ThemeCreatorState;
use crate::ui_egui::dialogs::theme_dialog::ThemeDialogState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    Day,
    Week,
    WorkWeek,
    Month,
}

pub struct AppState {
    pub backup_manager_state: BackupManagerState,
    pub theme_dialog_state: ThemeDialogState,
    pub theme_creator_state: ThemeCreatorState,
    pub pending_root_geometry: Option<CountdownCardGeometry>,
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
            pending_root_geometry,
        }
    }
}
