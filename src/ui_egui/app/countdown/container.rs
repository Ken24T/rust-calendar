//! Container mode for countdown cards - displays all cards in a single resizable window.

use crate::services::countdown::{
    CountdownCardGeometry, CountdownCardId, CountdownCardState, CountdownCardVisuals,
    CountdownNotificationConfig, CountdownWarningState, RgbaColor, MAX_DAYS_FONT_SIZE,
};
use chrono::{DateTime, Local};
use std::collections::HashMap;

/// Format the detailed countdown tooltip for a card
pub fn format_card_tooltip(card: &CountdownCardState, now: DateTime<Local>) -> String {
    let mut lines = Vec::new();
    
    // Event date range if available
    if let (Some(start), Some(end)) = (card.event_start, card.event_end) {
        let start_str = start.format("%d %b %Y %H:%M").to_string();
        let end_str = if start.date_naive() == end.date_naive() {
            // Same day - just show time for end
            end.format("%H:%M").to_string()
        } else {
            end.format("%d %b %Y %H:%M").to_string()
        };
        lines.push(format!("📅 {} → {}", start_str, end_str));
    }
    
    // Detailed countdown (DD:HH:MM)
    let duration = card.start_at.signed_duration_since(now);
    if duration.num_seconds() > 0 {
        let total_seconds = duration.num_seconds();
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        let minutes = (total_seconds % 3600) / 60;
        
        if days > 0 {
            lines.push(format!("⏱ {}d {:02}h {:02}m remaining", days, hours, minutes));
        } else if hours > 0 {
            lines.push(format!("⏱ {:02}h {:02}m remaining", hours, minutes));
        } else {
            lines.push(format!("⏱ {:02}m remaining", minutes));
        }
    } else {
        lines.push("⏱ Event has started!".to_string());
    }
    
    // Target time
    lines.push(format!("🎯 Target: {}", card.start_at.format("%d %b %Y %H:%M")));
    
    // Comment/description if present
    if let Some(body) = card.comment.as_ref().map(|t| t.trim()).filter(|t| !t.is_empty()) {
        lines.push(String::new()); // blank line
        lines.push(format!("📝 {}", body));
    }
    
    lines.join("\n")
}

/// Layout orientation for cards within the container
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutOrientation {
    /// Cards stacked vertically (tall/narrow container)
    Vertical,
    /// Cards arranged horizontally (wide container)
    Horizontal,
}

impl Default for LayoutOrientation {
    fn default() -> Self {
        Self::Vertical
    }
}

/// Minimum dimensions for container (absolute minimums for usability)
pub const CONTAINER_MIN_WIDTH: f32 = 80.0;
pub const CONTAINER_MIN_HEIGHT: f32 = 60.0;
/// Minimum card dimensions within container
pub const MIN_CARD_WIDTH: f32 = 60.0;
pub const MIN_CARD_HEIGHT: f32 = 50.0;
pub const CARD_PADDING: f32 = 8.0;

// Card rendering constants
const CARD_ROUNDING: f32 = 8.0;
const CARD_MIN_COUNTDOWN_HEIGHT: f32 = 36.0;
const CARD_SPACING: f32 = 4.0;

/// Number of frames to wait before checking if window position is valid
const VISIBILITY_CHECK_FRAMES: u32 = 10;

/// Layout calculator for arranging cards within the container
#[derive(Debug, Clone)]
pub struct ContainerLayout {
    /// Current layout orientation
    pub orientation: LayoutOrientation,
    /// Computed rectangles for each card
    pub card_rects: HashMap<CountdownCardId, egui::Rect>,
    /// Minimum card width
    pub min_card_width: f32,
    /// Minimum card height
    pub min_card_height: f32,
    /// Padding between cards
    pub padding: f32,
    /// Whether the container has been initialized (first frame rendered)
    pub initialized: bool,
    /// Track the last known card count to detect additions/removals
    pub last_card_count: usize,
    /// Frame counter for visibility check
    pub visibility_check_frames: u32,
    /// Whether the position has been verified as working
    pub position_verified: bool,
}

impl Default for ContainerLayout {
    fn default() -> Self {
        Self {
            orientation: LayoutOrientation::Vertical,
            card_rects: HashMap::new(),
            min_card_width: MIN_CARD_WIDTH,
            min_card_height: MIN_CARD_HEIGHT,
            padding: CARD_PADDING,
            initialized: false,
            last_card_count: 0,
            visibility_check_frames: 0,
            position_verified: false,
        }
    }
}

