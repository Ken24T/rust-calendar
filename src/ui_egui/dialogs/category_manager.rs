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
        .resizable(true)
        .default_width(500.0)
        .min_width(400.0)
        .default_height(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Error/success messages
            if let Some(ref error) = state.error_message {
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::RED, format!("❌ {}", error));
                });
                ui.add_space(4.0);
            }
            if let Some(ref success) = state.success_message {
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::GREEN, format!("✓ {}", success));
                });
                ui.add_space(4.0);
            }

            // Split view: list on left, editor on right
            ui.horizontal(|ui| {
                // Category list (left side)
                ui.vertical(|ui| {
                    ui.set_min_width(200.0);
                    ui.heading("Categories");
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for cat in &categories {
                                let is_selected = state.editing_category.as_ref()
                                    .map(|e| e.id == cat.id)
                                    .unwrap_or(false);

                                ui.horizontal(|ui| {
                                    // Color swatch
                                    let color = parse_hex_color(&cat.color);
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(16.0, 16.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 3.0, color);

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

                    ui.add_space(8.0);

                    if ui.button("➕ New Category").clicked() {
                        state.start_new();
                    }
                });

                ui.separator();

                // Editor (right side)
                ui.vertical(|ui| {
                    ui.set_min_width(220.0);
                    
                    let is_editing = state.editing_category.is_some();
                    let is_system = state.editing_category.as_ref()
                        .map(|c| c.is_system)
                        .unwrap_or(false);

                    ui.heading(if is_editing { "Edit Category" } else { "New Category" });
                    ui.add_space(8.0);

                    // Name field (disabled for system categories)
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add_enabled(
                            !is_system,
                            egui::TextEdit::singleline(&mut state.name_input)
                                .desired_width(150.0)
                                .hint_text("Category name"),
                        );
                    });
                    if is_system {
                        ui.label(RichText::new("System category names cannot be changed").small().weak());
                    }

                    ui.add_space(4.0);

                    // Color picker
                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        
                        // Text input for hex
                        ui.add(
                            egui::TextEdit::singleline(&mut state.color_input)
                                .desired_width(80.0)
                                .hint_text("#RRGGBB"),
                        );

                        // Color preview
                        let preview_color = parse_hex_color(&state.color_input);
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(24.0, 24.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 4.0, preview_color);
                    });

                    // Color presets
                    ui.horizontal(|ui| {
                        ui.label("Presets:");
                        let presets = [
                            ("#3B82F6", "Blue"),
                            ("#10B981", "Green"),
                            ("#F59E0B", "Amber"),
                            ("#EF4444", "Red"),
                            ("#8B5CF6", "Purple"),
                            ("#EC4899", "Pink"),
                        ];
                        for (hex, _name) in presets {
                            let color = parse_hex_color(hex);
                            if ui.add(egui::Button::new("").fill(color).min_size(egui::vec2(20.0, 20.0))).clicked() {
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
                                .desired_width(60.0)
                                .hint_text("emoji"),
                        );
                        ui.label(RichText::new("(optional emoji)").small().weak());
                    });

                    // Icon presets
                    ui.horizontal(|ui| {
                        ui.label("Presets:");
                        let icon_presets = ["💼", "🏠", "🎂", "🎉", "👥", "⏰", "📅", "✈️", "🏃", "📚"];
                        for icon in icon_presets {
                            if ui.small_button(icon).clicked() {
                                state.icon_input = icon.to_string();
                            }
                        }
                    });

                    ui.add_space(16.0);

                    // Action buttons
                    ui.horizontal(|ui| {
                        // Save button
                        if ui.button(if is_editing { "💾 Save" } else { "➕ Create" }).clicked() {
                            state.error_message = None;
                            state.success_message = None;

                            let name = state.name_input.trim();
                            if name.is_empty() {
                                state.error_message = Some("Name cannot be empty".to_string());
                            } else if !is_valid_hex_color(&state.color_input) {
                                state.error_message = Some("Invalid color format".to_string());
                            } else {
                                // Check for duplicate name
                                let exclude_id = state.editing_category.as_ref().and_then(|c| c.id);
                                if service.name_exists(name, exclude_id).unwrap_or(false) {
                                    state.error_message = Some("A category with this name already exists".to_string());
                                } else {
                                    // Create or update
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
                            }
                        }

                        // Delete button (only for non-system, existing categories)
                        if is_editing && !is_system {
                            if let Some(ref cat) = state.editing_category {
                                if let Some(id) = cat.id {
                                    if ui.button(RichText::new("🗑 Delete").color(Color32::LIGHT_RED)).clicked() {
                                        // Get usage count for confirmation
                                        let usage = service.get_usage_count(&cat.name).unwrap_or(0);
                                        state.delete_pending = Some((id, cat.name.clone(), usage));
                                    }
                                }
                            }
                        }

                        // Cancel edit button
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
                        ui.add_space(4.0);
                        
                        ui.colored_label(Color32::YELLOW, format!(
                            "⚠ Delete '{}'?", name
                        ));
                        if usage > 0 {
                            ui.label(format!(
                                "{} event(s) use this category and will be uncategorized.",
                                usage
                            ));
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

                            if ui.button("No, Keep").clicked() {
                                state.delete_pending = None;
                            }
                        });
                    }
                });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // Close button
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
