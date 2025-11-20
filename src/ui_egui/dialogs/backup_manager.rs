use crate::services::backup::{BackupInfo, BackupService};
use anyhow::Result;
use egui::{Color32, RichText};
use std::path::PathBuf;

pub struct BackupManagerState {
    pub show_dialog: bool,
    backups: Vec<BackupInfo>,
    error_message: Option<String>,
    success_message: Option<String>,
    confirm_restore_index: Option<usize>,
    confirm_delete_index: Option<usize>,
    db_path: PathBuf,
}

impl BackupManagerState {
    pub fn new(db_path: PathBuf) -> Self {
        let backups = BackupService::list_backups(None).unwrap_or_default();
        Self {
            show_dialog: false,
            backups,
            error_message: None,
            success_message: None,
            confirm_restore_index: None,
            confirm_delete_index: None,
            db_path,
        }
    }

    pub fn open(&mut self) {
        self.show_dialog = true;
        self.refresh_backups();
        self.clear_messages();
    }

    pub fn close(&mut self) {
        self.show_dialog = false;
        self.clear_messages();
        self.confirm_restore_index = None;
        self.confirm_delete_index = None;
    }

    pub fn refresh_backups(&mut self) {
        match BackupService::list_backups(None) {
            Ok(backups) => {
                self.backups = backups;
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to list backups: {}", e));
            }
        }
    }

    fn clear_messages(&mut self) {
        self.error_message = None;
        self.success_message = None;
    }

    pub fn create_backup(&mut self) -> Result<()> {
        let backup_path = BackupService::create_backup(&self.db_path, None)?;
        self.success_message = Some(format!(
            "Backup created successfully: {}",
            backup_path.file_name().unwrap().to_string_lossy()
        ));
        self.refresh_backups();
        Ok(())
    }

    pub fn restore_backup(&mut self, index: usize) -> Result<()> {
        if index >= self.backups.len() {
            anyhow::bail!("Invalid backup index");
        }

        let backup_info = &self.backups[index];
        BackupService::restore_backup(&backup_info.path, &self.db_path)?;

        self.success_message = Some(format!(
            "Database restored from backup: {}",
            backup_info.filename
        ));
        self.confirm_restore_index = None;
        Ok(())
    }

    pub fn delete_backup(&mut self, index: usize) -> Result<()> {
        if index >= self.backups.len() {
            anyhow::bail!("Invalid backup index");
        }

        let backup_info = &self.backups[index];
        BackupService::delete_backup(&backup_info.path)?;

        self.success_message = Some(format!("Backup deleted: {}", backup_info.filename));
        self.confirm_delete_index = None;
        self.refresh_backups();
        Ok(())
    }
}

