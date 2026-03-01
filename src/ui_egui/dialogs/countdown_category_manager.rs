//! Countdown category management dialog for creating, editing, and deleting
//! countdown card categories (separate from event categories).

use crate::services::countdown::{
    CountdownCardVisuals, CountdownCategory, CountdownCategoryId, CountdownService,
    RgbaColor, DEFAULT_CATEGORY_ID,
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

/// State for the countdown category management dialog.
#[derive(Debug, Clone, Default)]
pub struct CountdownCategoryManagerState {
    /// Whether the dialog is open
    pub open: bool,
    /// Currently editing category (None = creating new)
    pub editing_category_id: Option<CountdownCategoryId>,
    /// Name input for new/edit category
    pub name_input: String,
    /// Display order input
    pub display_order_input: String,
    /// Error message to display
    pub error_message: Option<String>,
    /// Success message to display
    pub success_message: Option<String>,
    /// Category to delete (confirmation pending)
    pub delete_pending: Option<(CountdownCategoryId, String, usize)>,
    /// Working copy of the category's visual defaults.
    pub visual_defaults: CountdownCardVisuals,
    /// Working copy of the default card width.
    pub default_card_width: f32,
    /// Working copy of the default card height.
    pub default_card_height: f32,
    /// Working copy of the "use global defaults" toggle.
    pub use_global_defaults: bool,
    /// Whether the card-defaults section is expanded in the UI.
    pub defaults_expanded: bool,
}

impl CountdownCategoryManagerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog
    pub fn open(&mut self) {
        self.open = true;
        self.editing_category_id = None;
        self.clear_inputs();
        self.error_message = None;
        self.success_message = None;
        self.delete_pending = None;
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.open = false;
        self.editing_category_id = None;
        self.clear_inputs();
        self.delete_pending = None;
    }

    /// Start editing an existing category
    pub fn start_edit(&mut self, category: &CountdownCategory) {
        self.name_input = category.name.clone();
        self.display_order_input = category.display_order.to_string();
        self.editing_category_id = Some(category.id);
        self.visual_defaults = category.visual_defaults.clone();
        self.default_card_width = category.default_card_width;
        self.default_card_height = category.default_card_height;
        self.use_global_defaults = category.use_global_defaults;
        self.error_message = None;
        self.success_message = None;
    }

    /// Start creating a new category
    pub fn start_new(&mut self) {
        self.editing_category_id = None;
        self.clear_inputs();
        self.error_message = None;
        self.success_message = None;
    }

    /// Clear input fields
    fn clear_inputs(&mut self) {
        self.name_input.clear();
        self.display_order_input = "0".to_string();
        self.visual_defaults = CountdownCardVisuals::default();
        self.default_card_width = 120.0;
        self.default_card_height = 110.0;
        self.use_global_defaults = true;
        self.defaults_expanded = false;
    }
}

/// Response from the countdown category manager dialog.
#[derive(Debug, Default)]
pub struct CountdownCategoryManagerResponse {
    /// Whether categories were modified (callers should trigger a save)
    pub categories_changed: bool,
}

