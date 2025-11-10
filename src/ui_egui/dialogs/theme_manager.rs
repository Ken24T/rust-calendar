//! Theme manager dialog for managing custom themes

use egui::{Align, Context, Layout, RichText, Window};

/// State for the theme manager dialog
#[derive(Default)]
pub struct ThemeManagerState {
    pub is_open: bool,
}

impl ThemeManagerState {
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

/// Result of rendering the theme manager dialog
#[derive(Debug, Clone)]
pub enum ThemeManagerAction {
    None,
    CreateTheme,
    EditTheme(String),
    DeleteTheme(String),
    ApplyTheme(String),
    Close,
}

/// Render the theme manager dialog
pub fn render_theme_manager(
    ctx: &Context,
    state: &mut ThemeManagerState,
    available_themes: &[String],
    current_theme: &str,
) -> ThemeManagerAction {
    if !state.is_open {
        return ThemeManagerAction::None;
    }
    
    let mut action = ThemeManagerAction::None;
    let mut is_open = true;
    
    Window::new("Manage Themes")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .default_width(350.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("Available Themes");
                ui.add_space(10.0);
                
                // List all themes
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        for theme_name in available_themes {
                            let is_builtin = theme_name == "Light" || theme_name == "Dark";
                            let is_current = theme_name == current_theme;
                            
                            ui.horizontal(|ui| {
                                // Theme name - highlight if current
                                let label = if is_current {
                                    RichText::new(format!("• {}", theme_name))
                                        .strong()
                                        .color(ui.visuals().hyperlink_color)
                                } else {
                                    RichText::new(format!("  {}", theme_name))
                                };
                                
                                if ui.button(label).clicked() {
                                    action = ThemeManagerAction::ApplyTheme(theme_name.clone());
                                }
                                
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if is_builtin {
                                        ui.label(RichText::new("(Built-in)").weak());
                                    } else {
                                        // Delete button for custom themes
                                        if ui.button("Delete").clicked() {
                                            action = ThemeManagerAction::DeleteTheme(theme_name.clone());
                                        }
                                        
                                        // Edit button for custom themes
                                        if ui.button("Edit").clicked() {
                                            action = ThemeManagerAction::EditTheme(theme_name.clone());
                                        }
                                    }
                                });
                            });
                            
                            ui.add_space(5.0);
                        }
                    });
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                
                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("Create New Theme").clicked() {
                        action = ThemeManagerAction::CreateTheme;
                    }
                    
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            action = ThemeManagerAction::Close;
                        }
                    });
                });
            });
        });
    
    if !is_open {
        action = ThemeManagerAction::Close;
    }
    
    if matches!(action, ThemeManagerAction::Close) {
        state.close();
    }
    
    action
}
