//! Category management dialog for creating, editing, and deleting event categories.

use crate::models::category::Category;
use crate::services::category::CategoryService;
use crate::services::database::Database;
use egui::{Color32, RichText};

/// State for the category management dialog.
#[derive(Debug, Clone, Default)]
pub struct CategoryManagerState {
    /// Whether the dialog is open
    pub open: bool,
    /// Currently editing category (None = creating new)
    pub editing_category: Option<Category>,
    /// Name input for new/edit category
    pub name_input: String,
    /// Color input for new/edit category
    pub color_input: String,
    /// Icon input for new/edit category
    pub icon_input: String,
    /// Error message to display
    pub error_message: Option<String>,
    /// Success message to display
    pub success_message: Option<String>,
    /// Category to delete (confirmation pending)
    pub delete_pending: Option<(i64, String, i32)>, // (id, name, usage_count)
    /// Flag to refresh category list
    pub needs_refresh: bool,
}

impl CategoryManagerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog
    pub fn open(&mut self) {
        self.open = true;
        self.editing_category = None;
        self.clear_inputs();
        self.error_message = None;
        self.success_message = None;
        self.delete_pending = None;
        self.needs_refresh = true;
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.open = false;
        self.editing_category = None;
        self.clear_inputs();
        self.delete_pending = None;
    }

    /// Start editing an existing category
    pub fn start_edit(&mut self, category: Category) {
        self.name_input = category.name.clone();
        self.color_input = category.color.clone();
        self.icon_input = category.icon.clone().unwrap_or_default();
        self.editing_category = Some(category);
        self.error_message = None;
        self.success_message = None;
    }

    /// Start creating a new category
    pub fn start_new(&mut self) {
        self.editing_category = None;
        self.clear_inputs();
        self.error_message = None;
        self.success_message = None;
    }

    /// Clear input fields
    fn clear_inputs(&mut self) {
        self.name_input.clear();
        self.color_input = "#3B82F6".to_string(); // Default blue
        self.icon_input.clear();
    }
}

/// Response from the category manager dialog.
#[derive(Debug, Default)]
pub struct CategoryManagerResponse {
    /// Whether categories were modified
    pub categories_changed: bool,
}

