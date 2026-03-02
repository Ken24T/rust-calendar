//! Countdown card template management dialog for creating, editing, and
//! deleting reusable visual templates (colours, fonts, default dimensions).

use crate::services::countdown::{
    CountdownCardTemplate, CountdownCardTemplateId, CountdownCardVisuals, CountdownService,
    RgbaColor, DEFAULT_TEMPLATE_ID,
};
use egui::{Color32, RichText};

/// Convert an [`RgbaColor`] to an egui [`Color32`].
fn rgba_to_color32(c: RgbaColor) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

/// Convert an egui [`Color32`] to an [`RgbaColor`].
fn color32_to_rgba(c: Color32) -> RgbaColor {
    RgbaColor {
        r: c.r(),
        g: c.g(),
        b: c.b(),
        a: c.a(),
    }
}

/// State for the countdown template management dialog.
#[derive(Debug, Clone, Default)]
pub struct CountdownTemplateManagerState {
    /// Whether the dialog is open.
    pub open: bool,
    /// Currently editing template (None = creating new).
    pub editing_template_id: Option<CountdownCardTemplateId>,
    /// Working copy of the template name.
    pub name_input: String,
    /// Working copy of the template visuals.
    pub visuals: CountdownCardVisuals,
    /// Working copy of the default card width.
    pub default_card_width: f32,
    /// Working copy of the default card height.
    pub default_card_height: f32,
    /// Error message to display.
    pub error_message: Option<String>,
    /// Success message to display.
    pub success_message: Option<String>,
    /// Template to delete (confirmation pending).
    pub delete_pending: Option<(CountdownCardTemplateId, String)>,
}

impl CountdownTemplateManagerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog.
    pub fn open(&mut self) {
        self.open = true;
        self.editing_template_id = None;
        self.clear_inputs();
        self.error_message = None;
        self.success_message = None;
        self.delete_pending = None;
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.open = false;
        self.editing_template_id = None;
        self.clear_inputs();
        self.delete_pending = None;
    }

    /// Start editing an existing template.
    pub fn start_edit(&mut self, tmpl: &CountdownCardTemplate) {
        self.editing_template_id = Some(tmpl.id);
        self.name_input = tmpl.name.clone();
        self.visuals = tmpl.visuals.clone();
        self.default_card_width = tmpl.default_card_width;
        self.default_card_height = tmpl.default_card_height;
        self.error_message = None;
        self.success_message = None;
    }

    /// Start creating a new template.
    pub fn start_new(&mut self) {
        self.editing_template_id = None;
        self.clear_inputs();
        self.error_message = None;
        self.success_message = None;
    }

    /// Clear input fields.
    fn clear_inputs(&mut self) {
        self.name_input.clear();
        self.visuals = CountdownCardVisuals::default();
        self.default_card_width = 120.0;
        self.default_card_height = 110.0;
    }
}

/// Response from the countdown template manager dialog.
#[derive(Debug, Default)]
pub struct CountdownTemplateManagerResponse {
    /// Whether templates were modified (callers should trigger a save).
    pub templates_changed: bool,
}