/// Render the countdown category management dialog.
///
/// This borrows the countdown service mutably so that category additions,
/// renames, reorders, and deletions are applied immediately in memory.
/// The caller is responsible for persisting changes when `categories_changed`
/// is true.
pub fn render_countdown_category_manager_dialog(
    ctx: &egui::Context,
    state: &mut CountdownCategoryManagerState,
    service: &mut CountdownService,
) -> CountdownCategoryManagerResponse {
    let mut response = CountdownCategoryManagerResponse::default();

    if !state.open {
        return response;
    }

    let mut dialog_open = state.open;

    egui::Window::new("📂 Countdown Categories")
        .open(&mut dialog_open)
        .collapsible(false)
        .resizable(true)
        .default_size([620.0, 520.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Error/success messages at top
            if let Some(ref error) = state.error_message {
                ui.colored_label(Color32::RED, format!("❌ {error}"));
                ui.add_space(4.0);
            }
            if let Some(ref success) = state.success_message {
                ui.colored_label(Color32::GREEN, format!("✓ {success}"));
                ui.add_space(4.0);
            }

            // Snapshot category data for rendering (avoids borrow conflicts)
            let category_snapshot: Vec<(CountdownCategoryId, String, i32, usize)> = service
                .categories()
                .iter()
                .map(|c| {
                    let count = service.cards_in_category(c.id).len();
                    (c.id, c.name.clone(), c.display_order, count)
                })
                .collect();

            ui.columns(2, |columns| {
                // LEFT: category list
                columns[0].vertical(|ui| {
                    ui.heading("Categories");
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
                                .id_source("countdown_category_list")
                                .max_height(220.0)
                                .min_scrolled_height(220.0)
                                .show(ui, |ui| {
                                    ui.set_min_width(160.0);

                                    for (id, name, _order, card_count) in &category_snapshot {
                                        let is_selected = state.editing_category_id == Some(*id);
                                        let is_default = id.0 == DEFAULT_CATEGORY_ID;

                                        ui.horizontal(|ui| {
                                            let label_text = if is_selected {
                                                RichText::new(name).strong()
                                            } else {
                                                RichText::new(name)
                                            };
                                            if ui.selectable_label(is_selected, label_text).clicked() {
                                                // Look up the category for editing
                                                if let Some(cat) = service
                                                    .categories()
                                                    .iter()
                                                    .find(|c| c.id == *id)
                                                {
                                                    state.start_edit(cat);
                                                }
                                            }

                                            ui.label(
                                                RichText::new(format!("({})", card_count))
                                                    .small()
                                                    .weak(),
                                            );

                                            if is_default {
                                                ui.label(
                                                    RichText::new("default").small().weak(),
                                                );
                                            }
                                        });
                                    }
                                });
                        });
                });

                // RIGHT: editor panel
                columns[1].vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .id_source("category_editor_scroll")
                        .max_height(ui.available_height() - 8.0)
                        .show(ui, |ui| {
                    let is_editing = state.editing_category_id.is_some();
                    let is_default = state
                        .editing_category_id
                        .map(|id| id.0 == DEFAULT_CATEGORY_ID)
                        .unwrap_or(false);

                    ui.heading(if is_editing {
                        "Edit Category"
                    } else {
                        "New Category"
                    });
                    ui.add_space(8.0);

                    // Name field
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add_enabled(
                            !is_default,
                            egui::TextEdit::singleline(&mut state.name_input)
                                .desired_width(130.0)
                                .hint_text("Category name"),
                        );
                    });
                    if is_default {
                        ui.label(
                            RichText::new("Default category name cannot be changed")
                                .small()
                                .weak(),
                        );
                    }

                    ui.add_space(4.0);

                    // Display order field
                    ui.horizontal(|ui| {
                        ui.label("Order:");
                        ui.add(
                            egui::TextEdit::singleline(&mut state.display_order_input)
                                .desired_width(50.0)
                                .hint_text("0"),
                        );
                        ui.label(RichText::new("(lower = first)").small().weak());
                    });

                    ui.add_space(12.0);

                    // Action buttons
                    let can_create = !state.name_input.trim().is_empty();
                    ui.horizontal(|ui| {
                        let button_text = if is_editing { "💾 Save" } else { "➕ Create" };
                        let button_enabled = is_editing || can_create;
                        if ui
                            .add_enabled(button_enabled, egui::Button::new(button_text))
                            .clicked()
                        {
                            handle_save(state, service, &mut response);
                        }

                        if is_editing && !is_default {
                            if let Some(editing_id) = state.editing_category_id {
                                if ui
                                    .button(
                                        RichText::new("🗑 Delete").color(Color32::LIGHT_RED),
                                    )
                                    .clicked()
                                {
                                    let card_count =
                                        service.cards_in_category(editing_id).len();
                                    let name = state.name_input.clone();
                                    state.delete_pending =
                                        Some((editing_id, name, card_count));
                                }
                            }
                        }

                        if is_editing && ui.button("Cancel").clicked() {
                            state.start_new();
                        }
                    });

                    // Delete confirmation
                    if let Some((del_id, ref del_name, del_count)) =
                        state.delete_pending.clone()
                    {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.colored_label(
                            Color32::YELLOW,
                            format!("⚠ Delete '{del_name}'?"),
                        );
                        if del_count > 0 {
                            ui.label(
                                RichText::new(format!(
                                    "{del_count} card(s) will be moved to General."
                                ))
                                .small(),
                            );
                        }

                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    RichText::new("Yes, Delete").color(Color32::RED),
                                )
                                .clicked()
                            {
                                if service.remove_category(del_id) {
                                    state.success_message =
                                        Some(format!("Deleted '{del_name}'"));
                                    response.categories_changed = true;
                                    state.start_new();
                                } else {
                                    state.error_message =
                                        Some("Failed to delete category".to_string());
                                }
                                state.delete_pending = None;
                            }
                            if ui.button("No").clicked() {
                                state.delete_pending = None;
                            }
                        });
                    }

                    // Card Defaults section (only when editing)
                    if is_editing {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);

                        let header = if state.defaults_expanded {
                            "▼ Card Defaults"
                        } else {
                            "▶ Card Defaults"
                        };
                        if ui.button(header).clicked() {
                            state.defaults_expanded = !state.defaults_expanded;
                        }

                        if state.defaults_expanded {
                            ui.add_space(4.0);

                            // Use global defaults toggle
                            ui.checkbox(
                                &mut state.use_global_defaults,
                                "Inherit global defaults",
                            );
                            ui.label(
                                RichText::new(
                                    "When enabled, cards use global visual defaults.",
                                )
                                .small()
                                .weak(),
                            );

                            ui.add_space(8.0);
                            ui.label(RichText::new("Card Dimensions").strong().size(13.0));
                            ui.add_space(2.0);

                            // Card width
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

                            // Card height
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

                            // Visual settings (disabled when inheriting globals)
                            ui.add_space(8.0);
                            ui.label(RichText::new("Colours & Fonts").strong().size(13.0));
                            if state.use_global_defaults {
                                ui.label(
                                    RichText::new(
                                        "Disable \"Inherit global defaults\" to customise.",
                                    )
                                    .small()
                                    .weak()
                                    .italics(),
                                );
                            }
                            ui.add_space(2.0);

                            let enabled = !state.use_global_defaults;

                            ui.add_enabled_ui(enabled, |ui| {
                                render_category_color_row(
                                    ui,
                                    "Title BG:",
                                    &mut state.visual_defaults.title_bg_color,
                                );
                                render_category_color_row(
                                    ui,
                                    "Title Text:",
                                    &mut state.visual_defaults.title_fg_color,
                                );
                                render_category_color_row(
                                    ui,
                                    "Body BG:",
                                    &mut state.visual_defaults.body_bg_color,
                                );
                                render_category_color_row(
                                    ui,
                                    "Days Text:",
                                    &mut state.visual_defaults.days_fg_color,
                                );

                                ui.add_space(4.0);

                                ui.horizontal(|ui| {
                                    ui.label("Title size:");
                                    ui.add(
                                        egui::Slider::new(
                                            &mut state.visual_defaults.title_font_size,
                                            10.0..=48.0,
                                        )
                                        .suffix(" pt"),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Days size:");
                                    ui.add(
                                        egui::Slider::new(
                                            &mut state.visual_defaults.days_font_size,
                                            32.0..=220.0,
                                        )
                                        .suffix(" pt"),
                                    );
                                });
                            });
                        }
                    }
                    }); // end ScrollArea
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
                    RichText::new(format!("{} categories", category_snapshot.len())).weak(),
                );
            });
        });

    if !dialog_open {
        state.close();
    }

    response
}

