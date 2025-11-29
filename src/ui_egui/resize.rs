// Event Resize System
//
// Enables resizing events by dragging handles on event borders.
// - Top/Bottom handles: Adjust start/end time (Day/Week views)
// - Left/Right handles: Adjust start/end date (multi-day events)

use chrono::{DateTime, Local, NaiveDate, NaiveTime};
use egui::{Context, Id, Pos2, Rect, Vec2};

use crate::models::event::Event;

/// Which edge of the event is being resized
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeHandle {
    /// Top edge - adjusts start time
    Top,
    /// Bottom edge - adjusts end time
    Bottom,
    /// Left edge - adjusts start date (multi-day events)
    Left,
    /// Right edge - adjusts end date (multi-day events)
    Right,
}

impl ResizeHandle {
    /// Returns true if this handle adjusts time (vertical drag)
    pub fn is_vertical(&self) -> bool {
        matches!(self, ResizeHandle::Top | ResizeHandle::Bottom)
    }

    /// Returns true if this handle adjusts date (horizontal drag)
    pub fn is_horizontal(&self) -> bool {
        matches!(self, ResizeHandle::Left | ResizeHandle::Right)
    }

    /// Returns the cursor icon for this handle
    pub fn cursor_icon(&self) -> egui::CursorIcon {
        match self {
            ResizeHandle::Top | ResizeHandle::Bottom => egui::CursorIcon::ResizeVertical,
            ResizeHandle::Left | ResizeHandle::Right => egui::CursorIcon::ResizeHorizontal,
        }
    }
}

/// Which view the resize is happening in
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeView {
    Day,
    Week,
    WorkWeek,
    Ribbon,
}

/// Size of the resize handle hit area
pub const HANDLE_SIZE: f32 = 10.0;
/// Visual size of the handle circle
pub const HANDLE_VISUAL_SIZE: f32 = 8.0;
/// Width of the handle hit zone (narrower than event to avoid confusion with drag)
pub const HANDLE_HIT_WIDTH: f32 = 60.0;

/// Context for an active resize operation
#[derive(Clone, Debug)]
pub struct ResizeContext {
    /// The event being resized
    pub event_id: i64,
    /// Which handle is being dragged
    pub handle: ResizeHandle,
    /// Original event start time
    pub original_start: DateTime<Local>,
    /// Original event end time
    pub original_end: DateTime<Local>,
    /// Current pointer position
    pub pointer_pos: Option<Pos2>,
    /// Current hovered date (for horizontal resize)
    pub hovered_date: Option<NaiveDate>,
    /// Current hovered time (for vertical resize)
    pub hovered_time: Option<NaiveTime>,
    /// The view where resize is happening
    pub view: ResizeView,
    /// The event rect at drag start
    pub original_rect: Rect,
}

impl ResizeContext {
    /// Create a new resize context from an event
    pub fn new(
        event: &Event,
        handle: ResizeHandle,
        view: ResizeView,
        rect: Rect,
    ) -> Option<Self> {
        let event_id = event.id?;
        Some(Self {
            event_id,
            handle,
            original_start: event.start,
            original_end: event.end,
            pointer_pos: None,
            hovered_date: None,
            hovered_time: None,
            view,
            original_rect: rect,
        })
    }

    /// Create a resize context from an event (without requiring rect upfront)
    /// Used when we know we're on a resize handle but don't need the rect
    pub fn from_event(
        event: &Event,
        handle: ResizeHandle,
        view: ResizeView,
    ) -> Option<Self> {
        Self::new(event, handle, view, Rect::NOTHING)
    }

    /// Get the new start and end times based on hover state
    pub fn hovered_times(&self) -> Option<(DateTime<Local>, DateTime<Local>)> {
        let new_start = self.calculate_new_start()?;
        let new_end = self.calculate_new_end()?;
        
        // Validate: start must be before end, with minimum 15 min duration
        let min_duration = chrono::Duration::minutes(15);
        if new_end - new_start >= min_duration {
            Some((new_start, new_end))
        } else {
            None
        }
    }

