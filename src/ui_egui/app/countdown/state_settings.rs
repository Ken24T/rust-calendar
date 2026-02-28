use super::render::{
    viewport_builder_for_settings, viewport_title_matches, COUNTDOWN_SETTINGS_HEIGHT,
};
use super::settings::{render_countdown_settings_ui, CountdownSettingsCommand};
use super::state::{CountdownUiState, DeleteCardRequest};
use crate::services::countdown::{
    CountdownCardGeometry, CountdownCardState, CountdownService,
};
use egui::Context;

use super::super::geometry::{geometry_from_viewport_info, viewport_info};

impl CountdownUiState {
    pub(in crate::ui_egui) fn render_settings_dialogs(
        &mut self,
        ctx: &Context,
        service: &mut CountdownService,
    ) {
        if self.open_settings.is_empty() {
            return;
        }

        let cards_snapshot = service.cards().to_vec();
        self.open_settings
            .retain(|id| cards_snapshot.iter().any(|card| &card.id == id));

        let mut dialogs_to_close = Vec::new();
        let defaults_snapshot = service.defaults().clone();
        let open_windows: Vec<_> = self.open_settings.iter().copied().collect();

        for id in open_windows {
            if let Some(card) = cards_snapshot.iter().find(|card| card.id == id) {
                let default_geometry = default_settings_geometry_for(card);
                let geometry_copy = {
                    let entry = self.settings_geometry.entry(id).or_insert(default_geometry);
                    *entry
                };
                let viewport_id =
                    egui::ViewportId::from_hash_of(("countdown_settings", card.id.0));
                let apply_layout = self.settings_needs_layout.remove(&id);
                let settings_title = format!("Settings: {}", card.effective_title());
                let builder = viewport_builder_for_settings(
                    if apply_layout {
                        Some(geometry_copy)
                    } else {
                        None
                    },
                    card,
                );

                let card_clone = card.clone();
                let defaults_clone = defaults_snapshot.clone();
                let result = ctx.show_viewport_immediate(
                    viewport_id,
                    builder,
                    move |child_ctx, class| {
                        render_countdown_settings_ui(
                            child_ctx,
                            class,
                            &card_clone,
                            &defaults_clone,
                        )
                    },
                );

                let viewport_info = viewport_info(ctx, viewport_id);
                let mut should_close = viewport_info
                    .as_ref()
                    .map(|info| info.close_requested())
                    .unwrap_or(false);

                for command in result.commands {
                    if self.apply_settings_command(service, command) {
                        should_close = true;
                    }
                }

                if result.close_requested {
                    should_close = true;
                }

                if let Some(info) = viewport_info.as_ref() {
                    if viewport_title_matches(info, &settings_title) {
                        if let Some(geometry) = geometry_from_viewport_info(info) {
                            if let Some(entry) = self.settings_geometry.get_mut(&id) {
                                *entry = geometry;
                            }
                        }
                    }
                }

                if should_close {
                    dialogs_to_close.push(id);
                }
            } else {
                dialogs_to_close.push(id);
            }
        }

        for id in dialogs_to_close {
            self.open_settings.remove(&id);
            self.settings_geometry.remove(&id);
            self.settings_needs_layout.remove(&id);
        }
    }

