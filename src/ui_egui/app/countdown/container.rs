//! Container mode for countdown cards - displays all cards in a single resizable window.

use super::card_rendering::{render_card_content, CardUiAction};
use super::container_layout::{
    calculate_insertion_indicator_rect,
    CARD_PADDING, CONTAINER_MIN_HEIGHT, CONTAINER_MIN_WIDTH, MIN_CARD_HEIGHT, MIN_CARD_WIDTH,
    VISIBILITY_CHECK_FRAMES,
};
pub use super::card_rendering::format_card_tooltip;
pub use super::container_layout::{ContainerLayout, DragState};

use crate::services::countdown::{
    CountdownCardGeometry, CountdownCardId, CountdownCardState, CountdownCardVisuals,
    CountdownNotificationConfig,
};
use chrono::{DateTime, Local};

/// Actions that can result from container UI interactions
#[derive(Debug, Clone, Default)]
pub enum ContainerAction {
    /// No action needed
    #[default]
    None,
    /// Reorder cards to the specified order
    ReorderCards(Vec<CountdownCardId>),
    /// Delete a specific card
    DeleteCard(CountdownCardId),
    /// Open settings for a specific card
    OpenSettings(CountdownCardId),
    /// Open the event dialog for a specific card
    OpenEventDialog(CountdownCardId),
    /// Navigate to a specific date in the calendar
    GoToDate(chrono::NaiveDate),
    /// Refresh a specific card
    RefreshCard(CountdownCardId),
    /// Container geometry changed
    GeometryChanged(CountdownCardGeometry),
    /// Container was closed
    Closed,
}

/// Get the primary monitor width from context, with fallback to 1920
fn get_primary_monitor_width(ctx: &egui::Context) -> f32 {
    ctx.input(|input| {
        input.raw.viewports
            .values()
            .filter_map(|info| info.monitor_size)
            .next()
            .map(|s| s.x)
            .unwrap_or(1920.0)
    })
}

