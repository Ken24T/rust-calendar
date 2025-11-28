use super::container::{
    render_container_window, ContainerAction, ContainerLayout, DragState,
};
use super::render::{
    render_countdown_card_ui, viewport_builder_for_card, viewport_builder_for_settings,
    viewport_title_matches, CountdownCardUiAction, COUNTDOWN_SETTINGS_HEIGHT,
};
use super::settings::{render_countdown_settings_ui, CountdownSettingsCommand};
use crate::services::countdown::{
    CountdownCardGeometry, CountdownCardId, CountdownCardState, CountdownCardVisuals,
    CountdownDisplayMode, CountdownService,
};
use chrono::Local;
use egui::{self, Context};
use log;
use std::collections::{HashMap, HashSet};

use super::super::geometry::{geometry_changed, geometry_from_viewport_info, viewport_info};

/// A request to open the event dialog for a countdown card
#[derive(Debug, Clone)]
pub struct OpenEventDialogRequest {
    pub event_id: i64,
    pub card_id: CountdownCardId,
    pub visuals: CountdownCardVisuals,
}

/// A request to navigate to a specific date in the calendar
#[derive(Debug, Clone)]
pub struct GoToDateRequest {
    pub date: chrono::NaiveDate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CountdownRenderSnapshot {
    waiting_on_geometry: bool,
    geometry: CountdownCardGeometry,
}

#[derive(Default)]
pub(in super::super) struct CountdownUiState {
    pending_geometry: HashMap<CountdownCardId, PendingGeometryState>,
    open_settings: HashSet<CountdownCardId>,
    settings_geometry: HashMap<CountdownCardId, CountdownCardGeometry>,
    settings_needs_layout: HashSet<CountdownCardId>,
    render_log_state: HashMap<CountdownCardId, CountdownRenderSnapshot>,
    pending_event_body_updates: Vec<(i64, Option<String>)>,
    geometry_samples: HashMap<CountdownCardId, GeometrySampleState>,
    // Container mode fields
    container_layout: ContainerLayout,
    container_drag_state: DragState,
    // Pending delete requests from settings dialogs
    pending_delete_requests: Vec<DeleteCardRequest>,
}

const MAX_PENDING_GEOMETRY_FRAMES: u32 = 120;
const GEOMETRY_STABILITY_FRAMES: u32 = 4;

/// Request to confirm countdown card deletion
#[derive(Debug, Clone)]
pub struct DeleteCardRequest {
    pub card_id: CountdownCardId,
    pub card_title: String,
}

/// Result of rendering countdown cards, containing various navigation requests
#[derive(Default)]
pub struct CountdownRenderResult {
    pub event_dialog_requests: Vec<OpenEventDialogRequest>,
    pub go_to_date_requests: Vec<GoToDateRequest>,
    pub delete_card_requests: Vec<DeleteCardRequest>,
}

impl CountdownUiState {
    pub(in super::super) fn new(service: &CountdownService) -> Self {
        let mut state = Self::default();
        for card in service.cards() {
            state
                .pending_geometry
                .insert(card.id, PendingGeometryState::new(card.geometry));
        }
        state
    }

    /// Reset container state so it will re-initialize on next render
    pub(in super::super) fn reset_container_state(&mut self) {
        self.container_layout.initialized = false;
    }
    
    /// Drain pending delete requests from settings dialogs
    pub(in super::super) fn drain_delete_requests(&mut self) -> Vec<DeleteCardRequest> {
        std::mem::take(&mut self.pending_delete_requests)
    }

    pub(in super::super) fn mark_card_pending(
        &mut self,
        card_id: CountdownCardId,
        geometry: CountdownCardGeometry,
    ) {
        self.pending_geometry
            .insert(card_id, PendingGeometryState::new(geometry));
    }

    fn pending_geometry_target(&self, card_id: CountdownCardId) -> Option<CountdownCardGeometry> {
        self.pending_geometry
            .get(&card_id)
            .map(|state| state.target)
    }

    fn should_hide_card_geometry(&mut self, card_id: CountdownCardId) -> bool {
        if let Some(state) = self.pending_geometry.get_mut(&card_id) {
            if state.force_visible {
                return false;
            }

            state.attempts += 1;
            if state.attempts >= MAX_PENDING_GEOMETRY_FRAMES {
                state.force_visible = true;
                log::warn!(
                    "Countdown card {:?} geometry did not settle after {} frames; showing window but continuing to enforce configured size",
                    card_id,
                    MAX_PENDING_GEOMETRY_FRAMES
                );
                return false;
            }

            true
        } else {
            false
        }
    }