/// Render the category management dialog.
pub fn render_category_manager_dialog(
    ctx: &egui::Context,
    state: &mut CategoryManagerState,
    database: &Database,
) -> CategoryManagerResponse {
    let mut response = CategoryManagerResponse::default();

    if !state.open {
        return response;
    }

    let mut dialog_open = state.open;
    let service = CategoryService::new(database.connection());

    // Load categories
    let categories = service.list_all().unwrap_or_default();

    egui::Window::new("📂 Manage Categories")
        .open(&mut dialog_open)
        .collapsible(false)
        .resizable(false)
        .fixed_size([550.0, 380.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Error/success messages at top
            if let Some(ref error) = state.error_message {
                ui.colored_label(Color32::RED, format!("❌ {}", error));
                ui.add_space(4.0);
            }
            if let Some(ref success) = state.success_message {
                ui.colored_label(Color32::GREEN, format!("✓ {}", success));
                ui.add_space(4.0);
            }

            // Main content using columns for proper side-by-side layout
            ui.columns(2, |columns| {
                // LEFT COLUMN: Category list
                columns[0].vertical(|ui| {
                    ui.heading("Categories");
                    ui.add_space(4.0);

                    // Frame for the category list with fixed height
                    egui::Frame::none()
                        .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
                        .rounding(4.0)
                        .inner_margin(4.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_source("category_list")
                                .max_height(220.0)
                                .min_scrolled_height(220.0)
                                .show(ui, |ui| {
                                    ui.set_min_width(180.0);
                                    
                                    for cat in &categories {
                                        let is_selected = state.editing_category.as_ref()
                                            .map(|e| e.id == cat.id)
                                            .unwrap_or(false);

                                        ui.horizontal(|ui| {
                                            // Color swatch
                                            let color = parse_hex_color(&cat.color);
                                            let (rect, _) = ui.allocate_exact_size(
                                                egui::vec2(14.0, 14.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(rect, 2.0, color);

                                            // Category name with icon
                                            let display = cat.display_name();
                                            let text = if is_selected {
                                                RichText::new(&display).strong()
                                            } else {
                                                RichText::new(&display)
                                            };

                                            if ui.selectable_label(is_selected, text).clicked() {
                                                state.start_edit(cat.clone());
                                            }

                                            // System badge
                                            if cat.is_system {
                                                ui.label(RichText::new("(system)").small().weak());
                                            }
                                        });
                                    }
                                });
                        });
                });

                // RIGHT COLUMN: Editor
                columns[1].vertical(|ui| {
                    let is_editing = state.editing_category.is_some();
                    let is_system = state.editing_category.as_ref()
                        .map(|c| c.is_system)
                        .unwrap_or(false);

                    ui.heading(if is_editing { "Edit Category" } else { "New Category" });
                    ui.add_space(8.0);

                    // Name field
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add_enabled(
                            !is_system,
                            egui::TextEdit::singleline(&mut state.name_input)
                                .desired_width(140.0)
                                .hint_text(if is_editing { "" } else { "Enter name" }),
                        );
                    });
                    if is_system {
                        ui.label(RichText::new("System names cannot be changed").small().weak());
                    }

                    ui.add_space(4.0);

                    // Color picker
                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        ui.add(
                            egui::TextEdit::singleline(&mut state.color_input)
                                .desired_width(70.0)
                                .hint_text("#RRGGBB"),
                        );
                        // Color preview/picker - clickable for custom categories
                        let preview_color = parse_hex_color(&state.color_input);
                        if !is_system {
                            // Interactive color button that opens color picker
                            let mut color_arr = [
                                preview_color.r() as f32 / 255.0,
                                preview_color.g() as f32 / 255.0,
                                preview_color.b() as f32 / 255.0,
                            ];
                            if ui.color_edit_button_rgb(&mut color_arr).changed() {
                                state.color_input = format!(
                                    "#{:02X}{:02X}{:02X}",
                                    (color_arr[0] * 255.0) as u8,
                                    (color_arr[1] * 255.0) as u8,
                                    (color_arr[2] * 255.0) as u8,
                                );
                            }
                        } else {
                            // Just show preview for system categories
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(20.0, 20.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 3.0, preview_color);
                        }
                    });

                    // Color presets
                    ui.horizontal(|ui| {
                        ui.label("Presets:");
                        let presets = [
                            "#3B82F6", "#10B981", "#F59E0B",
                            "#EF4444", "#8B5CF6", "#EC4899",
                        ];
                        for hex in presets {
                            let color = parse_hex_color(hex);
                            if ui.add(egui::Button::new("").fill(color).min_size(egui::vec2(18.0, 18.0))).clicked() {
                                state.color_input = hex.to_string();
                            }
                        }
                    });

                    ui.add_space(4.0);

                    // Icon field
                    ui.horizontal(|ui| {
                        ui.label("Icon:");
                        ui.add(
                            egui::TextEdit::singleline(&mut state.icon_input)
                                .desired_width(50.0)
                                .hint_text("emoji"),
                        );
                        ui.label(RichText::new("(optional)").small().weak());
                    });

                    // Icon presets
                    ui.horizontal(|ui| {
                        ui.label("Presets:");
                        let icons = ["💼", "🏠", "🎂", "🎉", "👥", "⏰", "📅", "✈️"];
                        for icon in icons {
                            if ui.small_button(icon).clicked() {
                                state.icon_input = icon.to_string();
                            }
                        }
                    });

                    ui.add_space(12.0);

                    // Action buttons
                    let can_create = !state.name_input.trim().is_empty();
                    ui.horizontal(|ui| {
                        let button_text = if is_editing { "💾 Save" } else { "➕ Create" };
                        let button_enabled = is_editing || can_create;
                        if ui.add_enabled(button_enabled, egui::Button::new(button_text)).clicked() {
                            handle_save(state, &service, &mut response);
                        }

                        if is_editing && !is_system {
                            if let Some(ref cat) = state.editing_category {
                                if let Some(id) = cat.id {
                                    if ui.button(RichText::new("🗑 Delete").color(Color32::LIGHT_RED)).clicked() {
                                        let usage = service.get_usage_count(&cat.name).unwrap_or(0);
                                        state.delete_pending = Some((id, cat.name.clone(), usage));
                                    }
                                }
                            }
                        }

                        if is_editing {
                            if ui.button("Cancel").clicked() {
                                state.start_new();
                            }
                        }
                    });

                    // Delete confirmation
                    if let Some((id, name, usage)) = state.delete_pending.clone() {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.colored_label(Color32::YELLOW, format!("⚠ Delete '{}'?", name));
                        if usage > 0 {
                            ui.label(RichText::new(format!("{} event(s) will be uncategorized.", usage)).small());
                        }

                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("Yes, Delete").color(Color32::RED)).clicked() {
                                match service.delete(id) {
                                    Ok(_) => {
                                        state.success_message = Some(format!("Deleted '{}'", name));
                                        response.categories_changed = true;
                                        state.needs_refresh = true;
                                        state.start_new();
                                    }
                                    Err(e) => {
                                        state.error_message = Some(format!("Failed to delete: {}", e));
                                    }
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

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // Footer
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    state.close();
                }
                ui.add_space(20.0);
                ui.label(RichText::new(format!("{} categories", categories.len())).weak());
            });
        });

    if !dialog_open {
        state.close();
    }

    response
}