impl ContainerLayout {
    /// Calculate the layout for cards within the container.
    /// 
    /// This method determines the orientation based on the container's aspect ratio:
    /// - Wide containers (aspect ratio > 1.5) use horizontal layout
    /// - Tall/square containers use vertical layout
    /// 
    /// Cards are evenly distributed within the available space, respecting minimum sizes.
    pub fn calculate_layout(
        &mut self,
        available_rect: egui::Rect,
        card_ids: &[CountdownCardId],
    ) {
        self.card_rects.clear();

        let count = card_ids.len();
        if count == 0 {
            return;
        }

        // Determine orientation based on aspect ratio
        let aspect_ratio = available_rect.width() / available_rect.height();
        self.orientation = if aspect_ratio > 1.5 {
            LayoutOrientation::Horizontal
        } else {
            LayoutOrientation::Vertical
        };

        // Calculate card positions
        match self.orientation {
            LayoutOrientation::Vertical => {
                self.layout_vertical(available_rect, card_ids);
            }
            LayoutOrientation::Horizontal => {
                self.layout_horizontal(available_rect, card_ids);
            }
        }
    }

    /// Layout cards vertically (stacked top to bottom)
    fn layout_vertical(&mut self, available: egui::Rect, card_ids: &[CountdownCardId]) {
        let count = card_ids.len() as f32;
        
        // Available space accounting for padding
        let total_padding = self.padding * (count + 1.0);
        let available_height = (available.height() - total_padding).max(0.0);
        
        // Calculate card dimensions
        let card_height = (available_height / count).max(self.min_card_height);
        let card_width = (available.width() - self.padding * 2.0).max(self.min_card_width);

        for (i, card_id) in card_ids.iter().enumerate() {
            let x = available.left() + self.padding;
            let y = available.top() + self.padding + i as f32 * (card_height + self.padding);

            self.card_rects.insert(
                *card_id,
                egui::Rect::from_min_size(
                    egui::pos2(x, y),
                    egui::vec2(card_width, card_height),
                ),
            );
        }
    }

    /// Layout cards horizontally (side by side)
    fn layout_horizontal(&mut self, available: egui::Rect, card_ids: &[CountdownCardId]) {
        let count = card_ids.len() as f32;
        
        // Available space accounting for padding
        let total_padding = self.padding * (count + 1.0);
        let available_width = (available.width() - total_padding).max(0.0);
        
        // Calculate card dimensions
        let card_width = (available_width / count).max(self.min_card_width);
        let card_height = (available.height() - self.padding * 2.0).max(self.min_card_height);

        for (i, card_id) in card_ids.iter().enumerate() {
            let x = available.left() + self.padding + i as f32 * (card_width + self.padding);
            let y = available.top() + self.padding;

            self.card_rects.insert(
                *card_id,
                egui::Rect::from_min_size(
                    egui::pos2(x, y),
                    egui::vec2(card_width, card_height),
                ),
            );
        }
    }

    /// Get the rect for a specific card, if it exists
    pub fn get_card_rect(&self, card_id: CountdownCardId) -> Option<egui::Rect> {
        self.card_rects.get(&card_id).copied()
    }

    /// Calculate which card index a position would insert before (for drag-drop)
    pub fn calculate_insert_index(
        &self,
        pos: egui::Pos2,
        card_order: &[CountdownCardId],
    ) -> usize {
        if card_order.is_empty() {
            return 0;
        }

        match self.orientation {
            LayoutOrientation::Vertical => {
                // Find insertion point based on Y position
                for (i, card_id) in card_order.iter().enumerate() {
                    if let Some(rect) = self.card_rects.get(card_id) {
                        let mid_y = rect.center().y;
                        if pos.y < mid_y {
                            return i;
                        }
                    }
                }
                card_order.len()
            }
            LayoutOrientation::Horizontal => {
                // Find insertion point based on X position
                for (i, card_id) in card_order.iter().enumerate() {
                    if let Some(rect) = self.card_rects.get(card_id) {
                        let mid_x = rect.center().x;
                        if pos.x < mid_x {
                            return i;
                        }
                    }
                }
                card_order.len()
            }
        }
    }
}

