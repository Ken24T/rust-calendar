//! Confirmation dialog system for destructive actions.
//!
//! Shows modal dialogs asking the user to confirm actions like deleting events,
//! discarding changes, etc.

// Allow unused variants/methods - these are API surface for future use
#![allow(dead_code)]

use egui::{Context, RichText};

/// Types of confirmation dialogs
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    /// Delete an event by ID
    DeleteEvent { event_id: i64, event_title: String },
    /// Delete a single occurrence of a recurring event
    DeleteEventOccurrence { 
        event_id: i64, 
        event_title: String, 
        occurrence_date: chrono::DateTime<chrono::Local> 
    },
    /// Delete a countdown card by ID
    DeleteCountdownCard { card_id: crate::services::countdown::CountdownCardId, card_title: String },
    /// Discard unsaved changes in a dialog
    DiscardChanges,
    /// Delete a custom theme
    DeleteTheme { theme_name: String },
}

impl ConfirmAction {
    /// Get the dialog title for this action
    pub fn title(&self) -> &'static str {
        match self {
            ConfirmAction::DeleteEvent { .. } => "Delete Event",
            ConfirmAction::DeleteEventOccurrence { .. } => "Delete Occurrence",
            ConfirmAction::DeleteCountdownCard { .. } => "Delete Countdown",
            ConfirmAction::DiscardChanges => "Discard Changes",
            ConfirmAction::DeleteTheme { .. } => "Delete Theme",
        }
    }

    /// Get the confirmation message for this action
    pub fn message(&self) -> String {
        match self {
            ConfirmAction::DeleteEvent { event_title, .. } => {
                format!("Are you sure you want to delete \"{}\"?\n\nThis action cannot be undone.", event_title)
            }
            ConfirmAction::DeleteEventOccurrence { event_title, occurrence_date, .. } => {
                format!(
                    "Are you sure you want to delete this occurrence of \"{}\" on {}?\n\nOther occurrences will not be affected.",
                    event_title,
                    occurrence_date.format("%B %d, %Y")
                )
            }
            ConfirmAction::DeleteCountdownCard { card_title, .. } => {
                format!("Are you sure you want to delete the countdown \"{}\"?\n\nThis action cannot be undone.", card_title)
            }
            ConfirmAction::DiscardChanges => {
                "You have unsaved changes.\n\nAre you sure you want to discard them?".to_string()
            }
            ConfirmAction::DeleteTheme { theme_name } => {
                format!("Are you sure you want to delete the theme \"{}\"?\n\nThis action cannot be undone.", theme_name)
            }
        }
    }

    /// Get the confirm button text for this action
    pub fn confirm_text(&self) -> &'static str {
        match self {
            ConfirmAction::DeleteEvent { .. } => "Delete",
            ConfirmAction::DeleteEventOccurrence { .. } => "Delete",
            ConfirmAction::DeleteCountdownCard { .. } => "Delete",
            ConfirmAction::DiscardChanges => "Discard",
            ConfirmAction::DeleteTheme { .. } => "Delete",
        }
    }

    /// Check if this is a destructive action (shows confirm button in red)
    pub fn is_destructive(&self) -> bool {
        match self {
            ConfirmAction::DeleteEvent { .. } => true,
            ConfirmAction::DeleteEventOccurrence { .. } => true,
            ConfirmAction::DeleteCountdownCard { .. } => true,
            ConfirmAction::DiscardChanges => false,
            ConfirmAction::DeleteTheme { .. } => true,
        }
    }
}

/// Result of a confirmation dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmResult {
    /// User confirmed the action
    Confirmed,
    /// User cancelled the action
    Cancelled,
    /// Dialog is still open
    Pending,
}

/// State for the confirmation dialog
#[derive(Debug, Default)]
pub struct ConfirmDialogState {
    /// The pending action to confirm
    pending_action: Option<ConfirmAction>,
}

impl ConfirmDialogState {
    /// Create a new confirm dialog state
    pub fn new() -> Self {
        Self::default()
    }

    /// Request confirmation for an action
    pub fn request(&mut self, action: ConfirmAction) {
        self.pending_action = Some(action);
    }

    /// Check if there's a pending confirmation
    pub fn is_open(&self) -> bool {
        self.pending_action.is_some()
    }

    /// Get the pending action (if any)
    pub fn pending_action(&self) -> Option<&ConfirmAction> {
        self.pending_action.as_ref()
    }

    /// Close the dialog without confirming
    pub fn cancel(&mut self) {
        self.pending_action = None;
    }