    /// Calculate the new start time based on drag position
    pub fn calculate_new_start(&self) -> Option<DateTime<Local>> {
        match self.handle {
            ResizeHandle::Top => {
                // Vertical resize - use hovered time with original date
                self.hovered_time.and_then(|time| {
                    self.original_start
                        .date_naive()
                        .and_time(time)
                        .and_local_timezone(Local)
                        .single()
                })
            }
            ResizeHandle::Left => {
                // Horizontal resize - use hovered date with original time
                self.hovered_date.and_then(|date| {
                    date.and_time(self.original_start.time())
                        .and_local_timezone(Local)
                        .single()
                })
            }
            _ => Some(self.original_start),
        }
    }

    /// Calculate the new end time based on drag position
    pub fn calculate_new_end(&self) -> Option<DateTime<Local>> {
        match self.handle {
            ResizeHandle::Bottom => {
                // Vertical resize - use hovered time with original date
                self.hovered_time.and_then(|time| {
                    self.original_end
                        .date_naive()
                        .and_time(time)
                        .and_local_timezone(Local)
                        .single()
                })
            }
            ResizeHandle::Right => {
                // Horizontal resize - use hovered date with original time
                self.hovered_date.and_then(|date| {
                    date.and_time(self.original_end.time())
                        .and_local_timezone(Local)
                        .single()
                })
            }
            _ => Some(self.original_end),
        }
    }
}

/// Manager for resize operations (similar to DragManager)
pub struct ResizeManager;

impl ResizeManager {
    fn storage_id() -> Id {
        Id::new("calendar_event_resize_state")
    }

    /// Begin a resize operation
    pub fn begin(ctx: &Context, context: ResizeContext) {
        ctx.memory_mut(|mem| {
            mem.data.insert_persisted(Self::storage_id(), context);
        });
    }

    /// Get the active resize context, if any
    pub fn active(ctx: &Context) -> Option<ResizeContext> {
        ctx.memory_mut(|mem| mem.data.get_persisted::<ResizeContext>(Self::storage_id()))
    }

    /// Get the active resize context if it matches the given view
    pub fn active_for_view(ctx: &Context, view: ResizeView) -> Option<ResizeContext> {
        Self::active(ctx).filter(|ctx_data| ctx_data.view == view)
    }

    /// Check if a resize is active for the given view
    pub fn is_active_for_view(ctx: &Context, view: ResizeView) -> bool {
        Self::active_for_view(ctx, view).is_some()
    }

    /// Check if resizing a specific event
    pub fn is_resizing_event(ctx: &Context, event_id: i64) -> bool {
        Self::active(ctx).map_or(false, |c| c.event_id == event_id)
    }

    /// Update hover position during resize
    /// For bottom handle: use slot_end as the target time
    /// For top handle: use slot_start as the target time
    pub fn update_hover(
        ctx: &Context,
        date: NaiveDate,
        slot_start: NaiveTime,
        slot_end: NaiveTime,
        pointer_pos: Pos2,
    ) {
        let id = Self::storage_id();
        ctx.memory_mut(|mem| {
            if let Some(mut state) = mem.data.get_persisted::<ResizeContext>(id) {
                state.hovered_date = Some(date);
                // Use appropriate time based on which handle is being dragged
                state.hovered_time = Some(match state.handle {
                    ResizeHandle::Bottom | ResizeHandle::Right => slot_end,
                    ResizeHandle::Top | ResizeHandle::Left => slot_start,
                });
                state.pointer_pos = Some(pointer_pos);
                mem.data.insert_persisted(id, state);
            }
        });
    }

    /// Finish the resize operation and return the context
    pub fn finish(ctx: &Context) -> Option<ResizeContext> {
        let id = Self::storage_id();
        let mut result = None;
        ctx.memory_mut(|mem| {
            if let Some(current) = mem.data.get_persisted::<ResizeContext>(id) {
                result = Some(current);
                mem.data.remove::<ResizeContext>(id);
            }
        });
        result
    }

    /// Finish resize only if it matches the given view
    pub fn finish_for_view(ctx: &Context, view: ResizeView) -> Option<ResizeContext> {
        let id = Self::storage_id();
        let mut result = None;
        ctx.memory_mut(|mem| {
            if let Some(current) = mem.data.get_persisted::<ResizeContext>(id) {
                if current.view == view {
                    result = Some(current);
                    mem.data.remove::<ResizeContext>(id);
                }
            }
        });
        result
    }

