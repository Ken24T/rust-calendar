//! Unified theme dialog combining quick theme selection and custom theme management

use crate::ui_egui::theme::{CalendarTheme, ThemePreset};
use egui::{Align, Color32, Context, Layout, RichText, Stroke, Vec2, Window};
use std::collections::HashMap;

/// State for the unified theme dialog
#[derive(Default)]
pub struct ThemeDialogState {
    pub is_open: bool,
    /// Theme to be deleted (for confirmation)
    pub delete_confirm: Option<String>,
    /// Original theme when dialog opened (for live preview revert)
    pub original_theme: Option<String>,
    /// Preview mode - temporarily applying a theme
    pub preview_theme: Option<String>,
    /// Cached preview colors for custom themes
    pub custom_theme_colors: HashMap<String, [Color32; 4]>,
    /// Theme to duplicate
    pub duplicate_source: Option<String>,
    /// Name for the duplicated theme
    pub duplicate_name: String,
}

impl ThemeDialogState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, current_theme: &str) {
        self.is_open = true;
        self.original_theme = Some(current_theme.to_string());
        self.delete_confirm = None;
        self.preview_theme = None;
        self.duplicate_source = None;
        self.duplicate_name.clear();
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.delete_confirm = None;
        self.preview_theme = None;
        self.duplicate_source = None;
        self.duplicate_name.clear();
    }

    /// Cache preview colors for a custom theme
    pub fn cache_theme_colors(&mut self, name: &str, colors: [Color32; 4]) {
        self.custom_theme_colors.insert(name.to_string(), colors);
    }
}

/// Result of rendering the theme dialog
#[derive(Debug, Clone)]
pub enum ThemeDialogAction {
    None,
    ApplyTheme(String),
    PreviewTheme(String),
    RevertPreview,
    CreateTheme,
    EditTheme(String),
    DeleteTheme(String),
    DuplicateTheme { source: String, new_name: String },
    ExportTheme(String),
    ImportTheme,
    Close,
}

/// Render color swatches for a theme
fn render_color_swatches(ui: &mut egui::Ui, colors: [Color32; 4]) {
    let size = Vec2::new(12.0, 12.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for color in colors {
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, color);
            ui.painter().rect_stroke(rect, 2.0, Stroke::new(0.5, Color32::GRAY));
        }
    });
}

/// Get preview colors for a preset theme
fn get_preset_colors(preset: ThemePreset) -> [Color32; 4] {
    preset.to_theme().preview_colors()
}

