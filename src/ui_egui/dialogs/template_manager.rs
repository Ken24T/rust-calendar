// Template Manager Dialog
// UI for managing event templates (create, edit, delete)

use egui::{Color32, RichText};

use crate::models::settings::Settings;
use crate::models::template::EventTemplate;
use crate::services::category::CategoryService;
use crate::services::database::Database;
use crate::services::template::TemplateService;

/// State for the template manager dialog
#[derive(Default)]
pub struct TemplateManagerState {
    pub is_open: bool,
    pub templates: Vec<EventTemplate>,
    pub selected_template_id: Option<i64>,
    pub editing_template: Option<TemplateEditState>,
    pub error_message: Option<String>,
    pub needs_refresh: bool,
}

/// State for editing a single template
pub struct TemplateEditState {
    pub id: Option<i64>,
    pub name: String,
    pub title: String,
    pub description: String,
    pub location: String,
    pub duration_hours: u32,
    pub duration_minutes: u32,
    pub all_day: bool,
    pub category: String,
    pub color: String,
}

impl TemplateEditState {
    pub fn new() -> Self {
        Self {
            id: None,
            name: String::new(),
            title: String::new(),
            description: String::new(),
            location: String::new(),
            duration_hours: 1,
            duration_minutes: 0,
            all_day: false,
            category: String::new(),
            color: "#3B82F6".to_string(),
        }
    }

    /// Create a new template edit state with the given default duration in minutes
    pub fn new_with_duration(default_duration_minutes: u32) -> Self {
        Self {
            id: None,
            name: String::new(),
            title: String::new(),
            description: String::new(),
            location: String::new(),
            duration_hours: default_duration_minutes / 60,
            duration_minutes: default_duration_minutes % 60,
            all_day: false,
            category: String::new(),
            color: "#3B82F6".to_string(),
        }
    }

    pub fn from_template(template: &EventTemplate) -> Self {
        let total_minutes = template.duration_minutes as u32;
        let hours = total_minutes / 60;
        let minutes = total_minutes % 60;

        Self {
            id: template.id,
            name: template.name.clone(),
            title: template.title.clone(),
            description: template.description.clone().unwrap_or_default(),
            location: template.location.clone().unwrap_or_default(),
            duration_hours: hours,
            duration_minutes: minutes,
            all_day: template.all_day,
            category: template.category.clone().unwrap_or_default(),
            color: template.color.clone().unwrap_or_else(|| "#3B82F6".to_string()),
        }
    }

    pub fn to_template(&self) -> EventTemplate {
        let duration_minutes = (self.duration_hours * 60 + self.duration_minutes) as i32;

        EventTemplate {
            id: self.id,
            name: self.name.clone(),
            title: self.title.clone(),
            description: if self.description.is_empty() { None } else { Some(self.description.clone()) },
            location: if self.location.is_empty() { None } else { Some(self.location.clone()) },
            duration_minutes: if self.all_day { 24 * 60 } else { duration_minutes.max(1) },
            all_day: self.all_day,
            category: if self.category.is_empty() { None } else { Some(self.category.clone()) },
            color: if self.color.is_empty() { None } else { Some(self.color.clone()) },
            recurrence_rule: None, // Not editing recurrence in templates for now
            created_at: None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Template name is required".to_string());
        }
        if self.title.trim().is_empty() {
            return Err("Event title is required".to_string());
        }
        if !self.all_day && self.duration_hours == 0 && self.duration_minutes == 0 {
            return Err("Duration must be at least 1 minute".to_string());
        }
        Ok(())
    }
}

impl Default for TemplateEditState {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateManagerState {
    pub fn open(&mut self, database: &Database) {
        self.is_open = true;
        self.refresh_templates(database);
        self.selected_template_id = None;
        self.editing_template = None;
        self.error_message = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.editing_template = None;
        self.error_message = None;
    }

    pub fn refresh_templates(&mut self, database: &Database) {
        let service = TemplateService::new(database.connection());
        match service.list_all() {
            Ok(templates) => {
                self.templates = templates;
                self.needs_refresh = false;
            }
            Err(e) => {
                log::error!("Failed to load templates: {}", e);
                self.error_message = Some(format!("Failed to load templates: {}", e));
            }
        }
    }

    pub fn start_new_template(&mut self, default_duration_minutes: u32) {
        self.editing_template = Some(TemplateEditState::new_with_duration(default_duration_minutes));
        self.error_message = None;
    }