    /// Cancel the resize operation
    pub fn cancel(ctx: &Context) {
        ctx.memory_mut(|mem| {
            mem.data.remove::<ResizeContext>(Self::storage_id());
        });
    }
}

/// Calculate handle rects for an event
pub struct HandleRects {
    pub top: Option<Rect>,
    pub bottom: Option<Rect>,
    pub left: Option<Rect>,
    pub right: Option<Rect>,
}

impl HandleRects {
    /// Create handle rects for a timed event (top/bottom only)
    /// - show_top: true if this is the starting slot for the event
    /// - show_bottom: true if this is the ending slot for the event
    pub fn for_timed_event_in_slot(event_rect: Rect, show_top: bool, show_bottom: bool) -> Self {
        // Height of the hit zone at each edge
        let zone_height = HANDLE_SIZE + 4.0; // Handle size plus small padding
        
        // Add extra padding to catch drag starts that slip just outside the event
        const EDGE_PADDING: f32 = 6.0;
        
        // Hit zone is centered on the event, narrower than full width to avoid
        // confusion with event drag operations. Use min of event width and constant.
        let hit_width = event_rect.width().min(HANDLE_HIT_WIDTH);
        let center_x = event_rect.center().x;
        
        Self {
            top: if show_top {
                // Centered hit zone at top edge, extending slightly above
                Some(Rect::from_min_size(
                    Pos2::new(center_x - hit_width / 2.0, event_rect.top() - EDGE_PADDING),
                    Vec2::new(hit_width, zone_height + EDGE_PADDING),
                ))
            } else {
                None
            },
            bottom: if show_bottom {
                // Centered hit zone at bottom edge, extending slightly below
                Some(Rect::from_min_size(
                    Pos2::new(center_x - hit_width / 2.0, event_rect.bottom() - zone_height),
                    Vec2::new(hit_width, zone_height + EDGE_PADDING),
                ))
            } else {
                None
            },
            left: None,
            right: None,
        }
    }

    /// Create handle rects for a timed event (top/bottom only) - shows both handles
    pub fn for_timed_event(event_rect: Rect) -> Self {
        Self::for_timed_event_in_slot(event_rect, true, true)
    }

    /// Create handle rects for a multi-day event (all four handles)
    pub fn for_multiday_event(event_rect: Rect) -> Self {
        let handle_height = event_rect.height().min(20.0);
        let handle_width = event_rect.width().min(30.0);
        
        Self {
            top: Some(Rect::from_center_size(
                Pos2::new(event_rect.center().x, event_rect.top()),
                Vec2::new(handle_width, HANDLE_SIZE),
            )),
            bottom: Some(Rect::from_center_size(
                Pos2::new(event_rect.center().x, event_rect.bottom()),
                Vec2::new(handle_width, HANDLE_SIZE),
            )),
            left: Some(Rect::from_center_size(
                Pos2::new(event_rect.left(), event_rect.center().y),
                Vec2::new(HANDLE_SIZE, handle_height),
            )),
            right: Some(Rect::from_center_size(
                Pos2::new(event_rect.right(), event_rect.center().y),
                Vec2::new(HANDLE_SIZE, handle_height),
            )),
        }
    }

    /// Create handle rects for ribbon events (left/right only)
    pub fn for_ribbon_event(event_rect: Rect) -> Self {
        let handle_height = event_rect.height().min(20.0);
        
        Self {
            top: None,
            bottom: None,
            left: Some(Rect::from_center_size(
                Pos2::new(event_rect.left(), event_rect.center().y),
                Vec2::new(HANDLE_SIZE, handle_height),
            )),
            right: Some(Rect::from_center_size(
                Pos2::new(event_rect.right(), event_rect.center().y),
                Vec2::new(HANDLE_SIZE, handle_height),
            )),
        }
    }

    /// Check if a point hits any handle and return which one
    pub fn hit_test(&self, pos: Pos2) -> Option<ResizeHandle> {
        if self.top.map_or(false, |r| r.contains(pos)) {
            Some(ResizeHandle::Top)
        } else if self.bottom.map_or(false, |r| r.contains(pos)) {
            Some(ResizeHandle::Bottom)
        } else if self.left.map_or(false, |r| r.contains(pos)) {
            Some(ResizeHandle::Left)
        } else if self.right.map_or(false, |r| r.contains(pos)) {
            Some(ResizeHandle::Right)
        } else {
            None
        }
    }