/// State for drag-and-drop reordering of cards
#[derive(Debug, Clone, Default)]
pub struct DragState {
    /// The card currently being dragged, if any
    pub dragging_card: Option<CountdownCardId>,
    /// Position where the drag started
    pub drag_start_pos: Option<egui::Pos2>,
    /// Current drag position
    pub current_drag_pos: Option<egui::Pos2>,
    /// Computed insertion index for reordering
    pub insert_index: Option<usize>,
}

impl DragState {
    /// Start a new drag operation
    pub fn start_drag(&mut self, card_id: CountdownCardId, pos: egui::Pos2) {
        self.dragging_card = Some(card_id);
        self.drag_start_pos = Some(pos);
        self.current_drag_pos = Some(pos);
        self.insert_index = None;
    }

    /// Update the drag position
    pub fn update_drag(&mut self, pos: egui::Pos2) {
        self.current_drag_pos = Some(pos);
    }

    /// End the drag operation and return the card that was being dragged
    pub fn end_drag(&mut self) -> Option<CountdownCardId> {
        let card = self.dragging_card.take();
        self.drag_start_pos = None;
        self.current_drag_pos = None;
        self.insert_index = None;
        card
    }

    /// Check if we're currently dragging
    pub fn is_dragging(&self) -> bool {
        self.dragging_card.is_some()
    }

    /// Check if a specific card is being dragged
    pub fn is_dragging_card(&self, card_id: CountdownCardId) -> bool {
        self.dragging_card == Some(card_id)
    }
}

/// Actions that can result from container UI interactions
#[derive(Debug, Clone)]
pub enum ContainerAction {
    /// No action needed
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

impl Default for ContainerAction {
    fn default() -> Self {
        Self::None
    }
}

/// Action from rendering a single card within the container
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardUiAction {
    None,
    OpenSettings,
    OpenEventDialog,
    GoToDate,
    Delete,
    Refresh,
}

/// Convert an RGBA color to egui Color32
fn rgba_to_color32(color: RgbaColor) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}

/// Resolve effective color: use default if flag is set, otherwise use card's value
fn resolve_color(
    card_color: RgbaColor,
    default_color: RgbaColor,
    use_default: bool,
) -> RgbaColor {
    if use_default {
        default_color
    } else {
        card_color
    }
}

/// Calculate warning colors for a card based on its state
fn calculate_card_colors(
    card: &CountdownCardState,
    visual_defaults: &CountdownCardVisuals,
    warning_state: CountdownWarningState,
    notification_config: &CountdownNotificationConfig,
    ctx: &egui::Context,
) -> (egui::Color32, egui::Color32, egui::Color32, egui::Color32, f32) {
    // Resolve effective colors using defaults when flags are set
    let effective_title_bg = resolve_color(
        card.visuals.title_bg_color,
        visual_defaults.title_bg_color,
        card.visuals.use_default_title_bg,
    );
    let effective_title_fg = resolve_color(
        card.visuals.title_fg_color,
        visual_defaults.title_fg_color,
        card.visuals.use_default_title_fg,
    );
    let effective_body_bg = resolve_color(
        card.visuals.body_bg_color,
        visual_defaults.body_bg_color,
        card.visuals.use_default_body_bg,
    );
    let effective_days_fg = resolve_color(
        card.visuals.days_fg_color,
        visual_defaults.days_fg_color,
        card.visuals.use_default_days_fg,
    );

    let title_bg = rgba_to_color32(effective_title_bg);
    let title_fg = rgba_to_color32(effective_title_fg);

    if !notification_config.enabled || !notification_config.use_visual_warnings {
        let body_bg = rgba_to_color32(effective_body_bg);
        let days_fg = rgba_to_color32(effective_days_fg);
        return (title_bg, title_fg, body_bg, days_fg, 1.0);
    }

    let (body_bg, days_fg, stroke_width) = match warning_state {
        CountdownWarningState::Critical => {
            let pulse_phase = (ctx.input(|i| i.time) * 2.0) % 1.0;
            let pulse_alpha = (pulse_phase * 255.0) as u8;
            let body_bg =
                egui::Color32::from_rgba_unmultiplied(255, 100, 100, 255 - pulse_alpha / 2);
            let days_fg = egui::Color32::from_rgb(139, 0, 0);
            ctx.request_repaint();
            (body_bg, days_fg, 4.0)
        }
        CountdownWarningState::Imminent => {
            let body_bg = egui::Color32::from_rgb(255, 165, 0);
            let days_fg = egui::Color32::from_rgb(139, 69, 0);
            (body_bg, days_fg, 3.0)
        }
        CountdownWarningState::Starting => {
            let pulse_phase = (ctx.input(|i| i.time) * 3.0) % 1.0;
            let pulse_alpha = (pulse_phase * 255.0) as u8;
            let body_bg = egui::Color32::from_rgba_unmultiplied(0, 255, 100, 255 - pulse_alpha / 3);
            let days_fg = egui::Color32::from_rgb(0, 100, 0);
            ctx.request_repaint();
            (body_bg, days_fg, 5.0)
        }
        CountdownWarningState::Approaching => {
            let body_bg = rgba_to_color32(effective_body_bg);
            let days_fg = rgba_to_color32(effective_days_fg);
            (body_bg, days_fg, 2.0)
        }
        CountdownWarningState::Normal => {
            let body_bg = rgba_to_color32(effective_body_bg);
            let days_fg = rgba_to_color32(effective_days_fg);
            (body_bg, days_fg, 1.0)
        }
    };

    (title_bg, title_fg, body_bg, days_fg, stroke_width)
}