    pub fn start_edit_template(&mut self, template: &EventTemplate) {
        self.editing_template = Some(TemplateEditState::from_template(template));
        self.error_message = None;
    }
}

const FORM_LABEL_WIDTH: f32 = 120.0;

pub fn render_template_manager_dialog(
    ctx: &egui::Context,
    state: &mut TemplateManagerState,
    database: &Database,
    settings: &Settings,
) {
    if !state.is_open {
        return;
    }

    if state.needs_refresh {
        state.refresh_templates(database);
    }

    let mut dialog_open = state.is_open;
    let default_duration = settings.default_event_duration;

    egui::Window::new("📋 Manage Templates")
        .open(&mut dialog_open)
        .collapsible(false)
        .resizable(true)
        .default_width(650.0)
        .default_height(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if state.editing_template.is_some() {
                render_edit_form(ui, state, database);
            } else {
                render_template_list(ui, state, database, default_duration);
            }
        });

    if !dialog_open {
        state.close();
    }
}

fn render_template_list(
    ui: &mut egui::Ui,
    state: &mut TemplateManagerState,
    database: &Database,
    default_duration: u32,
) {
    // Error message
    if let Some(ref error) = state.error_message {
        ui.colored_label(Color32::RED, error);
        ui.add_space(8.0);
    }

    // Toolbar
    ui.horizontal(|ui| {
        if ui.button("➕ New Template").clicked() {
            state.start_new_template(default_duration);
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    if state.templates.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(RichText::new("No templates yet").weak());
            ui.add_space(8.0);
            ui.label("Create a template to quickly add common events.");
            ui.add_space(8.0);
            if ui.button("Create your first template").clicked() {
                state.start_new_template(default_duration);
            }
            ui.add_space(40.0);
        });
        return;
    }

    // Template list
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for template in state.templates.clone() {
                let is_selected = state.selected_template_id == template.id;
                
                egui::Frame::none()
                    .fill(if is_selected {
                        ui.visuals().selection.bg_fill
                    } else {
                        Color32::TRANSPARENT
                    })
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Color indicator
                            if let Some(ref color) = template.color {
                                if let Some(c) = parse_hex_color(color) {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(4.0, 32.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 2.0, c);
                                }
                            }

                            ui.add_space(8.0);

                            // Template info
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&template.name).strong());
                                    ui.label(">");
                                    ui.label(&template.title);
                                });
                                
                                ui.horizontal(|ui| {
                                    if template.all_day {
                                        ui.label(RichText::new("All-day").weak().small());
                                    } else {
                                        let hours = template.duration_minutes / 60;
                                        let mins = template.duration_minutes % 60;
                                        let duration = if hours > 0 && mins > 0 {
                                            format!("{}h {}m", hours, mins)
                                        } else if hours > 0 {
                                            format!("{}h", hours)
                                        } else {
                                            format!("{}m", mins)
                                        };
                                        ui.label(RichText::new(duration).weak().small());
                                    }
                                    
                                    if let Some(ref cat) = template.category {
                                        ui.label(RichText::new(format!("• {}", cat)).weak().small());
                                    }
                                    
                                    if let Some(ref loc) = template.location {
                                        if !loc.is_empty() {
                                            ui.label(RichText::new(format!("📍 {}", loc)).weak().small());
                                        }
                                    }
                                });
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("🗑").on_hover_text("Delete").clicked() {
                                    let service = TemplateService::new(database.connection());
                                    if let Some(id) = template.id {
                                        if let Err(e) = service.delete(id) {
                                            state.error_message = Some(format!("Delete failed: {}", e));
                                        } else {
                                            state.needs_refresh = true;
                                        }
                                    }
                                }
                                
                                if ui.button("✏").on_hover_text("Edit").clicked() {
                                    state.start_edit_template(&template);
                                }
                            });
                        });
                    });

                // Click to select
                let response = ui.interact(
                    ui.min_rect(),
                    ui.id().with(template.id),
                    egui::Sense::click(),
                );
                if response.clicked() {
                    state.selected_template_id = template.id;
                }
            }
        });
}