/// Render the countdown template management dialog.
pub fn render_countdown_template_manager_dialog(
    ctx: &egui::Context,
    state: &mut CountdownTemplateManagerState,
    service: &mut CountdownService,
) -> CountdownTemplateManagerResponse {
    let mut response = CountdownTemplateManagerResponse::default();

    if !state.open {
        return response;
    }

    let mut dialog_open = state.open;

    egui::Window::new("🎨 Card Templates")
        .open(&mut dialog_open)
        .collapsible(false)
        .resizable(true)
        .default_size([580.0, 480.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Error/success messages
            if let Some(ref error) = state.error_message {
                ui.colored_label(Color32::RED, format!("❌ {error}"));
                ui.add_space(4.0);
            }
            if let Some(ref success) = state.success_message {
                ui.colored_label(Color32::GREEN, format!("✓ {success}"));
                ui.add_space(4.0);
            }

            // Snapshot template list for rendering
            let template_snapshot: Vec<(CountdownCardTemplateId, String)> = service
                .templates()
                .iter()
                .map(|t| (t.id, t.name.clone()))
                .collect();

            ui.columns(2, |columns| {
                // LEFT: template list
                columns[0].vertical(|ui| {
                    ui.heading("Templates");
                    ui.add_space(4.0);

                    egui::Frame::none()
                        .stroke(egui::Stroke::new(
                            1.0,
                            ui.visuals().widgets.noninteractive.bg_stroke.color,
                        ))
                        .rounding(4.0)
                        .inner_margin(4.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_source("countdown_template_list")
                                .max_height(200.0)
                                .min_scrolled_height(200.0)
                                .show(ui, |ui| {
                                    ui.set_min_width(140.0);

                                    for (id, name) in &template_snapshot {
                                        let is_selected =
                                            state.editing_template_id == Some(*id);
                                        let is_default = id.0 == DEFAULT_TEMPLATE_ID;

                                        ui.horizontal(|ui| {
                                            let label_text = if is_selected {
                                                RichText::new(name).strong()
                                            } else {
                                                RichText::new(name)
                                            };
                                            if ui
                                                .selectable_label(is_selected, label_text)
                                                .clicked()
                                            {
                                                if let Some(tmpl) = service
                                                    .templates()
                                                    .iter()
                                                    .find(|t| t.id == *id)
                                                {
                                                    state.start_edit(tmpl);
                                                }
                                            }

                                            if is_default {
                                                ui.label(
                                                    RichText::new("built-in")
                                                        .small()
                                                        .weak(),
                                                );
                                            }
                                        });
                                    }
                                });
                        });

                    ui.add_space(4.0);
                    if ui.button("➕ New Template").clicked() {
                        state.start_new();
                    }
                });

                // RIGHT: editor panel
                columns[1].vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .id_source("template_editor_scroll")
                        .max_height(ui.available_height() - 8.0)
                        .show(ui, |ui| {
                            let is_editing = state.editing_template_id.is_some();

                            ui.heading(if is_editing {
                                "Edit Template"
                            } else {
                                "New Template"
                            });
                            ui.add_space(8.0);

                            // Name field
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut state.name_input)
                                        .desired_width(140.0)
                                        .hint_text("Template name"),
                                );
                            });

                            ui.add_space(8.0);

                            // Colours
                            ui.label(RichText::new("Colours").strong().size(13.0));
                            ui.add_space(2.0);
                            render_color_row(
                                ui,
                                "Title BG:",
                                &mut state.visuals.title_bg_color,
                            );
                            render_color_row(
                                ui,
                                "Title Text:",
                                &mut state.visuals.title_fg_color,
                            );
                            render_color_row(
                                ui,
                                "Body BG:",
                                &mut state.visuals.body_bg_color,
                            );
                            render_color_row(
                                ui,
                                "Days Text:",
                                &mut state.visuals.days_fg_color,
                            );

                            ui.add_space(8.0);

                            // Fonts
                            ui.label(RichText::new("Fonts").strong().size(13.0));
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label("Title size:");
                                ui.add(
                                    egui::Slider::new(
                                        &mut state.visuals.title_font_size,
                                        10.0..=48.0,
                                    )
                                    .suffix(" pt"),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Days size:");
                                ui.add(
                                    egui::Slider::new(
                                        &mut state.visuals.days_font_size,
                                        32.0..=220.0,
                                    )
                                    .suffix(" pt"),
                                );
                            });

                            ui.add_space(8.0);

                            // Default card dimensions
                            ui.label(
                                RichText::new("Default Card Dimensions").strong().size(13.0),
                            );
                            ui.label(
                                RichText::new("Categories can override these.")
                                    .small()
                                    .weak(),
                            );
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label("Width:");
                                ui.add(
                                    egui::Slider::new(
                                        &mut state.default_card_width,
                                        60.0..=400.0,
                                    )
                                    .suffix(" px"),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Height:");
                                ui.add(
                                    egui::Slider::new(
                                        &mut state.default_card_height,
                                        60.0..=400.0,
                                    )
                                    .suffix(" px"),
                                );
                            });

                            ui.add_space(12.0);

                            // Action buttons
                            let can_create = !state.name_input.trim().is_empty();
                            ui.horizontal(|ui| {
                                let button_text =
                                    if is_editing { "💾 Save" } else { "➕ Create" };
                                let button_enabled = is_editing || can_create;
                                if ui
                                    .add_enabled(
                                        button_enabled,
                                        egui::Button::new(button_text),
                                    )
                                    .clicked()
                                {
                                    handle_template_save(state, service, &mut response);
                                }

                                if is_editing {
                                    let is_default_tmpl = state
                                        .editing_template_id
                                        .map(|id| id.0 == DEFAULT_TEMPLATE_ID)
                                        .unwrap_or(false);

                                    if !is_default_tmpl {
                                        if let Some(eid) = state.editing_template_id {
                                            if ui
                                                .button(
                                                    RichText::new("🗑 Delete")
                                                        .color(Color32::LIGHT_RED),
                                                )
                                                .clicked()
                                            {
                                                let name = state.name_input.clone();
                                                state.delete_pending = Some((eid, name));
                                            }
                                        }
                                    }

                                    if ui.button("Cancel").clicked() {
                                        state.start_new();
                                    }
                                }
                            });

                            // Delete confirmation
                            if let Some((del_id, ref del_name)) =
                                state.delete_pending.clone()
                            {
                                ui.add_space(8.0);
                                ui.separator();
                                ui.colored_label(
                                    Color32::YELLOW,
                                    format!("⚠ Delete '{del_name}'?"),
                                );
                                ui.label(
                                    RichText::new(
                                        "Categories using this template will fall back to global defaults.",
                                    )
                                    .small(),
                                );

                                ui.horizontal(|ui| {
                                    if ui
                                        .button(
                                            RichText::new("Yes, Delete")
                                                .color(Color32::RED),
                                        )
                                        .clicked()
                                    {
                                        if service.remove_template(del_id) {
                                            state.success_message =
                                                Some(format!("Deleted '{del_name}'"));
                                            response.templates_changed = true;
                                            state.start_new();
                                        } else {
                                            state.error_message = Some(
                                                "Failed to delete template".to_string(),
                                            );
                                        }
                                        state.delete_pending = None;
                                    }
                                    if ui.button("No").clicked() {
                                        state.delete_pending = None;
                                    }
                                });
                            }
                        });
                });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // Footer
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    state.close();
                }
                ui.add_space(20.0);
                ui.label(
                    RichText::new(format!("{} templates", template_snapshot.len())).weak(),
                );
            });
        });

    if !dialog_open {
        state.close();
    }

    response
}

