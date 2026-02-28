use super::super::CalendarApp;
use crate::ui_egui::dialogs::theme_creator::{render_theme_creator, ThemeCreatorAction};
use crate::ui_egui::dialogs::theme_dialog::{render_theme_dialog, ThemeDialogAction};
use crate::ui_egui::theme::CalendarTheme;

impl CalendarApp {
    pub(in crate::ui_egui) fn render_theme_dialog(&mut self, ctx: &egui::Context) {
        let theme_service = self.context.theme_service();
        let available_themes = theme_service.list_themes().unwrap_or_default();

        // Cache custom theme colors for preview swatches
        for theme_name in &available_themes {
            if !CalendarTheme::is_builtin(theme_name)
                && !self
                    .state
                    .theme_dialog_state
                    .custom_theme_colors
                    .contains_key(theme_name)
            {
                if let Ok(theme) = theme_service.get_theme(theme_name) {
                    self.state
                        .theme_dialog_state
                        .cache_theme_colors(theme_name, theme.preview_colors());
                }
            }
        }

        let action = render_theme_dialog(
            ctx,
            &mut self.state.theme_dialog_state,
            &available_themes,
            &self.settings.theme,
        );

        match action {
            ThemeDialogAction::None => {}
            ThemeDialogAction::CreateTheme => {
                let base_theme = theme_service
                    .get_theme(&self.settings.theme)
                    .unwrap_or_else(|_| CalendarTheme::light());
                self.state.theme_creator_state.open_create(base_theme);
            }
            ThemeDialogAction::EditTheme(name) => {
                if let Ok(theme) = theme_service.get_theme(&name) {
                    self.state.theme_creator_state.open_edit(name, theme);
                }
            }
            ThemeDialogAction::DeleteTheme(name) => {
                if let Err(e) = theme_service.delete_theme(&name) {
                    log::error!("Failed to delete theme: {}", e);
                    self.toast_manager
                        .error(format!("Failed to delete theme: {}", e));
                } else {
                    log::info!("Successfully deleted theme: {}", name);
                    self.toast_manager
                        .success(format!("Deleted theme \"{}\"", name));
                    // Clear cached colors
                    self.state
                        .theme_dialog_state
                        .custom_theme_colors
                        .remove(&name);
                    // If we deleted the current theme, switch to Light
                    if self.settings.theme.eq_ignore_ascii_case(&name) {
                        self.settings.theme = "Light".to_string();
                        let theme = CalendarTheme::light();
                        theme.apply_to_context(ctx);
                        self.active_theme = theme;
                        let settings_service = self.context.settings_service();
                        let _ = settings_service.update(&self.settings);
                    }
                }
            }
            ThemeDialogAction::ApplyTheme(name) => {
                self.settings.theme = name.clone();
                self.state.theme_dialog_state.preview_theme = None;

                if let Ok(theme) = theme_service.get_theme(&name) {
                    theme.apply_to_context(ctx);
                    self.active_theme = theme.clone();
                    self.toast_manager
                        .success(format!("Applied theme \"{}\"", name));
                } else {
                    let fallback = Self::fallback_theme_for_settings(&self.settings);
                    fallback.apply_to_context(ctx);
                    self.active_theme = fallback;
                }

                let settings_service = self.context.settings_service();
                if let Err(e) = settings_service.update(&self.settings) {
                    log::error!("Failed to save theme setting: {}", e);
                }
            }
            ThemeDialogAction::PreviewTheme(name) => {
                // Temporarily apply theme for preview (don't save to settings)
                if let Ok(theme) = theme_service.get_theme(&name) {
                    theme.apply_to_context(ctx);
                    // Also update active_theme so all UI components use preview colors
                    self.active_theme = theme;
                    self.state.theme_dialog_state.preview_theme = Some(name);
                }
            }
            ThemeDialogAction::RevertPreview => {
                // Revert to the original theme
                if let Some(original) = &self.state.theme_dialog_state.original_theme {
                    if let Ok(theme) = theme_service.get_theme(original) {
                        theme.apply_to_context(ctx);
                        // Restore active_theme to original
                        self.active_theme = theme;
                    }
                }
                self.state.theme_dialog_state.preview_theme = None;
            }
            ThemeDialogAction::DuplicateTheme { source, new_name } => {
                if let Err(e) = theme_service.duplicate_theme(&source, &new_name) {
                    log::error!("Failed to duplicate theme: {}", e);
                    self.toast_manager
                        .error(format!("Failed to duplicate: {}", e));
                } else {
                    log::info!(
                        "Successfully duplicated theme '{}' to '{}'",
                        source,
                        new_name
                    );
                    self.toast_manager
                        .success(format!("Created \"{}\"", new_name));
                    // Cache colors for the new theme
                    if let Ok(theme) = theme_service.get_theme(&new_name) {
                        self.state
                            .theme_dialog_state
                            .cache_theme_colors(&new_name, theme.preview_colors());
                    }
                }
            }
            ThemeDialogAction::ExportTheme(name) => {
                // Use native file dialog to save
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Export Theme")
                    .set_file_name(format!("{}.toml", name))
                    .add_filter("TOML files", &["toml"])
                    .save_file()
                {
                    if let Err(e) = theme_service.export_theme(&name, &path) {
                        log::error!("Failed to export theme: {}", e);
                        self.toast_manager
                            .error(format!("Export failed: {}", e));
                    } else {
                        log::info!("Successfully exported theme to {:?}", path);
                        self.toast_manager.success("Theme exported");
                    }
                }
            }
            ThemeDialogAction::ImportTheme => {
                // Use native file dialog to open
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Import Theme")
                    .add_filter("TOML files", &["toml"])
                    .pick_file()
                {
                    match theme_service.import_theme(&path) {
                        Ok(name) => {
                            log::info!("Successfully imported theme: {}", name);
                            self.toast_manager
                                .success(format!("Imported \"{}\"", name));
                            // Cache colors for the imported theme
                            if let Ok(theme) = theme_service.get_theme(&name) {
                                self.state
                                    .theme_dialog_state
                                    .cache_theme_colors(&name, theme.preview_colors());
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to import theme: {}", e);
                            self.toast_manager
                                .error(format!("Import failed: {}", e));
                        }
                    }
                }
            }
            ThemeDialogAction::Close => {
                self.state.theme_dialog_state.close();
            }
        }
    }

    pub(in crate::ui_egui) fn render_theme_creator(&mut self, ctx: &egui::Context) {
        let action = render_theme_creator(ctx, &mut self.state.theme_creator_state);

        match action {
            ThemeCreatorAction::None => {}
            ThemeCreatorAction::Save(name, theme) => {
                let theme_service = self.context.theme_service();
                if let Err(e) = theme_service.save_theme(&theme, &name) {
                    log::error!("Failed to save theme: {}", e);
                    self.state.theme_creator_state.validation_error =
                        Some(format!("Failed to save: {}", e));
                    self.state.theme_creator_state.is_open = true;
                } else {
                    log::info!("Successfully saved theme: {}", name);

                    self.settings.theme = name.clone();
                    theme.apply_to_context(ctx);
                    self.active_theme = theme.clone();

                    let settings_service = self.context.settings_service();
                    if let Err(e) = settings_service.update(&self.settings) {
                        log::error!("Failed to save settings: {}", e);
                    }

                    self.state.theme_creator_state.close();
                }
            }
            ThemeCreatorAction::Cancel => {
                self.state.theme_creator_state.close();
            }
        }
    }
}