    /// Get the rect for a specific handle
    pub fn get(&self, handle: ResizeHandle) -> Option<Rect> {
        match handle {
            ResizeHandle::Top => self.top,
            ResizeHandle::Bottom => self.bottom,
            ResizeHandle::Left => self.left,
            ResizeHandle::Right => self.right,
        }
    }
}

/// Draw resize handles on an event
pub fn draw_handles(
    ui: &mut egui::Ui,
    handles: &HandleRects,
    hovered_handle: Option<ResizeHandle>,
    color: egui::Color32,
) {
    let draw_handle = |rect: Rect, handle_type: ResizeHandle, is_hovered: bool| {
        // Position the visual elements at the edge
        let center_x = rect.center().x;
        let (center_y, bar_y) = match handle_type {
            ResizeHandle::Top => (
                rect.top() + HANDLE_VISUAL_SIZE / 2.0 + 4.0,
                rect.top() + 4.0,
            ),
            ResizeHandle::Bottom => (
                rect.bottom() - HANDLE_VISUAL_SIZE / 2.0 - 4.0,
                rect.bottom() - 4.0,
            ),
            ResizeHandle::Left | ResizeHandle::Right => (
                rect.center().y,
                rect.center().y,
            ),
        };
        let center = Pos2::new(center_x, center_y);
        
        let radius = if is_hovered {
            HANDLE_VISUAL_SIZE / 2.0 + 2.0
        } else {
            HANDLE_VISUAL_SIZE / 2.0
        };
        
        // Draw a subtle bar indicator across the hit zone
        let bar_width = rect.width().min(40.0);
        if handle_type == ResizeHandle::Top || handle_type == ResizeHandle::Bottom {
            ui.painter().line_segment(
                [
                    Pos2::new(center_x - bar_width / 2.0, bar_y),
                    Pos2::new(center_x + bar_width / 2.0, bar_y),
                ],
                egui::Stroke::new(
                    if is_hovered { 3.0 } else { 2.0 },
                    if is_hovered {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180)
                    },
                ),
            );
        }
        
        // Draw circle handle (more prominent)
        ui.painter().circle_filled(
            center,
            radius,
            if is_hovered {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 220)
            },
        );
        ui.painter().circle_stroke(
            center,
            radius,
            egui::Stroke::new(
                if is_hovered { 2.0 } else { 1.5 },
                color.linear_multiply(0.8),
            ),
        );
    };

    if let Some(rect) = handles.top {
        draw_handle(rect, ResizeHandle::Top, hovered_handle == Some(ResizeHandle::Top));
    }
    if let Some(rect) = handles.bottom {
        draw_handle(rect, ResizeHandle::Bottom, hovered_handle == Some(ResizeHandle::Bottom));
    }
    if let Some(rect) = handles.left {
        draw_handle(rect, ResizeHandle::Left, hovered_handle == Some(ResizeHandle::Left));
    }
    if let Some(rect) = handles.right {
        draw_handle(rect, ResizeHandle::Right, hovered_handle == Some(ResizeHandle::Right));
    }
}