/// Render all countdown cards within a container viewport.
/// Returns any actions that need to be handled by the caller.
#[allow(clippy::too_many_arguments)]
pub fn render_container_window(
    ctx: &egui::Context,
    cards: &[CountdownCardState],
    card_order: &[CountdownCardId],
    layout: &mut ContainerLayout,
    drag_state: &mut DragState,
    now: DateTime<Local>,
    notification_config: &CountdownNotificationConfig,
    visual_defaults: &CountdownCardVisuals,
    container_geometry: Option<CountdownCardGeometry>,
    default_card_width: f32,
    default_card_height: f32,
) -> Vec<ContainerAction> {
    use std::time::Duration as StdDuration;

    // Store the settings-based card dimensions for initial sizing
    // Cards will scale to fit whatever container size the user chooses
    layout.min_card_width = MIN_CARD_WIDTH;
    layout.min_card_height = MIN_CARD_HEIGHT;

    // Request repaint for countdown updates
    ctx.request_repaint_after(StdDuration::from_secs(1));

    let mut actions = Vec::new();

    // Use absolute minimums for container - let users resize as small as they want
    let container_min_width = CONTAINER_MIN_WIDTH;
    let container_min_height = CONTAINER_MIN_HEIGHT;

    // Calculate container size based on settings card dimensions and card count
    let current_card_count = cards.len();
    let num_cards = current_card_count.max(1) as f32;
    let default_width = default_card_width + CARD_PADDING * 2.0;
    let ideal_vertical_height = default_card_height * num_cards + CARD_PADDING * (num_cards + 1.0);

    // Detect if card count changed (card added or removed)
    let card_count_changed = layout.initialized && layout.last_card_count != current_card_count;
    let cards_added = current_card_count > layout.last_card_count;
    let card_count_diff = (current_card_count as i32 - layout.last_card_count as i32).abs() as f32;
    layout.last_card_count = current_card_count;

    // Calculate geometry - grow container when cards are added based on orientation
    let initial_geometry = if let Some(stored) = container_geometry {
        if card_count_changed {
            // Determine current orientation from stored geometry
            let aspect_ratio = stored.width / stored.height;
            let is_horizontal = aspect_ratio > 1.5;
            
            if is_horizontal {
                // Landscape: grow/shrink width when cards change
                let width_change = (default_card_width + CARD_PADDING) * card_count_diff;
                let new_width = if cards_added {
                    stored.width + width_change
                } else {
                    (stored.width - width_change).max(container_min_width)
                };
                CountdownCardGeometry {
                    x: stored.x,
                    y: stored.y,
                    width: new_width,
                    height: stored.height,
                }
            } else {
                // Portrait: grow/shrink height when cards change
                let height_change = (default_card_height + CARD_PADDING) * card_count_diff;
                let new_height = if cards_added {
                    stored.height + height_change
                } else {
                    (stored.height - height_change).max(container_min_height)
                };
                CountdownCardGeometry {
                    x: stored.x,
                    y: stored.y,
                    width: stored.width,
                    height: new_height,
                }
            }
        } else {
            stored
        }
    } else {
        // No stored geometry - use defaults for vertical layout
        CountdownCardGeometry {
            x: 100.0,
            y: 100.0,
            width: default_width.max(container_min_width),
            height: ideal_vertical_height.max(container_min_height).min(600.0),
        }
    };

    let viewport_id = egui::ViewportId::from_hash_of("countdown_container");

    // Set position/size on first render OR when card count changes
    let needs_resize = !layout.initialized || card_count_changed;
    
    // Log the geometry being used
    if !layout.initialized {
        log::info!(
            "Container first render - stored geometry: {:?}, using initial_geometry: x={}, y={}, w={}, h={}",
            container_geometry,
            initial_geometry.x, initial_geometry.y, initial_geometry.width, initial_geometry.height
        );
    }
    
    // Only set position/size in the builder on first render or when resizing
    // Otherwise, let the OS/user control the window position to prevent shaking during drag
    let builder = if needs_resize {
        egui::ViewportBuilder::default()
            .with_title("Countdown Cards")
            .with_resizable(true)
            .with_visible(true)
            .with_min_inner_size(egui::vec2(container_min_width, container_min_height))
            .with_position(egui::pos2(initial_geometry.x, initial_geometry.y))
            .with_inner_size(egui::vec2(initial_geometry.width, initial_geometry.height))
    } else {
        egui::ViewportBuilder::default()
            .with_title("Countdown Cards")
            .with_resizable(true)
            .with_visible(true)
            .with_min_inner_size(egui::vec2(container_min_width, container_min_height))
    };

    // Log on first render or resize
    if needs_resize {
        log::info!(
            "Container setting position to ({}, {}) size ({}, {})",
            initial_geometry.x, initial_geometry.y, initial_geometry.width, initial_geometry.height
        );
        
        // Push geometry change when resizing due to card count change
        if card_count_changed {
            actions.push(ContainerAction::GeometryChanged(initial_geometry));
        }
    }

    // Render the container viewport
    ctx.show_viewport_immediate(viewport_id, builder, |child_ctx, _class| {
        // On first render, explicitly set position and ensure window is visible
        if !layout.initialized {
            // Force the window position using viewport commands (more reliable than builder)
            child_ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                egui::pos2(initial_geometry.x, initial_geometry.y)
            ));
            child_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(initial_geometry.width, initial_geometry.height)
            ));
            child_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            
            log::info!(
                "Container: sent viewport commands for position ({}, {}) size ({}, {})",
                initial_geometry.x, initial_geometry.y, initial_geometry.width, initial_geometry.height
            );
        }
        
        // Check for close request
        let close_requested = child_ctx.input(|i| {
            i.viewport().close_requested()
        });

        if close_requested {
            actions.push(ContainerAction::Closed);
            return;
        }

        // Track geometry changes and check for visibility issues
        let current_geometry = child_ctx.input(|i| {
            let info = i.viewport();
            if let (Some(pos), Some(size)) = (info.outer_rect, info.inner_rect) {
                Some(CountdownCardGeometry {
                    x: pos.min.x,
                    y: pos.min.y,
                    width: size.width(),
                    height: size.height(),
                })
            } else {
                None
            }
        });
        
        // Check if window has focus (indicates it's actually visible and usable)
        let has_focus = child_ctx.input(|i| i.viewport().focused.unwrap_or(false));
        
        // Track if window has ever gained focus this session
        if has_focus {
            layout.has_ever_had_focus = true;
        }

        if let Some(new_geom) = current_geometry {
            // Log the actual geometry reported by the viewport
            if !layout.initialized {
                log::info!(
                    "Container actual viewport geometry after first render: x={}, y={}, w={}, h={}",
                    new_geom.x, new_geom.y, new_geom.width, new_geom.height
                );
            }
            
            // Visibility check: if the window position doesn't match what we requested
            // after several frames, the window might be stuck off-screen on a secondary monitor
            if !layout.position_verified {
                layout.visibility_check_frames += 1;
                
                if layout.visibility_check_frames >= VISIBILITY_CHECK_FRAMES {
                    // Get actual primary monitor width for multi-monitor detection
                    let primary_width = get_primary_monitor_width(child_ctx);
                    
                    // Check if position is way off from what we stored (indicating OS moved it)
                    // We're more lenient now - only consider "stuck" if:
                    // 1. Position is on a secondary monitor area AND
                    // 2. We've been trying to show for multiple frames AND  
                    // 3. Window has NEVER gained focus this session (not just currently unfocused)
                    let position_seems_stuck = if let Some(stored) = container_geometry {
                        // Window reports being at stored position but we can't see/interact with it
                        // This happens when the position is on a monitor that's no longer available
                        // Use dynamic primary monitor width instead of hardcoded 1920
                        let possibly_on_secondary = stored.x > primary_width || stored.x < 0.0 || stored.y < 0.0;
                        let position_matches = (new_geom.x - stored.x).abs() < 50.0 
                            && (new_geom.y - stored.y).abs() < 50.0;
                        
                        // Log diagnostic info for multi-monitor debugging
                        if possibly_on_secondary {
                            log::debug!(
                                "Container position check: stored=({}, {}), current=({}, {}), primary_width={}, has_focus={}, ever_focused={}",
                                stored.x, stored.y, new_geom.x, new_geom.y, primary_width, has_focus, layout.has_ever_had_focus
                            );
                        }
                        
                        // Only consider stuck if on secondary area, position matches stored,
                        // AND window has NEVER gained focus (if it did once, user can see it)
                        // This prevents false positives when user just clicked elsewhere
                        possibly_on_secondary && position_matches && !layout.has_ever_had_focus
                    } else {
                        false
                    };
                    
                    if position_seems_stuck {
                        log::warn!(
                            "Container appears stuck at off-screen position ({}, {}), moving to primary monitor",
                            new_geom.x, new_geom.y
                        );
                        // Force move to primary monitor
                        let safe_geom = CountdownCardGeometry {
                            x: 100.0,
                            y: 100.0,
                            width: new_geom.width,
                            height: new_geom.height,
                        };
                        child_ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                            egui::pos2(safe_geom.x, safe_geom.y)
                        ));
                        child_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        actions.push(ContainerAction::GeometryChanged(safe_geom));
                    }
                    
                    layout.position_verified = true;
                }
            }
            
            // Decrement skip_geometry_frames counter if active
            let skip_geometry_updates = layout.skip_geometry_frames > 0;
            if layout.skip_geometry_frames > 0 {
                layout.skip_geometry_frames -= 1;
            }
            
            // Save geometry for any reasonable position (unless skipping)
            // We previously tried to restrict to "primary monitor" but monitor layouts vary
            // Just save if the values are finite and not extremely off-screen
            if !skip_geometry_updates {
                let should_save_geometry = new_geom.x.is_finite() 
                    && new_geom.y.is_finite() 
                    && new_geom.x.abs() < 10000.0 
                    && new_geom.y.abs() < 10000.0;
                
                let geometry_changed = container_geometry.map(|g| {
                    (g.x - new_geom.x).abs() > 1.0
                        || (g.y - new_geom.y).abs() > 1.0
                        || (g.width - new_geom.width).abs() > 1.0
                        || (g.height - new_geom.height).abs() > 1.0
                }).unwrap_or(true);
                
                // Log geometry tracking for debugging
                if geometry_changed {
                    log::debug!(
                        "Container geometry changed: stored={:?} -> current=({}, {}, {}, {}), should_save={}",
                        container_geometry, new_geom.x, new_geom.y, new_geom.width, new_geom.height, should_save_geometry
                    );
                }
                
                if should_save_geometry && geometry_changed {
                    actions.push(ContainerAction::GeometryChanged(new_geom));
                }
            }
        }

        // Render container content
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_gray(30)))
            .show(child_ctx, |ui| {
                let available_rect = ui.available_rect_before_wrap();

                // Build ordered list of card IDs
                let ordered_ids: Vec<CountdownCardId> = if card_order.is_empty() {
                    cards.iter().map(|c| c.id).collect()
                } else {
                    // Use provided order, but include any cards not in the order
                    let mut ids: Vec<CountdownCardId> = card_order
                        .iter()
                        .filter(|id| cards.iter().any(|c| c.id == **id))
                        .copied()
                        .collect();
                    for card in cards {
                        if !ids.contains(&card.id) {
                            ids.push(card.id);
                        }
                    }
                    ids
                };

                // Calculate layout
                layout.calculate_layout(available_rect, &ordered_ids);

                // Render each card
                for card_id in &ordered_ids {
                    if let Some(card) = cards.iter().find(|c| c.id == *card_id) {
                        if let Some(rect) = layout.get_card_rect(*card_id) {
                            let is_dragging = drag_state.is_dragging_card(*card_id);

                            // Make the card draggable
                            let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                            // Handle drag start
                            if response.drag_started() {
                                if let Some(pos) = response.interact_pointer_pos() {
                                    drag_state.start_drag(*card_id, pos);
                                }
                            }

                            // Handle drag update
                            if response.dragged() && drag_state.is_dragging_card(*card_id) {
                                if let Some(pos) = response.interact_pointer_pos() {
                                    drag_state.update_drag(pos);
                                    let insert_idx = layout.calculate_insert_index(pos, &ordered_ids);
                                    drag_state.insert_index = Some(insert_idx);
                                }
                            }

                            // Handle drag end
                            if response.drag_stopped() {
                                if let Some(dragged_id) = drag_state.end_drag() {
                                    if let Some(insert_idx) = drag_state.insert_index.take() {
                                        // Reorder cards
                                        let mut new_order = ordered_ids.clone();
                                        if let Some(current_idx) = new_order.iter().position(|id| *id == dragged_id) {
                                            new_order.remove(current_idx);
                                            let adjusted_idx = if insert_idx > current_idx {
                                                insert_idx.saturating_sub(1)
                                            } else {
                                                insert_idx
                                            };
                                            new_order.insert(adjusted_idx.min(new_order.len()), dragged_id);
                                            actions.push(ContainerAction::ReorderCards(new_order));
                                        }
                                    }
                                }
                            }

                            // Render the card content
                            let card_action = render_card_content(
                                ui,
                                card,
                                visual_defaults,
                                rect,
                                now,
                                notification_config,
                                is_dragging,
                            );

                            // Convert card action to container action
                            match card_action {
                                CardUiAction::None => {}
                                CardUiAction::OpenSettings => {
                                    actions.push(ContainerAction::OpenSettings(*card_id));
                                }
                                CardUiAction::OpenEventDialog => {
                                    actions.push(ContainerAction::OpenEventDialog(*card_id));
                                }
                                CardUiAction::GoToDate => {
                                    actions.push(ContainerAction::GoToDate(card.start_at.date_naive()));
                                }
                                CardUiAction::Delete => {
                                    actions.push(ContainerAction::DeleteCard(*card_id));
                                }
                                CardUiAction::Refresh => {
                                    actions.push(ContainerAction::RefreshCard(*card_id));
                                }
                            }
                        }
                    }
                }

                // Draw drag insertion indicator
                if drag_state.is_dragging() {
                    if let (Some(insert_idx), Some(drag_pos)) = (drag_state.insert_index, drag_state.current_drag_pos) {
                        let indicator_rect = calculate_insertion_indicator_rect(
                            layout,
                            &ordered_ids,
                            insert_idx,
                            available_rect,
                        );

                        if let Some(indicator_rect) = indicator_rect {
                            ui.painter().rect_filled(
                                indicator_rect,
                                2.0,
                                egui::Color32::from_rgb(100, 149, 237), // Cornflower blue
                            );
                        }

                        // Also show a ghost of the dragged card at cursor position
                        let _ = drag_pos; // Could use this for ghost rendering
                    }
                }
            });
    });

    // Mark as initialized after first render
    if !layout.initialized {
        layout.initialized = true;
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_action_default() {
        let action = ContainerAction::default();
        assert!(matches!(action, ContainerAction::None));
    }
}