    /// Render the confirmation dialog and return the result
    pub fn render(&mut self, ctx: &Context) -> ConfirmResult {
        let Some(action) = &self.pending_action else {
            return ConfirmResult::Pending;
        };

        let mut result = ConfirmResult::Pending;
        let mut should_close = false;

        egui::Window::new(action.title())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(300.0);
                ui.set_max_width(400.0);

                ui.add_space(10.0);

                // Warning icon for destructive actions
                if action.is_destructive() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠").size(24.0).color(egui::Color32::from_rgb(220, 150, 50)));
                        ui.vertical(|ui| {
                            ui.label(action.message());
                        });
                    });
                } else {
                    ui.label(action.message());
                }

                ui.add_space(15.0);
                ui.separator();
                ui.add_space(10.0);

                // Buttons
                ui.horizontal(|ui| {
                    // Right-align buttons
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Confirm button
                        let confirm_button = if action.is_destructive() {
                            egui::Button::new(
                                RichText::new(action.confirm_text())
                                    .color(egui::Color32::WHITE)
                            )
                            .fill(egui::Color32::from_rgb(180, 60, 60))
                        } else {
                            egui::Button::new(action.confirm_text())
                        };

                        if ui.add(confirm_button).clicked() {
                            result = ConfirmResult::Confirmed;
                            should_close = true;
                        }

                        ui.add_space(10.0);

                        // Cancel button
                        if ui.button("Cancel").clicked() {
                            result = ConfirmResult::Cancelled;
                            should_close = true;
                        }
                    });
                });

                ui.add_space(5.0);
            });

        // Handle Escape key
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            result = ConfirmResult::Cancelled;
            should_close = true;
        }

        if should_close {
            self.pending_action = None;
        }

        result
    }

    /// Take the pending action (consuming it)
    pub fn take_action(&mut self) -> Option<ConfirmAction> {
        self.pending_action.take()
    }
}

use super::CalendarApp;

impl CalendarApp {
    /// Handle the confirmation dialog rendering and process confirmed actions
    pub(super) fn handle_confirm_dialog(&mut self, ctx: &Context) {
        use super::confirm::ConfirmResult;
        
        let result = self.confirm_dialog.render(ctx);
        
        if result == ConfirmResult::Confirmed {
            // Get the action that was confirmed
            if let Some(action) = self.confirm_dialog.take_action() {
                self.execute_confirmed_action(action);
            }
        }
    }
    
    /// Execute an action that has been confirmed by the user
    fn execute_confirmed_action(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::DeleteEvent { event_id, event_title } => {
                let event_service = self.context.event_service();
                if let Err(e) = event_service.delete(event_id) {
                    log::error!("Failed to delete event: {}", e);
                    self.toast_manager.error(format!("Failed to delete event: {}", e));
                } else {
                    log::info!("Deleted event: {} (ID: {})", event_title, event_id);
                    self.toast_manager.success(format!("Deleted \"{}\"", event_title));
                    
                    // Also remove any linked countdown card
                    self.context.countdown_service_mut().remove_cards_for_event(event_id);
                }
            }
            ConfirmAction::DeleteEventOccurrence { event_id, event_title, occurrence_date } => {
                let event_service = self.context.event_service();
                if let Err(e) = event_service.delete_occurrence(event_id, occurrence_date) {
                    log::error!("Failed to delete occurrence: {}", e);
                    self.toast_manager.error(format!("Failed to delete occurrence: {}", e));
                } else {
                    log::info!("Deleted occurrence of {} on {}", event_title, occurrence_date.format("%Y-%m-%d"));
                    self.toast_manager.success(format!("Deleted occurrence of \"{}\"", event_title));
                    // Note: Don't remove countdown card for occurrence-only deletion
                }
            }
            ConfirmAction::DeleteCountdownCard { card_id, card_title } => {
                self.context.countdown_service_mut().remove_card(card_id);
                log::info!("Deleted countdown card: {} (ID: {:?})", card_title, card_id);
                self.toast_manager.success(format!("Deleted countdown \"{}\"", card_title));
            }
            ConfirmAction::DiscardChanges => {
                // The dialog that requested this should handle closing itself
                log::info!("User chose to discard changes");
            }
            ConfirmAction::DeleteTheme { theme_name } => {
                let theme_service = self.context.theme_service();
                if let Err(e) = theme_service.delete_theme(&theme_name) {
                    log::error!("Failed to delete theme: {}", e);
                    self.toast_manager.error(format!("Failed to delete theme: {}", e));
                } else {
                    log::info!("Deleted theme: {}", theme_name);
                    self.toast_manager.success(format!("Deleted theme \"{}\"", theme_name));
                }
            }
        }
    }
}