    pub(in super::super) fn render_cards(
        &mut self,
        ctx: &Context,
        service: &mut CountdownService,
        default_card_width: f32,
        default_card_height: f32,
    ) -> CountdownRenderResult {
        // Branch based on display mode
        match service.display_mode() {
            CountdownDisplayMode::IndividualWindows => {
                self.render_individual_windows(ctx, service)
            }
            CountdownDisplayMode::Container => {
                self.render_container_mode(ctx, service, default_card_width, default_card_height)
            }
        }
    }

    /// Render countdown cards in container mode (all cards in a single window)
    fn render_container_mode(
        &mut self,
        ctx: &Context,
        service: &mut CountdownService,
        default_card_width: f32,
        default_card_height: f32,
    ) -> CountdownRenderResult {
        let cards = service.cards().to_vec();
        
        // Don't show empty container - wait until there are cards to display
        if cards.is_empty() {
            return CountdownRenderResult::default();
        }
        
        let now = Local::now();
        let notification_config = service.notification_config().clone();
        let visual_defaults = service.visual_defaults().clone();
        let container_geometry = service.container_geometry();
        let card_order = service.card_order().to_vec();

        let mut event_dialog_requests = Vec::new();
        let mut go_to_date_requests = Vec::new();

        // Render the container
        let actions = render_container_window(
            ctx,
            &cards,
            &card_order,
            &mut self.container_layout,
            &mut self.container_drag_state,
            now,
            &notification_config,
            &visual_defaults,
            container_geometry,
            default_card_width,
            default_card_height,
        );

        // Collect delete requests
        let mut delete_card_requests = Vec::new();
        
        // Process container actions
        for action in actions {
            match action {
                ContainerAction::None => {}
                ContainerAction::ReorderCards(new_order) => {
                    service.reorder_cards(new_order);
                }
                ContainerAction::DeleteCard(card_id) => {
                    log::info!("Delete confirmation requested for card {:?}", card_id);
                    if let Some(card) = cards.iter().find(|c| c.id == card_id) {
                        delete_card_requests.push(DeleteCardRequest {
                            card_id,
                            card_title: card.event_title.clone(),
                        });
                    }
                    // Clean up UI state (will be finalized if confirmed)
                    self.open_settings.remove(&card_id);
                    self.settings_geometry.remove(&card_id);
                    self.settings_needs_layout.remove(&card_id);
                }
                ContainerAction::OpenSettings(card_id) => {
                    if let Some(card) = cards.iter().find(|c| c.id == card_id) {
                        self.open_settings.insert(card_id);
                        let default_geometry = default_settings_geometry_for(card);
                        self.settings_geometry
                            .entry(card_id)
                            .or_insert(default_geometry);
                        self.settings_needs_layout.insert(card_id);
                    }
                }
                ContainerAction::OpenEventDialog(card_id) => {
                    if let Some(card) = cards.iter().find(|c| c.id == card_id) {
                        if let Some(event_id) = card.event_id {
                            event_dialog_requests.push(OpenEventDialogRequest {
                                event_id,
                                card_id,
                                visuals: card.visuals.clone(),
                            });
                        } else {
                            // Fall back to card settings if no event
                            self.open_settings.insert(card_id);
                            let default_geometry = default_settings_geometry_for(card);
                            self.settings_geometry
                                .entry(card_id)
                                .or_insert(default_geometry);
                            self.settings_needs_layout.insert(card_id);
                        }
                    }
                }
                ContainerAction::GoToDate(date) => {
                    go_to_date_requests.push(GoToDateRequest { date });
                }
                ContainerAction::RefreshCard(_card_id) => {
                    ctx.request_repaint();
                }
                ContainerAction::GeometryChanged(geometry) => {
                    service.update_container_geometry(geometry);
                }
                ContainerAction::Closed => {
                    // User closed the container - switch back to individual windows mode
                    log::info!("Container closed, switching to individual windows mode");
                    service.set_display_mode(CountdownDisplayMode::IndividualWindows);
                    // Reset initialized flag so next open will re-apply stored geometry
                    self.container_layout.initialized = false;
                }
            }
        }

        CountdownRenderResult {
            event_dialog_requests,
            go_to_date_requests,
            delete_card_requests,
        }
    }