/// Handle save/create action
fn handle_save(
    state: &mut CategoryManagerState,
    service: &CategoryService,
    response: &mut CategoryManagerResponse,
) {
    state.error_message = None;
    state.success_message = None;

    let name = state.name_input.trim();
    if name.is_empty() {
        state.error_message = Some("Name cannot be empty".to_string());
        return;
    }
    if !is_valid_hex_color(&state.color_input) {
        state.error_message = Some("Invalid color format".to_string());
        return;
    }

    let exclude_id = state.editing_category.as_ref().and_then(|c| c.id);
    if service.name_exists(name, exclude_id).unwrap_or(false) {
        state.error_message = Some("A category with this name already exists".to_string());
        return;
    }

    let icon = if state.icon_input.trim().is_empty() {
        None
    } else {
        Some(state.icon_input.trim().to_string())
    };

    if let Some(ref mut editing) = state.editing_category {
        // Update existing
        if !editing.is_system {
            editing.name = name.to_string();
        }
        editing.color = state.color_input.clone();
        editing.icon = icon;

        match service.update(editing) {
            Ok(_) => {
                state.success_message = Some("Category updated".to_string());
                response.categories_changed = true;
                state.needs_refresh = true;
            }
            Err(e) => {
                state.error_message = Some(format!("Failed to update: {}", e));
            }
        }
    } else {
        // Create new
        let mut new_cat = Category::new(name, &state.color_input);
        new_cat.icon = icon;

        match service.create(new_cat) {
            Ok(created) => {
                state.success_message = Some(format!("Created '{}'", created.name));
                response.categories_changed = true;
                state.needs_refresh = true;
                state.start_edit(created);
            }
            Err(e) => {
                state.error_message = Some(format!("Failed to create: {}", e));
            }
        }
    }
}

/// Parse a hex color string to Color32.
fn parse_hex_color(hex: &str) -> Color32 {
    let hex = hex.trim().trim_start_matches('#');
    
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Color32::from_rgb(r, g, b);
        }
    } else if hex.len() == 3 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..1], 16),
            u8::from_str_radix(&hex[1..2], 16),
            u8::from_str_radix(&hex[2..3], 16),
        ) {
            return Color32::from_rgb(r * 17, g * 17, b * 17);
        }
    }
    
    Color32::GRAY
}

/// Check if a string is a valid hex color.
fn is_valid_hex_color(color: &str) -> bool {
    let hex = color.trim().trim_start_matches('#');
    matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
}