    fn apply_settings_command(
        &mut self,
        service: &mut CountdownService,
        command: CountdownSettingsCommand,
    ) -> bool {
        match command {
            CountdownSettingsCommand::SetTitleOverride(id, title) => {
                service.set_title_override(id, title);
                false
            }
            CountdownSettingsCommand::SetComment(id, comment) => {
                let event_id = service
                    .cards()
                    .iter()
                    .find(|card| card.id == id)
                    .and_then(|card| card.event_id);
                let next_body = comment.clone();
                service.set_comment(id, comment);
                if let Some(event_id) = event_id {
                    self.pending_event_body_updates.push((event_id, next_body));
                }
                false
            }
            CountdownSettingsCommand::SetAlwaysOnTop(id, value) => {
                service.set_always_on_top(id, value);
                false
            }
            CountdownSettingsCommand::SetDaysFontSize(id, size) => {
                service.set_days_font_size(id, size);
                false
            }
            CountdownSettingsCommand::SetTitleFontSize(id, size) => {
                service.set_title_font_size(id, size);
                false
            }
            CountdownSettingsCommand::SetTitleBgColor(id, color) => {
                service.set_title_bg_color(id, color);
                false
            }
            CountdownSettingsCommand::SetTitleFgColor(id, color) => {
                service.set_title_fg_color(id, color);
                false
            }
            CountdownSettingsCommand::SetBodyBgColor(id, color) => {
                service.set_body_bg_color(id, color);
                false
            }
            CountdownSettingsCommand::SetDaysFgColor(id, color) => {
                service.set_days_fg_color(id, color);
                false
            }
            CountdownSettingsCommand::SetUseDefaultTitleBg(id, value) => {
                service.set_use_default_title_bg(id, value);
                false
            }
            CountdownSettingsCommand::SetUseDefaultTitleFg(id, value) => {
                service.set_use_default_title_fg(id, value);
                false
            }
            CountdownSettingsCommand::SetUseDefaultBodyBg(id, value) => {
                service.set_use_default_body_bg(id, value);
                false
            }
            CountdownSettingsCommand::SetUseDefaultDaysFg(id, value) => {
                service.set_use_default_days_fg(id, value);
                false
            }
            CountdownSettingsCommand::ApplyVisualDefaults(id) => {
                service.apply_visual_defaults(id);
                false
            }
            CountdownSettingsCommand::RequestDeleteCard(id, title) => {
                // Add to pending delete requests - will be handled by the main app
                self.pending_delete_requests.push(DeleteCardRequest {
                    card_id: id,
                    card_title: title,
                });
                // Close settings and clean up UI state
                self.open_settings.remove(&id);
                self.settings_geometry.remove(&id);
                self.settings_needs_layout.remove(&id);
                self.render_log_state.remove(&id);
                self.clear_geometry_wait_state(&id);
                true
            }
            CountdownSettingsCommand::SetStartAt(id, start_at) => {
                service.set_start_at(id, start_at);
                false
            }
            CountdownSettingsCommand::SetDefaultTitleBgColor(color) => {
                service.set_default_title_bg_color(color);
                false
            }
            CountdownSettingsCommand::ResetDefaultTitleBgColor => {
                service.reset_default_title_bg_color();
                false
            }
            CountdownSettingsCommand::SetDefaultTitleFgColor(color) => {
                service.set_default_title_fg_color(color);
                false
            }
            CountdownSettingsCommand::ResetDefaultTitleFgColor => {
                service.reset_default_title_fg_color();
                false
            }
            CountdownSettingsCommand::SetDefaultBodyBgColor(color) => {
                service.set_default_body_bg_color(color);
                false
            }
            CountdownSettingsCommand::ResetDefaultBodyBgColor => {
                service.reset_default_body_bg_color();
                false
            }
            CountdownSettingsCommand::SetDefaultDaysFgColor(color) => {
                service.set_default_days_fg_color(color);
                false
            }
            CountdownSettingsCommand::ResetDefaultDaysFgColor => {
                service.reset_default_days_fg_color();
                false
            }
            CountdownSettingsCommand::SetDefaultDaysFontSize(size) => {
                service.set_default_days_font_size(size);
                false
            }
            CountdownSettingsCommand::ResetDefaultDaysFontSize => {
                service.reset_default_days_font_size();
                false
            }
            CountdownSettingsCommand::SetDefaultTitleFontSize(size) => {
                service.set_default_title_font_size(size);
                false
            }
            CountdownSettingsCommand::ResetDefaultTitleFontSize => {
                service.reset_default_title_font_size();
                false
            }
        }
    }
}

pub(super) fn default_settings_geometry_for(card: &CountdownCardState) -> CountdownCardGeometry {
    // Try to position to the right of the card, but ensure it fits on screen
    let settings_width = 640.0;
    let settings_height = COUNTDOWN_SETTINGS_HEIGHT;

    // Start with position to the right of the card
    let mut x = card.geometry.x + card.geometry.width + 16.0;
    let mut y = card.geometry.y;

    // If that would go off the right edge, position to the left instead
    // Use a reasonable screen width assumption of 1920px if we can't detect
    let max_x = 1920.0 - settings_width - 20.0;
    if x + settings_width > max_x {
        x = (card.geometry.x - settings_width - 16.0).max(20.0);
    }

    // If would go off bottom, adjust y position
    let max_y = 1080.0 - settings_height - 20.0;
    if y + settings_height > max_y {
        y = max_y.max(20.0);
    }

    CountdownCardGeometry {
        x,
        y,
        width: settings_width,
        height: settings_height,
    }
}
