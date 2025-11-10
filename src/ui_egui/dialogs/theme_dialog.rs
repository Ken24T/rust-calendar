//! Unified theme dialog combining quick theme selection and custom theme management

use egui::{Align, Context, Layout, RichText, Window};

/// State for the unified theme dialog
#[derive(Default)]
pub struct ThemeDialogState {
    pub is_open: bool,
}

impl ThemeDialogState {
    pub fn new() -> Self {
        Self { is_open: false }
    }
    
    pub fn open(&mut self) {
        self.is_open = true;
    }
    
    pub fn close(&mut self) {
        self.is_open = false;
    }
}

/// Result of rendering the theme dialog
#[derive(Debug, Clone)]
pub enum ThemeDialogAction {
    None,
    ApplyTheme(String),
    CreateTheme,
    EditTheme(String),
    DeleteTheme(String),
    Close,
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
    
    Window::new("Themes")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .default_width(380.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Quick theme selection for built-in themes
                ui.heading("Quick Selection");
                ui.add_space(5.0);
                
                ui.horizontal(|ui| {
                    let is_light = current_theme.to_lowercase() == "light";
                    let is_dark = current_theme.to_lowercase() == "dark";
                    
                    if ui.selectable_label(is_light, "☀ Light").clicked() {
                        action = ThemeDialogAction::ApplyTheme("light".to_string());
                    }
                    
                    if ui.selectable_label(is_dark, "🌙 Dark").clicked() {
                        action = ThemeDialogAction::ApplyTheme("dark".to_string());
                    }
                });
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                
                // Custom themes section
                ui.heading("All Themes");
                ui.add_space(5.0);
                
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for theme_name in available_themes {
                            let is_builtin = theme_name.to_lowercase() == "light" 
                                || theme_name.to_lowercase() == "dark";
                            let is_current = theme_name.to_lowercase() == current_theme.to_lowercase();
                            
                            ui.horizontal(|ui| {
                                // Theme name with indicator if current
                                let label = if is_current {
                                    RichText::new(format!("• {}", theme_name))
                                        .strong()
                                        .color(ui.visuals().hyperlink_color)
                                } else {
                                    RichText::new(format!("  {}", theme_name))
                                };
                                
                                if ui.button(label).clicked() && !is_current {
                                    action = ThemeDialogAction::ApplyTheme(theme_name.to_lowercase());
                                }
                                
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if is_builtin {
                                        ui.label(RichText::new("(Built-in)").weak().italics());
                                    } else {
                                        // Delete button for custom themes
                                        if ui.small_button("Delete").clicked() {
                                            action = ThemeDialogAction::DeleteTheme(theme_name.clone());
                                        }
                                        
                                        // Edit button for custom themes
                                        if ui.small_button("Edit").clicked() {
                                            action = ThemeDialogAction::EditTheme(theme_name.clone());
                                        }
                                    }
                                });
                            });
                            
                            ui.add_space(3.0);
                        }
                    });
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                
                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("Create Custom Theme").clicked() {
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
        state.close();
    }
    
    action
}
