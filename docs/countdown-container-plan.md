# Enhanced Countdown Container Implementation Plan

> **Status: Implemented** — The countdown container has been built and shipped. This document is the original implementation plan, retained for reference. Actual implementation may differ in details.

## Overview

Implement a container mode for countdown cards that allows users to display all countdown cards in a single resizable window, while preserving the existing individual window mode.

## Background

Currently, each countdown card is rendered as a separate egui viewport/window. Users want the option to combine all cards into a single container where:

- The container is a single resizable window
- Cards auto-arrange based on container dimensions (vertical stack when tall/narrow, horizontal row when wide)
- Cards can be manually dragged to reorder within the container
- Individual window mode remains available

## Design Decisions

> [!NOTE]
> **Display Mode**: Global setting - all cards are either in Individual Windows mode OR Container mode (not mixed).

> [!NOTE]
> **Card Sizing**: Cards intelligently auto-resize within the container:
>
> - **Vertical layout** (tall/narrow container): Cards take full width, auto-size height proportionally
> - **Horizontal layout** (wide container): Cards take full height, auto-size width proportionally
> - Minimum size constraints apply to prevent cards from becoming too small

## Proposed Changes

### Core Models

#### [MODIFY] [models.rs](file:///c:/rust-calendar/src/services/countdown/models.rs)

Add display mode configuration:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CountdownDisplayMode {
    IndividualWindows,
    Container,
}

impl Default for CountdownDisplayMode {
    fn default() -> Self {
        Self::IndividualWindows
    }
}

// Add to CountdownPersistedState
pub display_mode: CountdownDisplayMode,
pub container_geometry: Option<CountdownCardGeometry>,
pub card_order: Vec<CountdownCardId>,  // Manual ordering for drag-to-reorder
```

---

### Service Layer

#### [MODIFY] [service.rs](file:///c:/rust-calendar/src/services/countdown/service.rs)

Add methods for display mode management:

```rust
pub fn set_display_mode(&mut self, mode: CountdownDisplayMode)
pub fn display_mode(&self) -> CountdownDisplayMode
pub fn reorder_cards(&mut self, ordered_ids: Vec<CountdownCardId>)
pub fn card_order(&self) -> &[CountdownCardId]
```

---

### UI Layer

#### [NEW] [container.rs](file:///c:/rust-calendar/src/ui_egui/app/countdown/container.rs)

Create new module for container rendering with layout logic:

```rust
pub struct ContainerLayout {
    orientation: LayoutOrientation,
    card_rects: HashMap<CountdownCardId, egui::Rect>,
    min_card_width: f32,
    min_card_height: f32,
    padding: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutOrientation {
    Vertical,    // Tall/narrow container
    Horizontal,  // Wide container
}

impl ContainerLayout {
    pub fn new() -> Self { /* ... */ }
    
    // Calculate layout based on container size and card count
    pub fn calculate_layout(
        &mut self,
        available_rect: egui::Rect,
        cards: &[CountdownCardState],
        card_order: &[CountdownCardId],
    ) {
        // Determine orientation based on aspect ratio
        let aspect_ratio = available_rect.width() / available_rect.height();
        self.orientation = if aspect_ratio > 1.2 {
            LayoutOrientation::Horizontal
        } else {
            LayoutOrientation::Vertical
        };
        
        // Calculate card positions
        self.layout_cards(available_rect, cards, card_order);
    }
    
    fn layout_cards(
        &mut self,
        available: egui::Rect,
        cards: &[CountdownCardState],
        order: &[CountdownCardId],
    ) {
        // Implementation for auto-positioning cards
    }
}

pub struct DragState {
    dragging_card: Option<CountdownCardId>,
    drag_start_pos: egui::Pos2,
    insert_index: usize,
}

pub fn render_container_window(
    ctx: &Context,
    viewport_id: ViewportId,
    cards: &[CountdownCardState],
    card_order: &[CountdownCardId],
    layout: &mut ContainerLayout,
    drag_state: &mut DragState,
    now: DateTime<Local>,
    notification_config: &CountdownNotificationConfig,
) -> ContainerAction {
    // Render all cards in container with drag-drop support
}

pub enum ContainerAction {
    None,
    ReorderCards(Vec<CountdownCardId>),
    DeleteCard(CountdownCardId),
    OpenSettings(CountdownCardId),
    OpenEventDialog(CountdownCardId),
    GoToDate(chrono::NaiveDate),
}
```

---

#### [MODIFY] [state.rs](file:///c:/rust-calendar/src/ui_egui/app/countdown/state.rs)

Update `CountdownUiState` to support container mode:

```rust
pub(in super::super) struct CountdownUiState {
    // ... existing fields ...
    
    // Container mode fields
    container_layout: ContainerLayout,
    container_drag_state: DragState,
    container_geometry: Option<CountdownCardGeometry>,
}

impl CountdownUiState {
    pub(in super::super) fn render_cards(
        &mut self,
        ctx: &Context,
        service: &mut CountdownService,
    ) -> CountdownRenderResult {
        match service.display_mode() {
            CountdownDisplayMode::IndividualWindows => {
                self.render_individual_windows(ctx, service)
            }
            CountdownDisplayMode::Container => {
                self.render_container_mode(ctx, service)
            }
        }
    }
    
    fn render_individual_windows(/* existing render_cards logic */) { }
    
