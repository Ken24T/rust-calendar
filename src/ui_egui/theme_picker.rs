use egui::{Context, Visuals};

/// Render a simple theme picker dialog
pub fn render_theme_picker(
    ctx: &Context,
    current_theme: &mut String,
    show_dialog: &mut bool,
) -> bool {
    let mut changed = false;
    
    egui::Window::new("Theme Selection")
        .collapsible(false)
        .resizable(false)
        .default_width(350.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.heading("Select Theme");
            ui.add_space(8.0);
            
            ui.label("Choose your preferred theme:");
            ui.add_space(8.0);
            
            // Light theme option
            ui.horizontal(|ui| {
                if ui.selectable_label(*current_theme == "light", "☀ Light Theme").clicked() {
                    *current_theme = "light".to_string();
                    apply_theme(ctx, current_theme);
                    changed = true;
                }
            });
            
            ui.add_space(4.0);
            
            // Dark theme option
            ui.horizontal(|ui| {
                if ui.selectable_label(*current_theme == "dark", "🌙 Dark Theme").clicked() {
                    *current_theme = "dark".to_string();
                    apply_theme(ctx, current_theme);
                    changed = true;
                }
            });
            
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            
            // Preview
            ui.label("Preview:");
            ui.group(|ui| {
                ui.set_min_width(300.0);
                ui.set_min_height(80.0);
                
                ui.heading("Sample Heading");
                ui.label("This is sample text in the selected theme.");
                ui.horizontal(|ui| {
                    let _ = ui.button("Sample Button");
                    ui.checkbox(&mut true.clone(), "Sample Checkbox");
                });
            });
            
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            
            // Close button
            ui.horizontal(|ui| {
                if ui.button("✓ Done").clicked() {
                    *show_dialog = false;
                }
            });
        });
    
    changed
}

/// Apply the theme to the egui context
fn apply_theme(ctx: &Context, theme: &str) {
    let visuals = if theme.to_lowercase().contains("dark") {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    
    ctx.set_visuals(visuals);
}