/// Render a single card's content within a given rect.
/// This is used by the container to render each card at its computed position.
pub fn render_card_content(
    ui: &mut egui::Ui,
    card: &CountdownCardState,
    visual_defaults: &CountdownCardVisuals,
    rect: egui::Rect,
    now: DateTime<Local>,
    notification_config: &CountdownNotificationConfig,
    is_being_dragged: bool,
) -> CardUiAction {
    let mut action = CardUiAction::None;

    // Calculate warning state
    let warning_state = if notification_config.enabled && notification_config.use_visual_warnings {
        card.warning_state(now, &notification_config.warning_thresholds)
    } else {
        CountdownWarningState::Normal
    };

    // Get colors (resolving defaults as needed)
    let (title_bg, title_fg, body_bg, days_fg, stroke_width) =
        calculate_card_colors(card, visual_defaults, warning_state, notification_config, ui.ctx());

    let title_font_size = card.visuals.title_font_size.max(12.0);
    let font_size = card.visuals.days_font_size.clamp(32.0, MAX_DAYS_FONT_SIZE);

    // Calculate stroke color based on warning state
    let stroke_color = if is_being_dragged {
        egui::Color32::from_rgb(100, 149, 237) // Cornflower blue for drag indicator
    } else {
        match warning_state {
            CountdownWarningState::Critical => egui::Color32::from_rgb(200, 0, 0),
            CountdownWarningState::Imminent => egui::Color32::from_rgb(255, 140, 0),
            CountdownWarningState::Starting => egui::Color32::from_rgb(0, 200, 0),
            CountdownWarningState::Approaching => egui::Color32::from_rgb(255, 200, 0),
            CountdownWarningState::Normal => egui::Color32::from_gray(40),
        }
    };

    let actual_stroke_width = if is_being_dragged { 3.0 } else { stroke_width };

    // Allocate the rect for this card
    let child_ui = ui.child_ui(rect, egui::Layout::top_down(egui::Align::LEFT), None);
    let mut child_ui = child_ui;

    let rounding = egui::Rounding::from(CARD_ROUNDING);
    let frame = egui::Frame::none()
        .fill(body_bg)
        .rounding(rounding)
        .stroke(egui::Stroke::new(actual_stroke_width, stroke_color));

    let inner = frame.show(&mut child_ui, |ui| {
        let width = rect.width();
        let total_height = rect.height().max(60.0);

        let desired_title_height = (title_font_size * 1.4).clamp(22.0, 48.0);
        let max_title_height = (total_height - CARD_MIN_COUNTDOWN_HEIGHT - CARD_SPACING).max(20.0);
        let title_height = desired_title_height.min(max_title_height);
        let countdown_height = (total_height - title_height - CARD_SPACING).max(CARD_MIN_COUNTDOWN_HEIGHT);

        // Title bar
        let title_size = egui::vec2(width, title_height);
        ui.allocate_ui_with_layout(
            title_size,
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
            |title_ui| {
                egui::Frame::none()
                    .fill(title_bg)
                    .rounding(egui::Rounding {
                        nw: rounding.nw,
                        ne: rounding.ne,
                        sw: 0.0,
                        se: 0.0,
                    })
                    .show(title_ui, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(card.effective_title())
                                        .color(title_fg)
                                        .size(title_font_size)
                                        .strong(),
                                )
                                .truncate()
                                .wrap_mode(egui::TextWrapMode::Truncate),
                            );
                        });
                    });
            },
        );

        ui.add_space(CARD_SPACING);

        // Countdown number
        let countdown_size = egui::vec2(width, countdown_height);
        ui.allocate_ui_with_layout(
            countdown_size,
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
            |countdown_ui| {
                let duration = card.start_at.signed_duration_since(now);
                let total_hours = duration.num_hours();
                
                // Show HH:MM if less than 24 hours, otherwise show days
                let countdown_text = if total_hours < 24 && total_hours >= 0 {
                    let hours = total_hours;
                    let minutes = (duration.num_minutes() % 60).max(0);
                    format!("{:02}:{:02}", hours, minutes)
                } else if total_hours < 0 {
                    // Event has passed
                    "00:00".to_string()
                } else {
                    let days_remaining = (card.start_at.date_naive() - now.date_naive())
                        .num_days()
                        .max(0);
                    days_remaining.to_string()
                };

                // Calculate font size based on available space and number of characters
                let char_count = countdown_text.len();
                let available_width = width * 0.9;
                let estimated_text_width = font_size * 0.6 * char_count as f32;

                let adjusted_font_size = if estimated_text_width > available_width {
                    (available_width / (0.6 * char_count as f32))
                        .max(32.0)
                        .min(font_size)
                } else {
                    font_size
                };

                let countdown_response = countdown_ui.label(
                    egui::RichText::new(countdown_text)
                        .size(adjusted_font_size)
                        .color(days_fg),
                );

                // Enhanced tooltip with event details and countdown
                countdown_response.on_hover_ui_at_pointer(|ui| {
                    ui.label(format_card_tooltip(card, now));
                });
            },
        );
    });

    // Context menu for the card
    inner.response.context_menu(|ui| {
        if card.event_id.is_some() {
            if ui.button("📝 Edit event...").clicked() {
                action = CardUiAction::OpenEventDialog;
                ui.close_menu();
            }
        }
        if ui.button("⚙ Card settings...").clicked() {
            action = CardUiAction::OpenSettings;
            ui.close_menu();
        }
        if ui.button("📅 Go to date").clicked() {
            action = CardUiAction::GoToDate;
            ui.close_menu();
        }
        if ui.button("🔄 Refresh countdown").clicked() {
            action = CardUiAction::Refresh;
            ui.close_menu();
        }
        ui.separator();
        if ui.button("🗑 Delete card").clicked() {
            action = CardUiAction::Delete;
            ui.close_menu();
        }
    });

    action
}

