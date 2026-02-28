use super::container::{
    render_container_window, ContainerAction, ContainerLayout, DragState,
};
use super::render::{
    render_countdown_card_ui, viewport_builder_for_card,
    viewport_title_matches, CountdownCardUiAction,
};
use crate::services::countdown::{
    CountdownCardGeometry, CountdownCardId, CountdownCardState, CountdownCardVisuals,
    CountdownCategoryId, CountdownDisplayMode, CountdownService,
};
use chrono::Local;
use egui::{self, Context};
use std::collections::{HashMap, HashSet};

use super::super::geometry::{geometry_changed, geometry_from_viewport_info, viewport_info};
use super::state_settings::default_settings_geometry_for;

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
    pub(super) open_settings: HashSet<CountdownCardId>,
    pub(super) settings_geometry: HashMap<CountdownCardId, CountdownCardGeometry>,
    pub(super) settings_needs_layout: HashSet<CountdownCardId>,
    pub(super) render_log_state: HashMap<CountdownCardId, CountdownRenderSnapshot>,
    pub(super) pending_event_body_updates: Vec<(i64, Option<String>)>,
    geometry_samples: HashMap<CountdownCardId, GeometrySampleState>,
    // Container mode fields
    container_layout: ContainerLayout,
    container_drag_state: DragState,
    // Category containers mode — per-category layout and drag state
    category_layouts: HashMap<CountdownCategoryId, ContainerLayout>,
    category_drag_states: HashMap<CountdownCategoryId, DragState>,
    // Pending delete requests from settings dialogs
    pub(super) pending_delete_requests: Vec<DeleteCardRequest>,
    // Skip geometry updates for this many frames (used after reset)
    skip_geometry_frames: u32,
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
    
    /// Full reset of all UI state when card positions are reset.
    /// This clears pending geometry, resets container layout completely,
    /// and clears any geometry sampling state to prevent flashing/feedback loops.
    /// 
    /// IMPORTANT: We clear pending_geometry entirely (not re-populate it) so that
    /// cards render immediately at their new positions without the hide/show cycle
    /// that causes flickering.
    pub(in super::super) fn reset_all_ui_state(&mut self) {
        log::info!("Resetting all countdown UI state for position reset");
        
        // Completely reset container layout to default state
        self.container_layout = ContainerLayout::default();
        // Skip geometry change detection for 30 frames to let the window settle
        self.container_layout.skip_geometry_frames = 30;
        
        // Clear ALL pending geometry - do NOT re-populate!
        // This ensures cards render immediately at new positions without visibility toggling
        self.pending_geometry.clear();
        
        // Clear geometry sampling state  
        self.geometry_samples.clear();
        
        // Clear render log state so next render logs fresh info
        self.render_log_state.clear();
        
        // Reset container drag state
        self.container_drag_state = DragState::default();
        
        // Reset category container state
        self.category_layouts.clear();
        self.category_drag_states.clear();
        
        // Skip geometry updates for individual cards too
        self.skip_geometry_frames = 30;
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
            CountdownDisplayMode::CategoryContainers => {
                self.render_category_containers_mode(
                    ctx,
                    service,
                    default_card_width,
                    default_card_height,
                )
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
        let categories: Vec<(CountdownCategoryId, String)> = service
            .categories()
            .iter()
            .map(|c| (c.id, c.name.clone()))
            .collect();

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
            &categories,
            "Countdown Cards",
            "countdown_container",
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
                ContainerAction::ChangeCategory(card_id, cat_id) => {
                    log::info!("Change category for card {:?} to {:?}", card_id, cat_id);
                    service.set_card_category(card_id, cat_id);
                }
            }
        }

        CountdownRenderResult {
            event_dialog_requests,
            go_to_date_requests,
            delete_card_requests,
        }
    }

    /// Render countdown cards in category containers mode — one container window per category.
    fn render_category_containers_mode(
        &mut self,
        ctx: &Context,
        service: &mut CountdownService,
        default_card_width: f32,
        default_card_height: f32,
    ) -> CountdownRenderResult {
        let all_cards = service.cards().to_vec();

        if all_cards.is_empty() {
            return CountdownRenderResult::default();
        }

        let now = Local::now();
        let notification_config = service.notification_config().clone();
        let visual_defaults = service.visual_defaults().clone();
        let categories: Vec<(CountdownCategoryId, String)> = service
            .categories()
            .iter()
            .map(|c| (c.id, c.name.clone()))
            .collect();

        // Snapshot per-category data so we can release the service borrow
        struct CategorySnapshot {
            id: CountdownCategoryId,
            name: String,
            container_geometry: Option<CountdownCardGeometry>,
            card_width: f32,
            card_height: f32,
        }

        let cat_snapshots: Vec<CategorySnapshot> = service
            .categories()
            .iter()
            .map(|c| CategorySnapshot {
                id: c.id,
                name: c.name.clone(),
                container_geometry: c.container_geometry,
                card_width: c.default_card_width,
                card_height: c.default_card_height,
            })
            .collect();

        let card_order = service.card_order().to_vec();

        let mut event_dialog_requests = Vec::new();
        let mut go_to_date_requests = Vec::new();
        let mut delete_card_requests = Vec::new();

        // Collect deferred actions to apply after rendering all categories
        let mut category_changes: Vec<(CountdownCardId, CountdownCategoryId)> = Vec::new();
        let mut geometry_updates: Vec<(CountdownCategoryId, CountdownCardGeometry)> = Vec::new();
        let mut reorder_updates: Vec<Vec<CountdownCardId>> = Vec::new();
        let mut closed_categories: Vec<CountdownCategoryId> = Vec::new();

        for cat_snap in &cat_snapshots {
            // Filter cards for this category
            let cat_cards: Vec<CountdownCardState> = all_cards
                .iter()
                .filter(|c| c.category_id == cat_snap.id)
                .cloned()
                .collect();

            // Skip categories with no cards
            if cat_cards.is_empty() {
                continue;
            }

            // Build per-category card order (preserving global ordering, filtered)
            let cat_card_order: Vec<CountdownCardId> = if card_order.is_empty() {
                cat_cards.iter().map(|c| c.id).collect()
            } else {
                card_order
                    .iter()
                    .filter(|id| cat_cards.iter().any(|c| c.id == **id))
                    .copied()
                    .collect()
            };

            // Get or create per-category layout and drag state
            let layout = self
                .category_layouts
                .entry(cat_snap.id)
                .or_default();
            let drag_state = self
                .category_drag_states
                .entry(cat_snap.id)
                .or_default();

            let window_title = format!("⏱ {}", cat_snap.name);
            let viewport_id_suffix = format!("countdown_category_{}", cat_snap.id.0);

            let actions = render_container_window(
                ctx,
                &cat_cards,
                &cat_card_order,
                layout,
                drag_state,
                now,
                &notification_config,
                &visual_defaults,
                cat_snap.container_geometry,
                cat_snap.card_width.max(default_card_width),
                cat_snap.card_height.max(default_card_height),
                &categories,
                &window_title,
                &viewport_id_suffix,
            );

            // Process container actions for this category
            for action in actions {
                match action {
                    ContainerAction::None => {}
                    ContainerAction::ReorderCards(new_order) => {
                        reorder_updates.push(new_order);
                    }
                    ContainerAction::DeleteCard(card_id) => {
                        log::info!("Delete requested for card {:?} in category {:?}", card_id, cat_snap.id);
                        if let Some(card) = cat_cards.iter().find(|c| c.id == card_id) {
                            delete_card_requests.push(DeleteCardRequest {
                                card_id,
                                card_title: card.event_title.clone(),
                            });
                        }
                        self.open_settings.remove(&card_id);
                        self.settings_geometry.remove(&card_id);
                        self.settings_needs_layout.remove(&card_id);
                    }
                    ContainerAction::OpenSettings(card_id) => {
                        if let Some(card) = cat_cards.iter().find(|c| c.id == card_id) {
                            self.open_settings.insert(card_id);
                            let default_geometry = default_settings_geometry_for(card);
                            self.settings_geometry
                                .entry(card_id)
                                .or_insert(default_geometry);
                            self.settings_needs_layout.insert(card_id);
                        }
                    }
                    ContainerAction::OpenEventDialog(card_id) => {
                        if let Some(card) = cat_cards.iter().find(|c| c.id == card_id) {
                            if let Some(event_id) = card.event_id {
                                event_dialog_requests.push(OpenEventDialogRequest {
                                    event_id,
                                    card_id,
                                    visuals: card.visuals.clone(),
                                });
                            } else {
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
                    ContainerAction::RefreshCard(_) => {
                        ctx.request_repaint();
                    }
                    ContainerAction::GeometryChanged(geometry) => {
                        geometry_updates.push((cat_snap.id, geometry));
                    }
                    ContainerAction::Closed => {
                        closed_categories.push(cat_snap.id);
                    }
                    ContainerAction::ChangeCategory(card_id, cat_id) => {
                        log::info!("Change category for card {:?} to {:?}", card_id, cat_id);
                        category_changes.push((card_id, cat_id));
                    }
                }
            }
        }

        // Apply deferred mutations
        for (card_id, cat_id) in category_changes {
            service.set_card_category(card_id, cat_id);
        }
        for (cat_id, geometry) in geometry_updates {
            service.update_category_container_geometry(cat_id, geometry);
        }
        for new_order in reorder_updates {
            service.reorder_cards(new_order);
        }
        // If any category container was closed, switch back to individual windows
        if !closed_categories.is_empty() {
            log::info!(
                "Category container(s) closed, switching to individual windows mode"
            );
            service.set_display_mode(CountdownDisplayMode::IndividualWindows);
            // Reset category layouts so next open will re-apply stored geometry
            self.category_layouts.clear();
            self.category_drag_states.clear();
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

        // Decrement skip counter at start of frame
        let skip_geometry_updates = self.skip_geometry_frames > 0;
        if self.skip_geometry_frames > 0 {
            self.skip_geometry_frames -= 1;
        }

        let now = Local::now();
        let mut removals = Vec::new();
        let mut event_dialog_requests = Vec::new();
        let mut go_to_date_requests = Vec::new();
        let mut delete_card_requests = Vec::new();

        // Build category list for context menus
        let categories: Vec<(CountdownCategoryId, String)> = service
            .categories()
            .iter()
            .map(|c| (c.id, c.name.clone()))
            .collect();

        let mut category_changes: Vec<(CountdownCardId, CountdownCategoryId)> = Vec::new();

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
            let categories_clone = categories.clone();
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
                        &categories_clone,
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
                CountdownCardUiAction::ChangeCategory(cat_id) => {
                    log::info!("Change category for card {:?} to {:?}", card.id, cat_id);
                    category_changes.push((card.id, cat_id));
                }
            }

            if queued_close {
                removals.push(card.id);
                continue;
            }

            // Skip geometry updates for several frames after reset to prevent feedback loops
            if !skip_geometry_updates {
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

        // Apply deferred category changes
        for (card_id, cat_id) in category_changes {
            service.set_card_category(card_id, cat_id);
        }

        service.flush_geometry_updates();

        CountdownRenderResult {
            event_dialog_requests,
            go_to_date_requests,
            delete_card_requests,
        }
    }



    pub(in super::super) fn drain_pending_event_bodies(&mut self) -> Vec<(i64, Option<String>)> {
        std::mem::take(&mut self.pending_event_body_updates)
    }



    pub(super) fn clear_geometry_wait_state(&mut self, card_id: &CountdownCardId) {
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