/// Render the backup manager dialog
/// Returns true if the application should reload the database (after restore)
pub fn render_backup_manager_dialog(ctx: &egui::Context, state: &mut BackupManagerState) -> bool {
    let mut should_reload_db = false;

    if !state.show_dialog {
        return false;
    }

    let mut dialog_open = state.show_dialog;

    egui::Window::new("Backup Manager")
        .open(&mut dialog_open)
        .collapsible(false)
        .resizable(true)
        .default_width(700.0)
        .default_height(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header with create backup button
                ui.horizontal(|ui| {
                    ui.heading("Database Backups");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🔄 Refresh").clicked() {
                            state.refresh_backups();
                            state.clear_messages();
                        }
                        if ui.button("➕ Create Backup").clicked() {
                            state.clear_messages();
                            if let Err(e) = state.create_backup() {
                                state.error_message =
                                    Some(format!("Failed to create backup: {}", e));
                            }
                        }
                    });
                });

                ui.add_space(8.0);

                // Display messages
                if let Some(ref error) = state.error_message {
                    ui.colored_label(Color32::RED, RichText::new(error).strong());
                    ui.add_space(4.0);
                }

                if let Some(ref success) = state.success_message {
                    ui.colored_label(Color32::DARK_GREEN, RichText::new(success).strong());
                    ui.add_space(4.0);
                }

                ui.separator();
                ui.add_space(4.0);

                // Backup location info
                if let Ok(backup_dir) = BackupService::default_backup_dir() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Backup Location:").strong());
                        ui.label(backup_dir.to_string_lossy().to_string());
                    });
                    ui.add_space(8.0);
                }

                // Backup list
                if state.backups.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.label(
                            RichText::new("No backups found")
                                .size(16.0)
                                .color(Color32::GRAY),
                        );
                        ui.label("Create a backup to get started");
                    });
                } else {
                    // Clone backups to avoid borrow checker issues
                    let backups_clone = state.backups.clone();
                    let confirm_restore = state.confirm_restore_index;
                    let confirm_delete = state.confirm_delete_index;

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (index, backup) in backups_clone.iter().enumerate() {
                                render_backup_item(
                                    ui,
                                    backup,
                                    index,
                                    confirm_restore,
                                    confirm_delete,
                                    state,
                                    &mut should_reload_db,
                                );
                                ui.add_space(4.0);
                            }
                        });
                }

                ui.add_space(8.0);

                // Footer with info
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Total backups: {}", state.backups.len())).weak(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            state.close();
                        }
                    });
                });
            });
        });

    if !dialog_open {
        state.close();
    }

    should_reload_db
}

fn render_backup_item(
    ui: &mut egui::Ui,
    backup: &BackupInfo,
    index: usize,
    confirm_restore_index: Option<usize>,
    confirm_delete_index: Option<usize>,
    state: &mut BackupManagerState,
    should_reload_db: &mut bool,
) {
    let is_restore_confirm = confirm_restore_index == Some(index);
    let is_delete_confirm = confirm_delete_index == Some(index);

    egui::Frame::none()
        .fill(if index % 2 == 0 {
            ui.visuals().faint_bg_color
        } else {
            ui.visuals().extreme_bg_color
        })
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Backup info
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&backup.filename).strong());
                    });

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Created:").weak());
                        ui.label(backup.created_at.format("%Y-%m-%d %H:%M:%S").to_string());

                        ui.add_space(16.0);

                        ui.label(RichText::new("Size:").weak());
                        ui.label(BackupService::format_size(backup.size_bytes));
                    });
                });

                // Actions
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_restore_confirm {
                        // Confirmation buttons for restore
                        if ui.button("❌ Cancel").clicked() {
                            state.confirm_restore_index = None;
                            state.clear_messages();
                        }
                        if ui
                            .button(RichText::new("✓ Confirm Restore").color(Color32::RED))
                            .clicked()
                        {
                            state.clear_messages();
                            match state.restore_backup(index) {
                                Ok(()) => {
                                    state.success_message = Some(
                                        "Restore successful! Application will reload..."
                                            .to_string(),
                                    );
                                    *should_reload_db = true;
                                }
                                Err(e) => {
                                    state.error_message = Some(format!("Restore failed: {}", e));
                                }
                            }
                        }
                        ui.label(
                            RichText::new("⚠ This will overwrite the current database!")
                                .color(Color32::from_rgb(255, 165, 0)),
                        );
                    } else if is_delete_confirm {
                        // Confirmation buttons for delete
                        if ui.button("❌ Cancel").clicked() {
                            state.confirm_delete_index = None;
                            state.clear_messages();
                        }
                        if ui
                            .button(RichText::new("✓ Confirm Delete").color(Color32::RED))
                            .clicked()
                        {
                            state.clear_messages();
                            if let Err(e) = state.delete_backup(index) {
                                state.error_message = Some(format!("Delete failed: {}", e));
                            }
                        }
                    } else {
                        // Normal action buttons
                        if ui.button("🗑 Delete").clicked() {
                            state.confirm_delete_index = Some(index);
                            state.clear_messages();
                        }
                        if ui.button("↩ Restore").clicked() {
                            state.confirm_restore_index = Some(index);
                            state.clear_messages();
                        }
                    }
                });
            });
        });
}
