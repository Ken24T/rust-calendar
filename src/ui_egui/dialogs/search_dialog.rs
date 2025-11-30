//! Search dialog for finding events

use chrono::NaiveDate;
use egui::{Margin, RichText, Stroke, Vec2};

use crate::models::event::Event;
use crate::services::database::Database;
use crate::services::event::EventService;
use crate::ui_egui::theme::CalendarTheme;
use crate::ui_egui::views::utils::get_event_color;

/// State for the search dialog
pub struct SearchDialogState {
    pub query: String,
    pub results: Vec<Event>,
    pub selected_event: Option<i64>,
}

impl Default for SearchDialogState {
    fn default() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected_event: None,
        }
    }
}

/// Action result from the search dialog
pub enum SearchDialogAction {
    /// No action
    None,
    /// Navigate to a specific date (to view the event)
    NavigateToDate(NaiveDate),
    /// Edit the selected event
    EditEvent(i64),
    /// Close the dialog
    Close,
}

/// Render the search dialog
pub fn render_search_dialog(
    ctx: &egui::Context,
    state: &mut SearchDialogState,
    database: &Database,
    theme: &CalendarTheme,
    open: &mut bool,
) -> SearchDialogAction {
    let mut action = SearchDialogAction::None;
    let mut dialog_open = *open;

    egui::Window::new("🔍 Search Events")
        .open(&mut dialog_open)
        .collapsible(false)
        .resizable(true)
        .default_width(500.0)
        .default_height(450.0)
        .min_width(400.0)
        .min_height(300.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Search input
            ui.horizontal(|ui| {
                ui.label("Search:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.query)
                        .desired_width(ui.available_width() - 80.0)
                        .hint_text("Type to search events..."),
                );

                // Auto-focus on open
                if response.gained_focus() || state.query.is_empty() {
                    response.request_focus();
                }

                if ui.button("Clear").clicked() {
                    state.query.clear();
                    state.results.clear();
                }
            });

            ui.add_space(8.0);

            // Perform search when query changes
            if !state.query.is_empty() {
                let event_service = EventService::new(database.connection());
                if let Ok(results) = event_service.search(&state.query) {
                    state.results = results;
                }
            } else {
                state.results.clear();
            }

            // Results count
            if state.query.is_empty() {
                ui.label(RichText::new("Enter a search term to find events").italics());
            } else if state.results.is_empty() {
                ui.label(RichText::new("No events found").italics());
            } else {
                ui.label(format!("{} event(s) found", state.results.len()));
            }

            ui.separator();

            // Results list
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(350.0)
                .show(ui, |ui| {
                    for event in &state.results {
                        let is_selected = state.selected_event == event.id;
                        
                        let frame_bg = if is_selected {
                            theme.today_background
                        } else {
                            theme.day_background
                        };
                        
                        let event_color = get_event_color(event);

                        let response = egui::Frame::none()
                            .fill(frame_bg)
                            .rounding(egui::Rounding::same(6.0))
                            .stroke(Stroke::new(1.0, theme.day_border))
                            .inner_margin(Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Color indicator
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::new(4.0, 40.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 2.0, event_color);

                                    ui.add_space(8.0);

                                    ui.vertical(|ui| {
                                        // Title
                                        ui.label(
                                            RichText::new(&event.title)
                                                .strong()
                                                .color(theme.text_primary),
                                        );

                                        // Date/time
                                        let date_str = if event.all_day {
                                            event.start.format("%B %d, %Y").to_string()
                                        } else {
                                            event.start.format("%B %d, %Y at %I:%M %p").to_string()
                                        };
                                        ui.label(
                                            RichText::new(date_str)
                                                .size(11.0)
                                                .color(theme.text_secondary),
                                        );

                                        // Location if present
                                        if let Some(ref loc) = event.location {
                                            if !loc.is_empty() {
                                                ui.label(
                                                    RichText::new(format!("📍 {}", loc))
                                                        .size(10.0)
                                                        .color(theme.text_secondary),
                                                );
                                            }
                                        }
                                    });
                                });
                            })
                            .response;

                        // Handle clicks - single click navigates to date
                        if response.clicked() {
                            state.selected_event = event.id;
                            action = SearchDialogAction::NavigateToDate(
                                event.start.date_naive(),
                            );
                        }

                        if response.double_clicked() {
                            if let Some(id) = event.id {
                                action = SearchDialogAction::EditEvent(id);
                            }
                        }

                        // Context menu
                        response.context_menu(|ui| {
                            if ui.button("📅 Go to date").clicked() {
                                action = SearchDialogAction::NavigateToDate(
                                    event.start.date_naive(),
                                );
                                ui.close_menu();
                            }
                            if ui.button("✏ Edit event").clicked() {
                                if let Some(id) = event.id {
                                    action = SearchDialogAction::EditEvent(id);
                                }
                                ui.close_menu();
                            }
                        });

                        ui.add_space(4.0);
                    }
                });

            ui.add_space(8.0);
            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                if let Some(_event_id) = state.selected_event {
                    if ui.button("📅 Go to date").clicked() {
                        if let Some(event) = state.results.iter().find(|e| e.id == state.selected_event) {
                            action = SearchDialogAction::NavigateToDate(event.start.date_naive());
                        }
                    }

                    if ui.button("✏ Edit").clicked() {
                        if let Some(id) = state.selected_event {
                            action = SearchDialogAction::EditEvent(id);
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        action = SearchDialogAction::Close;
                        *open = false;
                    }
                });
            });

            // Handle keyboard shortcuts
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                action = SearchDialogAction::Close;
                *open = false;
            }

            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && state.selected_event.is_some() {
                if let Some(id) = state.selected_event {
                    action = SearchDialogAction::EditEvent(id);
                }
            }
        });

    if !dialog_open {
        *open = false;
    }

    action
}
