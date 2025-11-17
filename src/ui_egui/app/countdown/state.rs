use super::render::{
    render_countdown_card_ui, viewport_builder_for_card, viewport_builder_for_settings,
    viewport_title_matches, CountdownCardUiAction, COUNTDOWN_SETTINGS_HEIGHT,
};
use super::settings::{render_countdown_settings_ui, CountdownSettingsCommand};
use crate::services::countdown::{
    CountdownCardGeometry, CountdownCardId, CountdownCardState, CountdownService,
};
use chrono::Local;
use egui::{self, Context};
use log;
use std::collections::{HashMap, HashSet};
use std::time::Duration as StdDuration;

use super::super::{geometry_changed, geometry_from_viewport_info, viewport_info};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CountdownRenderSnapshot {
    waiting_on_geometry: bool,
    geometry: CountdownCardGeometry,
}

#[derive(Default)]
pub(in super::super) struct CountdownUiState {
    pending_visibility: HashSet<CountdownCardId>,
    geometry_attempts: HashMap<CountdownCardId, u32>,
    open_settings: HashSet<CountdownCardId>,
    settings_geometry: HashMap<CountdownCardId, CountdownCardGeometry>,
    settings_needs_layout: HashSet<CountdownCardId>,
    render_log_state: HashMap<CountdownCardId, CountdownRenderSnapshot>,
    pending_event_body_updates: Vec<(i64, Option<String>)>,
    geometry_samples: HashMap<CountdownCardId, GeometrySampleState>,
}

const MAX_PENDING_GEOMETRY_FRAMES: u32 = 120;
const GEOMETRY_STABILITY_FRAMES: u32 = 4;

impl CountdownUiState {
    pub(in super::super) fn new(service: &CountdownService) -> Self {
        let mut state = Self::default();
        for card in service.cards() {
            state.pending_visibility.insert(card.id);
        }
        state
    }

    pub(in super::super) fn mark_card_pending(&mut self, card_id: CountdownCardId) {
        self.pending_visibility.insert(card_id);
    }

    pub(in super::super) fn render_cards(&mut self, ctx: &Context, service: &mut CountdownService) {
        let cards = service.cards().to_vec();
        if cards.is_empty() {
            return;
        }

        let now = Local::now();
        let mut removals = Vec::new();

        for card in cards {
            let viewport_id = egui::ViewportId::from_hash_of(("countdown_card", card.id.0));
            let waiting_on_geometry = self.should_wait_on_card_geometry(card.id);
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
            let action =
                ctx.show_viewport_immediate(viewport_id, builder, move |child_ctx, class| {
                    render_countdown_card_ui(
                        child_ctx,
                        class,
                        viewport_id,
                        &card_clone,
                        now,
                        waiting_on_geometry,
                    )
                });

            let viewport_info = viewport_info(ctx, viewport_id);
            let close_via_window = viewport_info
                .as_ref()
                .map(|info| info.close_requested())
                .unwrap_or(false);
            let mut queued_close = close_via_window;

            match action {
                CountdownCardUiAction::None => {}
                CountdownCardUiAction::Close => queued_close = true,
                CountdownCardUiAction::OpenSettings => {
                    self.open_settings.insert(card.id);
                    let default_geometry = default_settings_geometry_for(&card);
                    self.settings_geometry
                        .entry(card.id)
                        .or_insert(default_geometry);
                    self.settings_needs_layout.insert(card.id);
                }
                CountdownCardUiAction::GeometrySettled => {
                    self.clear_geometry_wait_state(&card.id);
                    log::debug!("card {:?} geometry settled", card.id);
                    ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::Focus);
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
            service.remove_card(id);
            self.open_settings.remove(&id);
            self.settings_geometry.remove(&id);
            self.settings_needs_layout.remove(&id);
            self.clear_geometry_wait_state(&id);
            self.render_log_state.remove(&id);
            self.geometry_samples.remove(&id);
        }

        service.flush_geometry_updates();
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
            CountdownSettingsCommand::DeleteCard(id) => {
                service.remove_card(id);
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

    fn should_wait_on_card_geometry(&mut self, card_id: CountdownCardId) -> bool {
        if !self.pending_visibility.contains(&card_id) {
            return false;
        }

        let exceeded_limit = {
            let attempts = self.geometry_attempts.entry(card_id).or_insert(0);
            log::debug!(
                "card {:?} geometry attempt {} (limit {})",
                card_id,
                *attempts + 1,
                MAX_PENDING_GEOMETRY_FRAMES
            );
            if *attempts >= MAX_PENDING_GEOMETRY_FRAMES {
                true
            } else {
                *attempts += 1;
                false
            }
        };

        if exceeded_limit {
            self.clear_geometry_wait_state(&card_id);
            log::warn!(
                "Countdown card {:?} geometry did not settle after {} frames; forcing visibility",
                card_id,
                MAX_PENDING_GEOMETRY_FRAMES
            );
            false
        } else {
            true
        }
    }

    fn clear_geometry_wait_state(&mut self, card_id: &CountdownCardId) {
        self.pending_visibility.remove(card_id);
        self.geometry_attempts.remove(card_id);
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

fn default_settings_geometry_for(card: &CountdownCardState) -> CountdownCardGeometry {
    CountdownCardGeometry {
        x: card.geometry.x + card.geometry.width + 16.0,
        y: card.geometry.y,
        width: 280.0,
        height: COUNTDOWN_SETTINGS_HEIGHT,
    }
}