/// Render all countdown cards within a container viewport.
/// Returns any actions that need to be handled by the caller.
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
    let ideal_height = default_card_height * num_cards + CARD_PADDING * (num_cards + 1.0);

    // Detect if card count changed (card added or removed)
    let card_count_changed = layout.initialized && layout.last_card_count != current_card_count;
    layout.last_card_count = current_card_count;

    // Use stored geometry, but adjust height if cards were added/removed
    let initial_geometry = if card_count_changed {
        // Calculate height adjustment based on card count change
        let current_geom = container_geometry.unwrap_or(CountdownCardGeometry {
            x: 100.0,
            y: 100.0,
            width: default_width.max(container_min_width),
            height: ideal_height.max(container_min_height),
        });
        // Resize to accommodate new card count while keeping position and width
        CountdownCardGeometry {
            x: current_geom.x,
            y: current_geom.y,
            width: current_geom.width,
            height: ideal_height.max(container_min_height).min(800.0),
        }
    } else {
        container_geometry.unwrap_or(CountdownCardGeometry {
            x: 100.0,
            y: 100.0,
            width: default_width.max(container_min_width),
            height: ideal_height.max(container_min_height).min(600.0),
        })
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
    
    let builder = egui::ViewportBuilder::default()
        .with_title("Countdown Cards")
        .with_resizable(true)
        .with_visible(true)  // Ensure window is visible
        .with_min_inner_size(egui::vec2(container_min_width, container_min_height))
        // Always set position and size from stored geometry (like individual cards do)
        .with_position(egui::pos2(initial_geometry.x, initial_geometry.y))
        .with_inner_size(egui::vec2(initial_geometry.width, initial_geometry.height));

    // Log on first render or when card count changes
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
                    // Check if position is way off from what we stored (indicating OS moved it)
                    // or if position is on a secondary monitor (x > 1920 or x < 0 typically)
                    let position_seems_stuck = if let Some(stored) = container_geometry {
                        // Window reports being at stored position but we can't see/interact with it
                        // This happens when the position is on a monitor that's no longer available
                        // or when egui/winit fails to properly position on secondary monitors
                        let on_secondary = stored.x > 1920.0 || stored.x < 0.0 || stored.y < 0.0;
                        let position_matches = (new_geom.x - stored.x).abs() < 50.0 
                            && (new_geom.y - stored.y).abs() < 50.0;
                        
                        // If on secondary monitor and focus request didn't work, assume stuck
                        on_secondary && position_matches && !has_focus
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
            
            // Save geometry for any reasonable position
            // We previously tried to restrict to "primary monitor" but monitor layouts vary
            // Just save if the values are finite and not extremely off-screen
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

/// Calculate the rect for the insertion indicator during drag-and-drop
fn calculate_insertion_indicator_rect(
    layout: &ContainerLayout,
    ordered_ids: &[CountdownCardId],
    insert_index: usize,
    available_rect: egui::Rect,
) -> Option<egui::Rect> {
    let indicator_thickness = 4.0;
    let indicator_margin = 2.0;

    match layout.orientation {
        LayoutOrientation::Vertical => {
            let y = if insert_index == 0 {
                // Insert at top
                available_rect.top() + layout.padding - indicator_margin
            } else if insert_index >= ordered_ids.len() {
                // Insert at bottom
                if let Some(last_id) = ordered_ids.last() {
                    if let Some(last_rect) = layout.card_rects.get(last_id) {
                        last_rect.bottom() + indicator_margin
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            } else {
                // Insert between cards
                if let Some(card_id) = ordered_ids.get(insert_index) {
                    if let Some(rect) = layout.card_rects.get(card_id) {
                        rect.top() - layout.padding / 2.0
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            };

            Some(egui::Rect::from_min_size(
                egui::pos2(available_rect.left() + layout.padding, y - indicator_thickness / 2.0),
                egui::vec2(available_rect.width() - layout.padding * 2.0, indicator_thickness),
            ))
        }
        LayoutOrientation::Horizontal => {
            let x = if insert_index == 0 {
                // Insert at left
                available_rect.left() + layout.padding - indicator_margin
            } else if insert_index >= ordered_ids.len() {
                // Insert at right
                if let Some(last_id) = ordered_ids.last() {
                    if let Some(last_rect) = layout.card_rects.get(last_id) {
                        last_rect.right() + indicator_margin
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            } else {
                // Insert between cards
                if let Some(card_id) = ordered_ids.get(insert_index) {
                    if let Some(rect) = layout.card_rects.get(card_id) {
                        rect.left() - layout.padding / 2.0
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            };

            Some(egui::Rect::from_min_size(
                egui::pos2(x - indicator_thickness / 2.0, available_rect.top() + layout.padding),
                egui::vec2(indicator_thickness, available_rect.height() - layout.padding * 2.0),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_orientation_default() {
        let orientation = LayoutOrientation::default();
        assert_eq!(orientation, LayoutOrientation::Vertical);
    }

    #[test]
    fn test_container_layout_default() {
        let layout = ContainerLayout::default();
        assert_eq!(layout.orientation, LayoutOrientation::Vertical);
        assert!(layout.card_rects.is_empty());
        assert_eq!(layout.min_card_width, MIN_CARD_WIDTH);
        assert_eq!(layout.min_card_height, MIN_CARD_HEIGHT);
        assert_eq!(layout.padding, CARD_PADDING);
        assert!(!layout.initialized);
    }

    #[test]
    fn test_drag_state_default() {
        let state = DragState::default();
        assert!(!state.is_dragging());
        assert!(state.dragging_card.is_none());
    }

    #[test]
    fn test_drag_state_operations() {
        let mut state = DragState::default();
        let card_id = CountdownCardId(42);
        let start_pos = egui::pos2(100.0, 100.0);

        // Start drag
        state.start_drag(card_id, start_pos);
        assert!(state.is_dragging());
        assert!(state.is_dragging_card(card_id));
        assert!(!state.is_dragging_card(CountdownCardId(99)));
        assert_eq!(state.drag_start_pos, Some(start_pos));

        // Update drag
        let new_pos = egui::pos2(150.0, 150.0);
        state.update_drag(new_pos);
        assert_eq!(state.current_drag_pos, Some(new_pos));

        // End drag
        let ended_card = state.end_drag();
        assert_eq!(ended_card, Some(card_id));
        assert!(!state.is_dragging());
        assert!(state.drag_start_pos.is_none());
        assert!(state.current_drag_pos.is_none());
    }

    #[test]
    fn test_container_action_default() {
        let action = ContainerAction::default();
        assert!(matches!(action, ContainerAction::None));
    }

    #[test]
    fn test_calculate_layout_empty() {
        let mut layout = ContainerLayout::default();
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 600.0));
        
        layout.calculate_layout(rect, &[]);
        
        assert!(layout.card_rects.is_empty());
    }

    #[test]
    fn test_calculate_layout_vertical_orientation() {
        let mut layout = ContainerLayout::default();
        // Tall container should use vertical layout
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, 600.0));
        let cards = vec![CountdownCardId(1), CountdownCardId(2)];
        
        layout.calculate_layout(rect, &cards);
        
        assert_eq!(layout.orientation, LayoutOrientation::Vertical);
        assert_eq!(layout.card_rects.len(), 2);
        
        // Cards should be stacked vertically
        let rect1 = layout.get_card_rect(CountdownCardId(1)).unwrap();
        let rect2 = layout.get_card_rect(CountdownCardId(2)).unwrap();
        
        assert!(rect1.top() < rect2.top(), "First card should be above second");
        assert!((rect1.width() - rect2.width()).abs() < 0.01, "Cards should have same width");
    }

    #[test]
    fn test_calculate_layout_horizontal_orientation() {
        let mut layout = ContainerLayout::default();
        // Wide container should use horizontal layout
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 300.0));
        let cards = vec![CountdownCardId(1), CountdownCardId(2)];
        
        layout.calculate_layout(rect, &cards);
        
        assert_eq!(layout.orientation, LayoutOrientation::Horizontal);
        assert_eq!(layout.card_rects.len(), 2);
        
        // Cards should be side by side
        let rect1 = layout.get_card_rect(CountdownCardId(1)).unwrap();
        let rect2 = layout.get_card_rect(CountdownCardId(2)).unwrap();
        
        assert!(rect1.left() < rect2.left(), "First card should be left of second");
        assert!((rect1.height() - rect2.height()).abs() < 0.01, "Cards should have same height");
    }

    #[test]
    fn test_calculate_insert_index_vertical() {
        let mut layout = ContainerLayout::default();
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, 600.0));
        let cards = vec![CountdownCardId(1), CountdownCardId(2), CountdownCardId(3)];
        
        layout.calculate_layout(rect, &cards);
        
        // Test insertion at various Y positions
        let top_insert = layout.calculate_insert_index(egui::pos2(150.0, 10.0), &cards);
        assert_eq!(top_insert, 0, "Should insert at beginning for top position");
        
        let bottom_insert = layout.calculate_insert_index(egui::pos2(150.0, 590.0), &cards);
        assert_eq!(bottom_insert, 3, "Should insert at end for bottom position");
    }

    #[test]
    fn test_calculate_insert_index_horizontal() {
        let mut layout = ContainerLayout::default();
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(900.0, 300.0));
        let cards = vec![CountdownCardId(1), CountdownCardId(2), CountdownCardId(3)];
        
        layout.calculate_layout(rect, &cards);
        
        // Test insertion at various X positions
        let left_insert = layout.calculate_insert_index(egui::pos2(10.0, 150.0), &cards);
        assert_eq!(left_insert, 0, "Should insert at beginning for left position");
        
        let right_insert = layout.calculate_insert_index(egui::pos2(890.0, 150.0), &cards);
        assert_eq!(right_insert, 3, "Should insert at end for right position");
    }

    #[test]
    fn test_single_card_layout() {
        let mut layout = ContainerLayout::default();
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 300.0));
        let cards = vec![CountdownCardId(1)];
        
        layout.calculate_layout(rect, &cards);
        
        assert_eq!(layout.card_rects.len(), 1);
        let card_rect = layout.get_card_rect(CountdownCardId(1)).unwrap();
        
        // Single card should fill most of the container
        assert!(card_rect.width() > 300.0);
        assert!(card_rect.height() > 200.0);
    }
}