/// Handle save/create action.
fn handle_save(
    state: &mut CountdownCategoryManagerState,
    service: &mut CountdownService,
    response: &mut CountdownCategoryManagerResponse,
) {
    state.error_message = None;
    state.success_message = None;

    let name = state.name_input.trim();
    if name.is_empty() {
        state.error_message = Some("Name cannot be empty".to_string());
        return;
    }

    let display_order: i32 = state
        .display_order_input
        .trim()
        .parse()
        .unwrap_or(0);

    // Duplicate name check (excluding the currently-editing category)
    let duplicate = service.categories().iter().any(|c| {
        c.name.eq_ignore_ascii_case(name)
            && state
                .editing_category_id
                .map(|eid| eid != c.id)
                .unwrap_or(true)
    });
    if duplicate {
        state.error_message = Some("A category with this name already exists".to_string());
        return;
    }

    if let Some(editing_id) = state.editing_category_id {
        // Update existing
        if let Some(cat) = service.category_mut(editing_id) {
            let is_default = editing_id.0 == DEFAULT_CATEGORY_ID;
            if !is_default {
                cat.name = name.to_string();
            }
            cat.display_order = display_order;
            cat.visual_defaults = state.visual_defaults.clone();
            cat.default_card_width = state.default_card_width;
            cat.default_card_height = state.default_card_height;
            cat.use_global_defaults = state.use_global_defaults;
            state.success_message = Some("Category updated".to_string());
            response.categories_changed = true;
        } else {
            state.error_message = Some("Category not found".to_string());
        }
    } else {
        // Create new
        let new_cat = CountdownCategory {
            id: CountdownCategoryId(0), // add_category assigns proper ID
            name: name.to_string(),
            display_order,
            visual_defaults: state.visual_defaults.clone(),
            default_card_width: state.default_card_width,
            default_card_height: state.default_card_height,
            use_global_defaults: state.use_global_defaults,
            ..CountdownCategory::default()
        };
        let added = service.add_category(new_cat);
        let added_id = added.id;
        state.success_message = Some(format!("Created '{name}'"));
        response.categories_changed = true;

        // Select the new category
        if let Some(cat) = service.categories().iter().find(|c| c.id == added_id) {
            state.start_edit(cat);
        }
    }
}

/// Render a labelled colour-picker row for a category default colour.
fn render_category_color_row(ui: &mut egui::Ui, label: &str, color: &mut RgbaColor) {
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
