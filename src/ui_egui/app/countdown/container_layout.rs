//! Layout computation and drag-and-drop state for the countdown container.

use crate::services::countdown::CountdownCardId;
use std::collections::HashMap;

/// Layout orientation for cards within the container
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutOrientation {
    /// Cards stacked vertically (tall/narrow container)
    #[default]
    Vertical,
    /// Cards arranged horizontally (wide container)
    Horizontal,
}

/// Minimum dimensions for container (absolute minimums for usability)
pub const CONTAINER_MIN_WIDTH: f32 = 80.0;
pub const CONTAINER_MIN_HEIGHT: f32 = 60.0;
/// Minimum card dimensions within container
pub const MIN_CARD_WIDTH: f32 = 60.0;
pub const MIN_CARD_HEIGHT: f32 = 50.0;
pub const CARD_PADDING: f32 = 8.0;

/// Number of frames to wait before checking if window position is valid
pub const VISIBILITY_CHECK_FRAMES: u32 = 15;

/// Layout calculator for arranging cards within the container
#[derive(Debug, Clone)]
pub struct ContainerLayout {
    /// Current layout orientation (always recalculated from aspect ratio)
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
    /// Whether the window has ever gained focus this session
    pub has_ever_had_focus: bool,
    /// Number of frames to skip geometry change detection (used after reset)
    pub skip_geometry_frames: u32,
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
            has_ever_had_focus: false,
            skip_geometry_frames: 0,
        }
    }
}

impl ContainerLayout {
    /// Calculate the layout for cards within the container.
    ///
    /// Orientation is determined by the container's aspect ratio:
    /// - Wide containers (aspect ratio > 1.5) use horizontal layout (landscape)
    /// - Tall/square containers use vertical layout (portrait)
    ///
    /// Cards are evenly distributed within the available space, respecting minimum sizes.
    /// When the user resizes the container, orientation updates to match the new aspect ratio.
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

        // Always determine orientation from current aspect ratio
        // This allows the container to switch between portrait/landscape when resized
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

/// Calculate the rect for the insertion indicator during drag-and-drop
pub fn calculate_insertion_indicator_rect(
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
        use super::super::container::ContainerAction;
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