    fn render_container_mode(
        &mut self,
        ctx: &Context,
        service: &mut CountdownService,
    ) -> CountdownRenderResult {
        let viewport_id = ViewportId::from_hash_of("countdown_container");
        let cards = service.cards().to_vec();
        let card_order = service.card_order().to_vec();
        
        // Create viewport for container
        let builder = /* viewport builder */;
        
        let action = ctx.show_viewport_immediate(viewport_id, builder, |child_ctx, class| {
            render_container_window(
                child_ctx,
                viewport_id,
                &cards,
                &card_order,
                &mut self.container_layout,
                &mut self.container_drag_state,
                Local::now(),
                service.notification_config(),
            )
        });
        
        // Handle container actions
        match action {
            ContainerAction::ReorderCards(new_order) => {
                service.reorder_cards(new_order);
            }
            // ... handle other actions
        }
        
        CountdownRenderResult::default()
    }
}
```

---

#### [MODIFY] [settings.rs](file:///c:/rust-calendar/src/ui_egui/app/countdown/settings.rs)

Add display mode toggle to countdown settings:

```rust
pub enum CountdownSettingsCommand {
    // ... existing variants ...
    SetDisplayMode(CountdownDisplayMode),
}

// In render_countdown_settings_ui, add:
ui.heading("Display Mode");
ui.radio_value(&mut display_mode, CountdownDisplayMode::IndividualWindows, "Individual Windows");
ui.radio_value(&mut display_mode, CountdownDisplayMode::Container, "Container (All cards in one window)");
```

---

### Drag-and-Drop Implementation

The drag implementation will use egui's built-in drag-and-drop:

```rust
// In container.rs rendering loop
for (index, card_id) in card_order.iter().enumerate() {
    let card_rect = layout.card_rects.get(card_id).unwrap();
    
    let response = ui.allocate_rect(*card_rect, egui::Sense::click_and_drag());
    
    // Render card content
    render_card_content(ui, card, now, notification_config);
    
    // Handle drag
    if response.drag_started() {
        drag_state.dragging_card = Some(*card_id);
        drag_state.drag_start_pos = response.interact_pointer_pos().unwrap();
    }
    
    if response.dragged() && drag_state.dragging_card == Some(*card_id) {
        // Calculate insert index based on drag position
        let drag_pos = response.interact_pointer_pos().unwrap();
        drag_state.insert_index = calculate_insert_index(
            drag_pos,
            card_order,
            &layout.card_rects,
        );
        
        // Visual feedback: draw insertion line/indicator
    }
    
    if response.drag_stopped() {
        if let Some(dragged) = drag_state.dragging_card {
            // Reorder the cards
            let new_order = reorder_vec(card_order, dragged, drag_state.insert_index);
            return ContainerAction::ReorderCards(new_order);
        }
        drag_state.dragging_card = None;
    }
}
```

---

### Layout Algorithm

```rust
fn layout_cards(
    &mut self,
    available: egui::Rect,
    cards: &[CountdownCardState],
    order: &[CountdownCardId],
) {
    self.card_rects.clear();
    
    let count = cards.len() as f32;
    if count == 0.0 { return; }
    
    match self.orientation {
        LayoutOrientation::Vertical => {
            let card_height = (available.height() - self.padding * (count + 1.0)) / count;
            let card_height = card_height.max(self.min_card_height);
            let card_width = available.width() - self.padding * 2.0;
            
            for (i, card_id) in order.iter().enumerate() {
                let y = available.top() + self.padding + i as f32 * (card_height + self.padding);
                let x = available.left() + self.padding;
                
                self.card_rects.insert(*card_id, egui::Rect::from_min_size(
                    egui::pos2(x, y),
                    egui::vec2(card_width, card_height),
                ));
            }
        }
        LayoutOrientation::Horizontal => {
            let card_width = (available.width() - self.padding * (count + 1.0)) / count;
            let card_width = card_width.max(self.min_card_width);
            let card_height = available.height() - self.padding * 2.0;
            
            for (i, card_id) in order.iter().enumerate() {
                let x = available.left() + self.padding + i as f32 * (card_width + self.padding);
                let y = available.top() + self.padding;
                
                self.card_rects.insert(*card_id, egui::Rect::from_min_size(
                    egui::pos2(x, y),
                    egui::vec2(card_width, card_height),
                ));
            }
        }
    }
}
```

---

## Verification Plan

### Automated Tests

- Unit tests for layout calculation with various container sizes
- Test card reordering logic
- Test orientation switching

### Manual Verification

1. **Mode Switching**: Toggle between individual and container modes, verify cards transition smoothly
2. **Responsive Layout**:
   - Start with wide container → verify horizontal layout
   - Resize to tall/narrow → verify switches to vertical layout
   - Resize back → verify returns to horizontal
3. **Drag Reordering**:
   - Drag card to new position in vertical stack
   - Drag card to new position in horizontal row
   - Verify order persists after app restart
4. **Card Operations in Container**:
   - Delete card from container → verify layout reflows
   - Add new card → verify appears in container
   - Open settings from container card → verify works
   - Click "Go to Date" → verify navigates calendar
5. **Geometry Persistence**:
   - Resize container window
   - Close and reopen app
   - Verify container opens at saved size/position

### Edge Cases

- Single card in container
- Many cards (10+) in container → verify scrolling if needed
- Very small container size → verify minimum sizes enforced
- Switching modes with cards open → verify cleanup