/// Draw a resize preview silhouette showing where the event will end up
/// 
/// Parameters:
/// - `ui`: The egui Ui context
/// - `resize_ctx`: The active resize context with drag state
/// - `slot_rect`: The rect of the current time slot being rendered
/// - `slot_date`: The date of the current slot (to check if preview should appear in this column)
/// - `slot_time`: The start time of the current slot
/// - `slot_end_time`: The end time of the current slot  
/// - `event_color`: The color of the event (will be made translucent)
/// - `left_margin`: Left margin for the event rect within the slot
pub fn draw_resize_preview(
    ui: &mut egui::Ui,
    resize_ctx: &ResizeContext,
    slot_rect: Rect,
    slot_date: NaiveDate,
    slot_time: NaiveTime,
    slot_end_time: NaiveTime,
    event_color: egui::Color32,
    left_margin: f32,
) {
    // Get the new start/end times based on current drag position
    let (preview_start, preview_end) = match resize_ctx.hovered_times() {
        Some(times) => times,
        None => return, // No valid preview yet
    };
    
    // Only draw preview in the correct column (same date as the event)
    // For vertical resize, the date doesn't change
    let event_date = resize_ctx.original_start.date_naive();
    if slot_date != event_date {
        return;
    }
    
    let preview_start_time = preview_start.time();
    let preview_end_time = preview_end.time();
    
    // Check if this slot overlaps with the preview time range
    let slot_overlaps = preview_start_time < slot_end_time && preview_end_time > slot_time;
    
    if !slot_overlaps {
        return;
    }
    
    // Calculate the visible portion of the preview in this slot
    let visible_start = preview_start_time.max(slot_time);
    let visible_end = preview_end_time.min(slot_end_time);
    
    // Calculate Y positions within the slot
    let slot_duration = (slot_end_time - slot_time).num_minutes() as f32;
    let start_offset = (visible_start - slot_time).num_minutes() as f32 / slot_duration;
    let end_offset = (visible_end - slot_time).num_minutes() as f32 / slot_duration;
    
    let top_y = slot_rect.top() + slot_rect.height() * start_offset;
    let bottom_y = slot_rect.top() + slot_rect.height() * end_offset;
    
    // Create the preview rect (same layout as events)
    let preview_rect = Rect::from_min_max(
        Pos2::new(slot_rect.left() + left_margin, top_y + 2.0),
        Pos2::new(slot_rect.right() - 5.0, bottom_y - 2.0),
    );
    
    // Draw with a pale, translucent version of the event color
    let preview_color = egui::Color32::from_rgba_unmultiplied(
        event_color.r(),
        event_color.g(),
        event_color.b(),
        60, // Very translucent
    );
    
    // Fill
    ui.painter().rect_filled(preview_rect, 3.0, preview_color);
    
    // Dashed border effect - draw as a stroke with the event color
    let border_color = egui::Color32::from_rgba_unmultiplied(
        event_color.r(),
        event_color.g(),
        event_color.b(),
        140,
    );
    ui.painter().rect_stroke(
        preview_rect,
        3.0,
        egui::Stroke::new(2.0, border_color),
    );
    
    // Draw accent bar on left side
    let bar_rect = Rect::from_min_size(
        Pos2::new(preview_rect.left(), preview_rect.top()),
        Vec2::new(4.0, preview_rect.height()),
    );
    ui.painter().rect_filled(
        bar_rect,
        2.0,
        egui::Color32::from_rgba_unmultiplied(
            event_color.r(),
            event_color.g(),
            event_color.b(),
            100,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_handle_is_vertical() {
        assert!(ResizeHandle::Top.is_vertical());
        assert!(ResizeHandle::Bottom.is_vertical());
        assert!(!ResizeHandle::Left.is_vertical());
        assert!(!ResizeHandle::Right.is_vertical());
    }

    #[test]
    fn test_resize_handle_is_horizontal() {
        assert!(!ResizeHandle::Top.is_horizontal());
        assert!(!ResizeHandle::Bottom.is_horizontal());
        assert!(ResizeHandle::Left.is_horizontal());
        assert!(ResizeHandle::Right.is_horizontal());
    }

    #[test]
    fn test_handle_rects_for_timed_event() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(200.0, 50.0));
        let handles = HandleRects::for_timed_event(rect);
        
        assert!(handles.top.is_some());
        assert!(handles.bottom.is_some());
        assert!(handles.left.is_none());
        assert!(handles.right.is_none());
    }

    #[test]
    fn test_handle_rects_for_ribbon_event() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(200.0, 20.0));
        let handles = HandleRects::for_ribbon_event(rect);
        
        assert!(handles.top.is_none());
        assert!(handles.bottom.is_none());
        assert!(handles.left.is_some());
        assert!(handles.right.is_some());
    }

    #[test]
    fn test_handle_hit_test() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(200.0, 50.0));
        let handles = HandleRects::for_timed_event(rect);
        
        // Test top handle hit
        let top_center = Pos2::new(200.0, 100.0); // center x, top y
        assert_eq!(handles.hit_test(top_center), Some(ResizeHandle::Top));
        
        // Test bottom handle hit
        let bottom_center = Pos2::new(200.0, 150.0); // center x, bottom y
        assert_eq!(handles.hit_test(bottom_center), Some(ResizeHandle::Bottom));
        
        // Test miss
        let miss = Pos2::new(200.0, 125.0); // center, middle
        assert_eq!(handles.hit_test(miss), None);
    }
}