    /// Render countdown cards as individual windows (original behavior)
    fn render_individual_windows(
        &mut self,
        ctx: &Context,
        service: &mut CountdownService,
    ) -> CountdownRenderResult {
        let cards = service.cards().to_vec();
        
        if cards.is_empty() {
            return CountdownRenderResult::default();
        }

        let now = Local::now();
        let mut removals = Vec::new();
        let mut event_dialog_requests = Vec::new();
        let mut go_to_date_requests = Vec::new();
        let mut delete_card_requests = Vec::new();

        for card in cards {
            let viewport_id = egui::ViewportId::from_hash_of(("countdown_card", card.id.0));
            let waiting_on_geometry = self.should_hide_card_geometry(card.id);
            let target_geometry = self.pending_geometry_target(card.id);
            let snapshot = CountdownRenderSnapshot {
                waiting_on_geometry,
                geometry: card.geometry,
            };
            let should_log = self
                .render_log_state
                .get(&card.id)
                .map(|last| last != &snapshot)
                .unwrap_or(true);
            if should_log {
                log::debug!(
                    "rendering card {:?} title='{}' waiting={} geom={:?}",
                    card.id,
                    card.effective_title(),
                    waiting_on_geometry,
                    card.geometry
                );
                self.render_log_state.insert(card.id, snapshot);
            }

            let builder = viewport_builder_for_card(&card, waiting_on_geometry);
            let card_clone = card.clone();
            let notification_config = service.notification_config().clone();
            let action =
                ctx.show_viewport_immediate(viewport_id, builder, move |child_ctx, class| {
                    render_countdown_card_ui(
                        child_ctx,
                        class,
                        viewport_id,
                        &card_clone,
                        now,
                        waiting_on_geometry,
                        target_geometry,
                        &notification_config,
                    )
                });

            let viewport_info = viewport_info(ctx, viewport_id);
            let close_via_window = viewport_info
                .as_ref()
                .map(|info| info.close_requested())
                .unwrap_or(false);
            let queued_close = close_via_window;

            match action {
                CountdownCardUiAction::None => {}
                CountdownCardUiAction::Delete => {
                    log::info!(
                        "Delete confirmation requested for card {:?} (event {:?})",
                        card.id,
                        card.event_id
                    );
                    delete_card_requests.push(DeleteCardRequest {
                        card_id: card.id,
                        card_title: card.event_title.clone(),
                    });
                }
                CountdownCardUiAction::OpenSettings => {
                    self.open_settings.insert(card.id);
                    let default_geometry = default_settings_geometry_for(&card);
                    self.settings_geometry
                        .entry(card.id)
                        .or_insert(default_geometry);
                    self.settings_needs_layout.insert(card.id);
                }
                CountdownCardUiAction::OpenEventDialog => {
                    if let Some(event_id) = card.event_id {
                        event_dialog_requests.push(OpenEventDialogRequest {
                            event_id,
                            card_id: card.id,
                            visuals: card.visuals.clone(),
                        });
                    } else {
                        // Fall back to card settings if no event
                        self.open_settings.insert(card.id);
                        let default_geometry = default_settings_geometry_for(&card);
                        self.settings_geometry
                            .entry(card.id)
                            .or_insert(default_geometry);
                        self.settings_needs_layout.insert(card.id);
                    }
                }
                CountdownCardUiAction::GeometrySettled => {
                    self.clear_geometry_wait_state(&card.id);
                    log::debug!("card {:?} geometry settled", card.id);
                    ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Focus);
                }
                CountdownCardUiAction::GoToDate => {
                    go_to_date_requests.push(GoToDateRequest {
                        date: card.start_at.date_naive(),
                    });
                }
                CountdownCardUiAction::Refresh => {
                    log::info!("Refresh action triggered for card {:?}", card.id);
                    ctx.request_repaint();
                }
            }

            if queued_close {
                removals.push(card.id);
                continue;
            }

            if let Some(info) = viewport_info.as_ref() {
                if !waiting_on_geometry && viewport_title_matches(info, &card.event_title) {
                    if let Some(current_geometry) = geometry_from_viewport_info(info) {
                        log::debug!(
                            "card {:?} sampled viewport geometry {:?}",
                            card.id,
                            current_geometry
                        );
                        if self.record_geometry_sample(card.id, current_geometry)
                            && geometry_changed(card.geometry, current_geometry)
                            && service.queue_geometry_update(card.id, current_geometry)
                        {
                            log::debug!(
                                "queue geometry update for card {:?}: {:?} -> {:?}",
                                card.id,
                                card.geometry,
                                current_geometry
                            );
                        }
                    }
                }
            }
        }

