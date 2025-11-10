//! Theme creator/editor dialog for creating and modifying custom themes

use egui::{Align, Color32, Context, Layout, RichText, TextEdit, Ui, Window};
use crate::ui_egui::theme::CalendarTheme;

/// State for the theme creator/editor dialog
pub struct ThemeCreatorState {
    pub is_open: bool,
    pub is_editing: bool,
    pub original_name: String,
    pub theme_name: String,
    pub theme: CalendarTheme,
    pub validation_error: Option<String>,
}

impl Default for ThemeCreatorState {
    fn default() -> Self {
        Self {
            is_open: false,
            is_editing: false,
            original_name: String::new(),
            theme_name: String::new(),
            theme: CalendarTheme::light(),
            validation_error: None,
        }
    }
}

impl ThemeCreatorState {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Open for creating a new theme based on an existing one
    pub fn open_create(&mut self, base_theme: CalendarTheme) {
        self.is_open = true;
        self.is_editing = false;
        self.original_name.clear();
        self.theme_name.clear();
        self.theme = base_theme;
        self.validation_error = None;
    }
    
    /// Open for editing an existing theme
    pub fn open_edit(&mut self, name: String, theme: CalendarTheme) {
        self.is_open = true;
        self.is_editing = true;
        self.original_name = name.clone();
        self.theme_name = name;
        self.theme = theme;
        self.validation_error = None;
    }
    
    pub fn close(&mut self) {
        self.is_open = false;
        self.validation_error = None;
    }
}

/// Result of rendering the theme creator dialog
#[derive(Debug, Clone)]
pub enum ThemeCreatorAction {
    None,
    Save(String, CalendarTheme),
    Cancel,
}

/// Render the theme creator/editor dialog
pub fn render_theme_creator(
    ctx: &Context,
    state: &mut ThemeCreatorState,
) -> ThemeCreatorAction {
    if !state.is_open {
        return ThemeCreatorAction::None;
    }
    
    let mut action = ThemeCreatorAction::None;
    let mut is_open = true;
    
    let title = if state.is_editing {
        "Edit Custom Theme"
    } else {
        "Create Custom Theme"
    };
    
    Window::new(title)
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .default_width(700.0)
        .default_height(600.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Validation error
                if let Some(error) = &state.validation_error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                    ui.add_space(10.0);
                }
                
                // Theme name
                ui.horizontal(|ui| {
                    ui.label("Theme Name:");
                    ui.add(
                        TextEdit::singleline(&mut state.theme_name)
                            .hint_text("Enter theme name...")
                            .desired_width(200.0)
                    );
                });
                
                ui.add_space(10.0);
                
                // Base theme selector (only for new themes)
                if !state.is_editing {
                    ui.horizontal(|ui| {
                        ui.label("Base Theme:");
                        if ui.radio(state.theme.is_dark, "Dark").clicked() {
                            state.theme = CalendarTheme::dark();
                        }
                        if ui.radio(!state.theme.is_dark, "Light").clicked() {
                            state.theme = CalendarTheme::light();
                        }
                    });
                    ui.add_space(10.0);
                }
                
                ui.separator();
                ui.add_space(10.0);
                
                // Two-column layout: colors on left, preview on right
                ui.columns(2, |columns| {
                    // Left column: Color pickers
                    columns[0].vertical(|ui| {
                        ui.heading("Theme Colors");
                        ui.add_space(10.0);
                        
                        render_color_picker(ui, "App Background", &mut state.theme.app_background);
                        render_color_picker(ui, "Calendar Background", &mut state.theme.calendar_background);
                        render_color_picker(ui, "Weekend Background", &mut state.theme.weekend_background);
                        render_color_picker(ui, "Today Background", &mut state.theme.today_background);
                        render_color_picker(ui, "Today Border", &mut state.theme.today_border);
                        render_color_picker(ui, "Day Background", &mut state.theme.day_background);
                        render_color_picker(ui, "Day Border", &mut state.theme.day_border);
                        render_color_picker(ui, "Primary Text", &mut state.theme.text_primary);
                        render_color_picker(ui, "Secondary Text", &mut state.theme.text_secondary);
                    });
                    
                    // Right column: Preview
                    columns[1].vertical(|ui| {
                        ui.heading("Preview");
                        ui.add_space(10.0);
                        
                        render_theme_preview(ui, &state.theme);
                    });
                });
                
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);
                
                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        if state.theme_name.trim().is_empty() {
                            state.validation_error = Some("Theme name cannot be empty".to_string());
                        } else if state.theme_name == "Light" || state.theme_name == "Dark" {
                            state.validation_error = Some("Cannot use built-in theme names".to_string());
                        } else {
                            action = ThemeCreatorAction::Save(
                                state.theme_name.clone(),
                                state.theme.clone()
                            );
                        }
                    }
                    
                    if ui.button("Cancel").clicked() {
                        action = ThemeCreatorAction::Cancel;
                    }
                    
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Tip: Click color squares to edit colors").weak().italics());
                    });
                });
            });
        });
    
    if !is_open {
        action = ThemeCreatorAction::Cancel;
    }
    
    if !matches!(action, ThemeCreatorAction::None) {
        state.close();
    }
    
    action
}

/// Render a single color picker row
fn render_color_picker(ui: &mut Ui, label: &str, color: &mut Color32) {
    ui.horizontal(|ui| {
        ui.label(format!("{}:", label));
        ui.color_edit_button_srgba(color);
        ui.label(CalendarTheme::color_to_hex(*color));
    });
    ui.add_space(5.0);
}

/// Render a preview of the theme
fn render_theme_preview(ui: &mut Ui, theme: &CalendarTheme) {
    ui.vertical(|ui| {
        // App background preview
        egui::Frame::none()
            .fill(theme.app_background)
            .stroke(egui::Stroke::new(1.0, theme.day_border))
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("App Background")
                        .color(theme.text_primary)
                );
                
                ui.add_space(5.0);
                
                // Calendar background preview
                egui::Frame::none()
                    .fill(theme.calendar_background)
                    .stroke(egui::Stroke::new(1.0, theme.day_border))
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Regular day
                            egui::Frame::none()
                                .fill(theme.day_background)
                                .stroke(egui::Stroke::new(1.0, theme.day_border))
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new("Mon")
                                                .color(theme.text_primary)
                                        );
                                        ui.label(
                                            RichText::new("15")
                                                .color(theme.text_secondary)
                                                .size(10.0)
                                        );
                                    });
                                });
                            
                            // Weekend day
                            egui::Frame::none()
                                .fill(theme.weekend_background)
                                .stroke(egui::Stroke::new(1.0, theme.day_border))
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new("Sat")
                                                .color(theme.text_primary)
                                        );
                                        ui.label(
                                            RichText::new("16")
                                                .color(theme.text_secondary)
                                                .size(10.0)
                                        );
                                    });
                                });
                            
                            // Today
                            egui::Frame::none()
                                .fill(theme.today_background)
                                .stroke(egui::Stroke::new(2.0, theme.today_border))
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new("Sun")
                                                .color(theme.text_primary)
                                        );
                                        ui.label(
                                            RichText::new("17")
                                                .color(theme.text_secondary)
                                                .size(10.0)
                                        );
                                    });
                                });
                        });
                    });
            });
    });
}
