use chrono::Local;
use egui::{Color32, RichText};

use crate::models::event::Event;
use crate::models::settings::Settings;
use crate::services::category::CategoryService;
use crate::services::countdown::CountdownCardId;
use crate::services::database::Database;

use super::state::EventDialogState;
use super::widgets::{indented_row, labeled_row, parse_hex_color};
use super::{render_date_time, render_recurrence};

/// Changes to apply to a linked countdown card
#[derive(Debug, Clone)]
pub struct CountdownCardChanges {
    pub card_id: CountdownCardId,
    pub description: Option<String>,
    pub color: Option<String>,
    pub start_date: chrono::NaiveDate,
    pub start_time: chrono::NaiveTime,
    pub always_on_top: bool,
    pub title_font_size: f32,
    pub days_font_size: f32,
}

/// Request for delete confirmation from the event dialog
#[derive(Clone)]
pub struct EventDeleteRequest {
    pub event_id: i64,
    pub event_title: String,
}

#[derive(Default)]
pub struct EventDialogResult {
    pub saved_event: Option<Event>,
    pub card_changes: Option<CountdownCardChanges>,
    /// Request to show delete confirmation dialog
    pub delete_request: Option<EventDeleteRequest>,
}

impl EventDialogResult {}

pub fn render_event_dialog(
    ctx: &egui::Context,
    state: &mut EventDialogState,
    database: &Database,
    settings: &Settings,
    show_dialog: &mut bool,
) -> EventDialogResult {
    let mut result = EventDialogResult::default();
    let mut dialog_open = *show_dialog;

    // Check for warnings (overlap detection, etc.) - this updates state.warning_messages
    state.check_warnings(database);

    egui::Window::new(if state.event_id.is_some() {
        "Edit Event"
    } else {
        "New Event"
    })
    .open(&mut dialog_open)
    .collapsible(false)
    .resizable(true)
    .default_width(600.0)
    .default_height(750.0)
    .min_height(720.0)
    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    .show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            render_error_banner(ui, state);
            render_warning_banner(ui, state);
            render_basic_information_section(ui, state, database);
            render_date_time::render_date_time_section(ui, state);
            render_appearance_section(ui, state);
            render_recurrence::render_recurrence_section(ui, state, settings);
            render_countdown_card_section(ui, state);
            let action = render_action_buttons(ui, state, database, show_dialog);
            if action.saved_event.is_some() {
                result = action;
            }
        });
    });

    if !dialog_open {
        *show_dialog = false;
    }

    result
}

fn render_error_banner(ui: &mut egui::Ui, state: &EventDialogState) {
    if let Some(ref error) = state.error_message {
        ui.colored_label(Color32::RED, RichText::new(error).strong());
        ui.add_space(8.0);
    }
}

fn render_warning_banner(ui: &mut egui::Ui, state: &EventDialogState) {
    if state.warning_messages.is_empty() {
        return;
    }
    
    let warning_color = Color32::from_rgb(200, 140, 0); // Orange/amber
    
    for warning in &state.warning_messages {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⚠").color(warning_color));
            ui.colored_label(warning_color, warning);
        });
    }
    ui.add_space(4.0);
}

fn render_basic_information_section(ui: &mut egui::Ui, state: &mut EventDialogState, database: &Database) {
    ui.heading("Basic Information");
    ui.add_space(4.0);

    labeled_row(
        ui,
        if state.title.trim().is_empty() {
            RichText::new("Title:")
                .strong()
                .color(Color32::from_rgb(255, 150, 150))
        } else {
            RichText::new("Title:").strong()
        },
        |ui| {
            let title_response = ui.text_edit_singleline(&mut state.title);
            ui.label(RichText::new("*").color(Color32::from_rgb(255, 150, 150)));

            if title_response.changed() && state.error_message.is_some() {
                let _ = state.error_message.take();
            }
        },
    );

    labeled_row(ui, "Location:", |ui| {
        ui.text_edit_singleline(&mut state.location);
        
        // Show "Open in Maps" button if location is not empty
        let location_trimmed = state.location.trim();
        if !location_trimmed.is_empty()
            && ui.button("🗺").on_hover_text("Open in Google Maps").clicked() {
                let encoded = urlencoding::encode(location_trimmed);
                let url = format!("https://www.google.com/maps/search/?api=1&query={}", encoded);
                if let Err(e) = webbrowser::open(&url) {
                    log::error!("Failed to open maps URL: {}", e);
                }
            }
    });

    // Category dropdown
    labeled_row(ui, "Category:", |ui| {
        render_category_dropdown(ui, &mut state.category, database);
    });

    labeled_row(ui, "Description:", |ui| {
        let width = ui.available_width();
        ui.add_sized(
            [width, 100.0],
            egui::TextEdit::multiline(&mut state.description),
        );
    });

    if state.event_id.is_none() {
        ui.add_space(8.0);
        let today = Local::now().date_naive();
        let is_future_event = state.date > today;
        
        indented_row(ui, |ui| {
            ui.add_enabled_ui(is_future_event, |ui| {
                ui.checkbox(
                    &mut state.create_countdown,
                    "Create countdown card after saving",
                )
                .on_hover_text(if is_future_event {
                    "Also spawns a countdown using this event's color once you save."
                } else {
                    "Countdown cards can only be created for future events."
                });
            });
            
            if !is_future_event && state.create_countdown {
                // Auto-uncheck if date changed to today or past
                state.create_countdown = false;
            }
        });
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
}

fn render_appearance_section(ui: &mut egui::Ui, state: &mut EventDialogState) {
    ui.heading("Appearance");
    ui.add_space(4.0);

    labeled_row(ui, "Color:", |ui| {
        ui.add(egui::TextEdit::singleline(&mut state.color).desired_width(80.0));

        if let Some(mut color) = parse_hex_color(&state.color) {
            ui.color_edit_button_srgba(&mut color);
            state.color = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
        }
    });

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
                    state.color = hex.to_string();
                }
            }
        });
    });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
}