        for id in removals {
            log::info!("Processing removal for card {:?}", id);
            service.remove_card(id);
            self.open_settings.remove(&id);
            self.settings_geometry.remove(&id);
            self.settings_needs_layout.remove(&id);
            self.clear_geometry_wait_state(&id);
            self.render_log_state.remove(&id);
            self.geometry_samples.remove(&id);
        }

        service.flush_geometry_updates();

        CountdownRenderResult {
            event_dialog_requests,
            go_to_date_requests,
            delete_card_requests,
        }
    }

    pub(in super::super) fn render_settings_dialogs(
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
                let viewport_id = egui::ViewportId::from_hash_of(("countdown_settings", card.id.0));
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
                let result =
                    ctx.show_viewport_immediate(viewport_id, builder, move |child_ctx, class| {
                        render_countdown_settings_ui(child_ctx, class, &card_clone, &defaults_clone)
                    });

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

    pub(in super::super) fn drain_pending_event_bodies(&mut self) -> Vec<(i64, Option<String>)> {
        std::mem::take(&mut self.pending_event_body_updates)
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
            CountdownSettingsCommand::SetCompactMode(id, value) => {
                service.set_compact_mode(id, value);
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

    fn clear_geometry_wait_state(&mut self, card_id: &CountdownCardId) {
        self.pending_geometry.remove(card_id);
        self.geometry_samples.remove(card_id);
    }
    fn record_geometry_sample(
        &mut self,
        card_id: CountdownCardId,
        sample: CountdownCardGeometry,
    ) -> bool {
        let entry = self
            .geometry_samples
            .entry(card_id)
            .or_insert_with(|| GeometrySampleState::new(sample));

        // Check if we have a target geometry we're trying to enforce
        let target = self.pending_geometry.get(&card_id).map(|p| p.target);

        if geometry_changed(entry.last, sample) {
            log::debug!(
                "card {:?} geometry sample changed {:?} -> {:?}; resetting stability",
                card_id,
                entry.last,
                sample
            );
            entry.last = sample;
            entry.stable_frames = 1;
            entry.delivered = false;
            return false;
        }

        // If we have a target geometry, only accept stability if the sample matches the target
        // This prevents accepting geometry that was forced by the OS/window manager
        if let Some(target_geom) = target {
            let size_matches = (sample.width - target_geom.width).abs() < 5.0
                && (sample.height - target_geom.height).abs() < 5.0;

            if !size_matches {
                log::debug!(
                    "card {:?} geometry stable at {:?} but doesn't match target {:?}; continuing to enforce",
                    card_id,
                    sample,
                    target_geom
                );
                // Keep resetting stability so we don't accept the wrong size
                entry.stable_frames = 1;
                return false;
            }
        }

        if entry.stable_frames < GEOMETRY_STABILITY_FRAMES {
            entry.stable_frames += 1;
            log::debug!(
                "card {:?} geometry holding steady (frame {}/{})",
                card_id,
                entry.stable_frames,
                GEOMETRY_STABILITY_FRAMES
            );
            return false;
        }

        if entry.delivered {
            return false;
        }

        log::debug!(
            "card {:?} geometry considered stable enough to persist {:?}",
            card_id,
            entry.last
        );
        entry.delivered = true;
        true
    }
}

#[derive(Clone, Copy)]
struct GeometrySampleState {
    last: CountdownCardGeometry,
    stable_frames: u32,
    delivered: bool,
}

impl GeometrySampleState {
    fn new(sample: CountdownCardGeometry) -> Self {
        Self {
            last: sample,
            stable_frames: 1,
            delivered: false,
        }
    }
}

#[derive(Clone, Copy)]
struct PendingGeometryState {
    target: CountdownCardGeometry,
    attempts: u32,
    force_visible: bool,
}

impl PendingGeometryState {
    fn new(target: CountdownCardGeometry) -> Self {
        Self {
            target,
            attempts: 0,
            force_visible: false,
        }
    }
}

fn default_settings_geometry_for(card: &CountdownCardState) -> CountdownCardGeometry {
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