fn render_edit_form(
    ui: &mut egui::Ui,
    state: &mut TemplateManagerState,
    database: &Database,
) {
    let Some(editing) = state.editing_template.as_mut() else {
        return;
    };
    
    let is_new = editing.id.is_none();

    ui.heading(if is_new { "New Template" } else { "Edit Template" });
    ui.add_space(8.0);

    // Error message
    if let Some(ref error) = state.error_message {
        ui.colored_label(Color32::RED, error);
        ui.add_space(8.0);
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // Template name
            labeled_row(ui, "Template Name:", |ui| {
                ui.text_edit_singleline(&mut editing.name);
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Event title
            labeled_row(ui, "Event Title:", |ui| {
                ui.text_edit_singleline(&mut editing.title);
            });

            // Location
            labeled_row(ui, "Location:", |ui| {
                ui.text_edit_singleline(&mut editing.location);
            });

            // Category
            labeled_row(ui, "Category:", |ui| {
                let cat_service = CategoryService::new(database.connection());
                let categories = cat_service.list_all().unwrap_or_default();

                let selected_display = if editing.category.is_empty() {
                    "None".to_string()
                } else {
                    categories
                        .iter()
                        .find(|c| c.name == editing.category)
                        .map(|c| c.display_name())
                        .unwrap_or_else(|| editing.category.clone())
                };

                let selected_color = categories
                    .iter()
                    .find(|c| c.name == editing.category)
                    .and_then(|c| parse_hex_color(&c.color))
                    .unwrap_or(Color32::TRANSPARENT);

                ui.horizontal(|ui| {
                    if !editing.category.is_empty() {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 3.0, selected_color);
                    }

                    egui::ComboBox::from_id_source("template_category_dropdown")
                        .selected_text(&selected_display)
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            let is_none = editing.category.is_empty();
                            if ui.selectable_label(is_none, "None").clicked() {
                                editing.category.clear();
                            }

                            ui.separator();

                            for cat in &categories {
                                let is_selected = cat.name == editing.category;
                                ui.horizontal(|ui| {
                                    let color =
                                        parse_hex_color(&cat.color).unwrap_or(Color32::GRAY);
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(12.0, 12.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 2.0, color);

                                    if ui
                                        .selectable_label(is_selected, cat.display_name())
                                        .clicked()
                                    {
                                        editing.category = cat.name.clone();
                                    }
                                });
                            }
                        });
                });
            });

            // Description
            labeled_row(ui, "Description:", |ui| {
                ui.add_sized(
                    [ui.available_width(), 60.0],
                    egui::TextEdit::multiline(&mut editing.description),
                );
            });

            ui.add_space(8.0);

            // All-day checkbox
            labeled_row(ui, "", |ui| {
                ui.checkbox(&mut editing.all_day, "All-day event");
            });

            // Duration (only if not all-day)
            if !editing.all_day {
                labeled_row(ui, "Duration:", |ui| {
                    ui.add(egui::DragValue::new(&mut editing.duration_hours).range(0..=23));
                    ui.label("hours");
                    ui.add(egui::DragValue::new(&mut editing.duration_minutes).range(0..=59));
                    ui.label("minutes");
                });
            }

            ui.add_space(8.0);

            // Color
            labeled_row(ui, "Color:", |ui| {
                ui.add(egui::TextEdit::singleline(&mut editing.color).desired_width(80.0));

                if let Some(mut color) = parse_hex_color(&editing.color) {
                    ui.color_edit_button_srgba(&mut color);
                    editing.color = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
                }
            });

            // Color presets
            labeled_row(ui, "Presets:", |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (name, hex) in &[
                        ("Blue", "#3B82F6"),
                        ("Green", "#10B981"),
                        ("Red", "#EF4444"),
                        ("Yellow", "#F59E0B"),
                        ("Purple", "#8B5CF6"),
                        ("Pink", "#EC4899"),
                    ] {
                        if ui.button(*name).clicked() {
                            editing.color = hex.to_string();
                        }
                    }
                });
            });
        });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    // Collect values before action buttons to avoid borrow conflicts
    let editing = state.editing_template.as_ref().unwrap();
    let validation_result = editing.validate();
    let template_to_save = editing.to_template();
    let is_new = editing.id.is_none();
    let save_text = if is_new { "Create" } else { "Save" };

    // Action buttons
    let mut should_save = false;
    let mut should_cancel = false;

    ui.horizontal(|ui| {
        if ui.button(save_text).clicked() {
            should_save = true;
        }
        if ui.button("Cancel").clicked() {
            should_cancel = true;
        }
    });

    // Handle actions after UI rendering
    if should_save {
        if let Err(e) = validation_result {
            state.error_message = Some(e);
        } else {
            let service = TemplateService::new(database.connection());
            
            // Check for duplicate name
            if service.name_exists(&template_to_save.name, template_to_save.id).unwrap_or(false) {
                state.error_message = Some("A template with this name already exists".to_string());
            } else {
                let result = if is_new {
                    service.create(template_to_save)
                } else {
                    service.update(&template_to_save).map(|_| template_to_save)
                };

                match result {
                    Ok(_) => {
                        state.editing_template = None;
                        state.needs_refresh = true;
                        state.error_message = None;
                    }
                    Err(e) => {
                        state.error_message = Some(format!("Save failed: {}", e));
                    }
                }
            }
        }
    }

    if should_cancel {
        state.editing_template = None;
        state.error_message = None;
    }
}

fn labeled_row<F>(ui: &mut egui::Ui, label: &str, add_contents: F)
where
    F: FnOnce(&mut egui::Ui),
{
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::Vec2::new(FORM_LABEL_WIDTH, 24.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label(label);
            },
        );
        add_contents(ui);
    });
}

fn parse_hex_color(hex: &str) -> Option<Color32> {
    if !hex.starts_with('#') {
        return None;
    }
    
    let hex = &hex[1..];
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else if hex.len() == 3 {
        let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
        Some(Color32::from_rgb(r, g, b))
    } else {
        None
    }
}