/// Render the unified theme dialog
pub fn render_theme_dialog(
    ctx: &Context,
    state: &mut ThemeDialogState,
    available_themes: &[String],
    current_theme: &str,
) -> ThemeDialogAction {
    if !state.is_open {
        return ThemeDialogAction::None;
    }

    let mut action = ThemeDialogAction::None;
    let mut is_open = true;

    // Handle delete confirmation dialog
    if let Some(theme_to_delete) = &state.delete_confirm.clone() {
        let mut show_confirm = true;
        Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!("Are you sure you want to delete '{}'?", theme_to_delete));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        action = ThemeDialogAction::DeleteTheme(theme_to_delete.clone());
                        show_confirm = false;
                    }
                    if ui.button("Cancel").clicked() {
                        show_confirm = false;
                    }
                });
            });
        if !show_confirm {
            state.delete_confirm = None;
        }
        if !matches!(action, ThemeDialogAction::None) {
            return action;
        }
    }

    // Handle duplicate dialog
    if let Some(source) = &state.duplicate_source.clone() {
        let mut show_duplicate = true;
        Window::new("Duplicate Theme")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!("Create a copy of '{}'", source));
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label("New name:");
                    ui.text_edit_singleline(&mut state.duplicate_name);
                });
                ui.add_space(10.0);
                
                let name_valid = !state.duplicate_name.trim().is_empty() 
                    && !CalendarTheme::is_builtin(&state.duplicate_name);
                
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(name_valid, |ui| {
                        if ui.button("Duplicate").clicked() {
                            action = ThemeDialogAction::DuplicateTheme {
                                source: source.clone(),
                                new_name: state.duplicate_name.trim().to_string(),
                            };
                            show_duplicate = false;
                        }
                    });
                    if ui.button("Cancel").clicked() {
                        show_duplicate = false;
                    }
                });
                
                if !name_valid && !state.duplicate_name.is_empty() {
                    if CalendarTheme::is_builtin(&state.duplicate_name) {
                        ui.colored_label(ui.visuals().error_fg_color, "Cannot use a built-in theme name");
                    }
                }
            });
        if !show_duplicate {
            state.duplicate_source = None;
            state.duplicate_name.clear();
        }
        if !matches!(action, ThemeDialogAction::None) {
            return action;
        }
    }

    Window::new("Themes")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .default_width(450.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Built-in theme presets
                ui.heading("Theme Presets");
                ui.add_space(5.0);

                egui::Grid::new("preset_themes_grid")
                    .num_columns(2)
                    .spacing([20.0, 8.0])
                    .show(ui, |ui| {
                        for (i, preset) in ThemePreset::all().iter().enumerate() {
                            let name = preset.name();
                            let is_current = current_theme.eq_ignore_ascii_case(name);
                            let colors = get_preset_colors(*preset);

                            ui.horizontal(|ui| {
                                render_color_swatches(ui, colors);
                                
                                let label = if is_current {
                                    RichText::new(format!("{} {}", preset.icon(), name))
                                        .strong()
                                        .color(ui.visuals().hyperlink_color)
                                } else {
                                    RichText::new(format!("{} {}", preset.icon(), name))
                                };

                                let response = ui.selectable_label(is_current, label);
                                
                                // Live preview on hover
                                if response.hovered() && !is_current && state.preview_theme.as_deref() != Some(name) {
                                    action = ThemeDialogAction::PreviewTheme(name.to_string());
                                }
                                
                                if response.clicked() && !is_current {
                                    action = ThemeDialogAction::ApplyTheme(name.to_string());
                                }
                            });

                            // Two columns
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Custom themes section
                ui.horizontal(|ui| {
                    ui.heading("Custom Themes");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("📥 Import").clicked() {
                            action = ThemeDialogAction::ImportTheme;
                        }
                    });
                });
                ui.add_space(5.0);

                let custom_themes: Vec<_> = available_themes
                    .iter()
                    .filter(|name| !CalendarTheme::is_builtin(name))
                    .collect();

                if custom_themes.is_empty() {
                    ui.label(RichText::new("No custom themes yet. Create one or import from a file.").weak().italics());
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for theme_name in &custom_themes {
                                let is_current = theme_name.eq_ignore_ascii_case(current_theme);

                                ui.horizontal(|ui| {
                                    // Color swatches (from cache if available)
                                    if let Some(colors) = state.custom_theme_colors.get(*theme_name) {
                                        render_color_swatches(ui, *colors);
                                    } else {
                                        // Placeholder swatches
                                        render_color_swatches(ui, [Color32::GRAY; 4]);
                                    }

                                    // Theme name - clickable to apply
                                    let label = if is_current {
                                        RichText::new(theme_name.as_str())
                                            .strong()
                                            .color(ui.visuals().hyperlink_color)
                                    } else {
                                        RichText::new(theme_name.as_str())
                                    };

                                    let response = ui.selectable_label(is_current, label);
                                    
                                    // Live preview on hover
                                    if response.hovered() && !is_current && state.preview_theme.as_deref() != Some(theme_name.as_str()) {
                                        action = ThemeDialogAction::PreviewTheme(theme_name.to_string());
                                    }
                                    
                                    if response.clicked() && !is_current {
                                        action = ThemeDialogAction::ApplyTheme(theme_name.to_string());
                                    }

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        // Delete button
                                        if ui.small_button("🗑").on_hover_text("Delete").clicked() {
                                            state.delete_confirm = Some(theme_name.to_string());
                                        }

                                        // Export button
                                        if ui.small_button("📤").on_hover_text("Export").clicked() {
                                            action = ThemeDialogAction::ExportTheme(theme_name.to_string());
                                        }

                                        // Duplicate button
                                        if ui.small_button("📋").on_hover_text("Duplicate").clicked() {
                                            state.duplicate_source = Some(theme_name.to_string());
                                            state.duplicate_name = format!("{} Copy", theme_name);
                                        }

                                        // Edit button
                                        if ui.small_button("✏").on_hover_text("Edit").clicked() {
                                            action = ThemeDialogAction::EditTheme(theme_name.to_string());
                                        }
                                    });
                                });

                                ui.add_space(3.0);
                            }
                        });
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("✨ Create Custom Theme").clicked() {
                        action = ThemeDialogAction::CreateTheme;
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            action = ThemeDialogAction::Close;
                        }
                    });
                });
            });
        });

    if !is_open {
        action = ThemeDialogAction::Close;
    }

    if matches!(action, ThemeDialogAction::Close) {
        // Revert to original theme if we were previewing
        if state.preview_theme.is_some() {
            if let Some(original) = &state.original_theme {
                action = ThemeDialogAction::ApplyTheme(original.clone());
            }
        }
        state.close();
    }

    action
}