fn render_countdown_card_section(ui: &mut egui::Ui, state: &mut EventDialogState) {
    // Only show this section if a countdown card is linked to the event
    let Some(ref mut linked_card) = state.linked_card else {
        return;
    };

    ui.heading("Countdown Card");
    ui.add_space(4.0);

    // Collapsible header
    egui::CollapsingHeader::new("Card Display Settings")
        .default_open(state.show_card_settings)
        .show(ui, |ui| {
            ui.add_space(4.0);

            // Layout options
            indented_row(ui, |ui| {
                if ui
                    .checkbox(&mut linked_card.always_on_top, "Always on top")
                    .changed()
                {
                    linked_card.visuals.always_on_top = linked_card.always_on_top;
                }
            });

            ui.add_space(8.0);

            // Font sizes
            labeled_row(ui, "Title font size:", |ui| {
                ui.add(egui::Slider::new(
                    &mut linked_card.visuals.title_font_size,
                    12.0..=48.0,
                ));
            });

            labeled_row(ui, "Countdown font size:", |ui| {
                ui.add(egui::Slider::new(
                    &mut linked_card.visuals.days_font_size,
                    32.0..=220.0,
                ));
            });

            ui.add_space(8.0);
            ui.label(
                RichText::new("Note: Color changes are synced with the event color above.")
                    .small()
                    .weak(),
            );
        });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
}

fn render_action_buttons(
    ui: &mut egui::Ui,
    state: &mut EventDialogState,
    database: &Database,
    show_dialog: &mut bool,
) -> EventDialogResult {
    let mut saved_event = None;
    let mut delete_request = None;

    indented_row(ui, |ui| {
        let can_save = !state.title.trim().is_empty();
        let save_button = egui::Button::new("Save").fill(if can_save {
            Color32::from_rgb(70, 120, 200)
        } else {
            Color32::from_gray(60)
        });

        ui.add_enabled_ui(can_save, |ui| {
            if ui.add(save_button).clicked() {
                match state.save(database) {
                    Ok(event) => {
                        saved_event = Some(event);
                        *show_dialog = false;
                    }
                    Err(e) => {
                        state.error_message = Some(e);
                    }
                }
            }
        });

        if !can_save {
            ui.label(
                RichText::new("(Title required)")
                    .small()
                    .color(Color32::from_gray(150)),
            );
        }

        if ui.button("Cancel").clicked() {
            *show_dialog = false;
        }

        if state.event_id.is_some() {
            ui.add_space(20.0);
            if ui
                .button(RichText::new("Delete").color(Color32::RED))
                .clicked()
            {
                if let Some(id) = state.event_id {
                    delete_request = Some(EventDeleteRequest {
                        event_id: id,
                        event_title: state.title.clone(),
                    });
                    *show_dialog = false;
                }
            }
        }
    });

    // Add padding below buttons
    ui.add_space(16.0);

    // Build card changes if there's a linked card and we saved successfully
    let card_changes = if saved_event.is_some() {
        state.linked_card.as_ref().map(|card| CountdownCardChanges {
            card_id: card.card_id,
            description: if state.description.is_empty() {
                None
            } else {
                Some(state.description.clone())
            },
            color: if state.color.is_empty() {
                None
            } else {
                Some(state.color.clone())
            },
            start_date: state.date,
            start_time: state.start_time,
            always_on_top: card.always_on_top,
            title_font_size: card.visuals.title_font_size,
            days_font_size: card.visuals.days_font_size,
        })
    } else {
        None
    };

    EventDialogResult {
        saved_event,
        card_changes,
        delete_request,
    }
}

/// Render a category dropdown with color swatches
fn render_category_dropdown(ui: &mut egui::Ui, selected_category: &mut String, database: &Database) {
    let service = CategoryService::new(database.connection());
    let categories = service.list_all().unwrap_or_default();
    
    // Find the selected category for display
    let selected_display = if selected_category.is_empty() {
        "None".to_string()
    } else {
        categories.iter()
            .find(|c| c.name == *selected_category)
            .map(|c| c.display_name())
            .unwrap_or_else(|| selected_category.clone())
    };
    
    // Get the color of the selected category for the preview
    let selected_color = categories.iter()
        .find(|c| c.name == *selected_category)
        .map(|c| parse_hex_color(&c.color).unwrap_or(Color32::GRAY))
        .unwrap_or(Color32::TRANSPARENT);

    ui.horizontal(|ui| {
        // Show color swatch for selected category
        if !selected_category.is_empty() {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 3.0, selected_color);
        }
        
        egui::ComboBox::from_id_source("category_dropdown")
            .selected_text(&selected_display)
            .width(180.0)
            .show_ui(ui, |ui| {
                // "None" option
                let is_none = selected_category.is_empty();
                if ui.selectable_label(is_none, "None").clicked() {
                    selected_category.clear();
                }
                
                ui.separator();
                
                // Category options
                for cat in &categories {
                    let is_selected = cat.name == *selected_category;
                    
                    ui.horizontal(|ui| {
                        // Color swatch
                        let color = parse_hex_color(&cat.color).unwrap_or(Color32::GRAY);
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, color);
                        
                        if ui.selectable_label(is_selected, cat.display_name()).clicked() {
                            *selected_category = cat.name.clone();
                        }
                    });
                }
            });
    });
}