/// Handle save/create action for templates.
fn handle_template_save(
    state: &mut CountdownTemplateManagerState,
    service: &mut CountdownService,
    response: &mut CountdownTemplateManagerResponse,
) {
    state.error_message = None;
    state.success_message = None;

    let name = state.name_input.trim();
    if name.is_empty() {
        state.error_message = Some("Name cannot be empty".to_string());
        return;
    }

    // Duplicate name check (excluding the currently-editing template)
    let duplicate = service.templates().iter().any(|t| {
        t.name.eq_ignore_ascii_case(name)
            && state
                .editing_template_id
                .map(|eid| eid != t.id)
                .unwrap_or(true)
    });
    if duplicate {
        state.error_message = Some("A template with this name already exists".to_string());
        return;
    }

    if let Some(editing_id) = state.editing_template_id {
        // Update existing
        if let Some(tmpl) = service.template_mut(editing_id) {
            tmpl.name = name.to_string();
            tmpl.visuals = state.visuals.clone();
            tmpl.default_card_width = state.default_card_width;
            tmpl.default_card_height = state.default_card_height;
            state.success_message = Some("Template updated".to_string());
            response.templates_changed = true;
        } else {
            state.error_message = Some("Template not found".to_string());
        }
    } else {
        // Create new
        let new_tmpl = CountdownCardTemplate {
            id: CountdownCardTemplateId(0), // add_template assigns proper ID
            name: name.to_string(),
            visuals: state.visuals.clone(),
            default_card_width: state.default_card_width,
            default_card_height: state.default_card_height,
        };
        let added = service.add_template(new_tmpl);
        let added_id = added.id;
        state.success_message = Some(format!("Created '{name}'"));
        response.templates_changed = true;

        // Select the new template
        if let Some(tmpl) = service.templates().iter().find(|t| t.id == added_id) {
            state.start_edit(tmpl);
        }
    }
}

/// Render a labelled colour-picker row.
fn render_color_row(ui: &mut egui::Ui, label: &str, color: &mut RgbaColor) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut c32 = rgba_to_color32(*color);
        if egui::color_picker::color_edit_button_srgba(
            ui,
            &mut c32,
            egui::color_picker::Alpha::Opaque,
        )
        .changed()
        {
            *color = color32_to_rgba(c32);
        }
    });
}
