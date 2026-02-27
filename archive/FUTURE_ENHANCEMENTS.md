# Future Enhancements

This document outlines planned enhancements for future development.

---

## 1. Dim Past All-Day Events

**Priority:** Low  
**Complexity:** Simple  
**Status:** Planned

### Current Behavior
- Past timed events are dimmed to 50% opacity in Day, Week, and Month views
- All-day events are NOT dimmed, even when they're in the past

### Desired Behavior
- All-day events that have ended should also be dimmed to 50% opacity
- Consistent visual treatment for all past events

### Implementation Notes

#### Files to Modify
- `src/ui_egui/views/week_shared.rs` - Ribbon/all-day event rendering
- `src/ui_egui/views/day_view.rs` - All-day event section
- `src/ui_egui/views/month_view.rs` - Already handles timed events, check all-day

#### Logic
```rust
// For all-day events, check if end date is before today
let today = Local::now().date_naive();
let is_past = event.end.date_naive() < today;

// For multi-day all-day events, only dim if the ENTIRE event has passed
// e.g., "Newcastle Trip" spanning Nov 24-27 should not be dimmed on Nov 25
```

#### Considerations
- Multi-day events: Only dim after the last day has passed
- Current day all-day events: Should NOT be dimmed
- Consistent with timed event dimming (50% opacity via `linear_multiply(0.5)`)

---

## 2. Countdown Card Container

**Priority:** Medium  
**Complexity:** Moderate to High  
**Status:** Planned

### Current Behavior
- Countdown cards are created individually
- Each card is independent with no grouping
- Cards are displayed in a flat list
- Position/order is implicit based on creation or date

### Desired Behavior
- Cards live inside a "Card Container" or "Card Deck"
- Cards are automatically arranged by date (default sort)
- Users can manually drag cards within the container to reorder
- All existing card functionality preserved (edit, delete, toggle countdown, etc.)
- Container can be minimized/expanded
- Optional: Multiple containers for different categories

### Data Model Changes

#### New Database Table: `countdown_containers`
```sql
CREATE TABLE IF NOT EXISTS countdown_containers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL DEFAULT 'My Countdowns',
    position INTEGER NOT NULL DEFAULT 0,  -- For ordering multiple containers
    is_collapsed INTEGER NOT NULL DEFAULT 0,
    sort_mode TEXT NOT NULL DEFAULT 'date',  -- 'date', 'manual', 'priority'
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

#### Modify `countdown_cards` Table
```sql
-- Add container reference and manual position
ALTER TABLE countdown_cards ADD COLUMN container_id INTEGER REFERENCES countdown_containers(id);
ALTER TABLE countdown_cards ADD COLUMN manual_position INTEGER;  -- For drag reordering
```

### UI Components

#### Container Component
```
┌─────────────────────────────────────────┐
│ 📦 My Countdowns                    ▼ ─ │  <- Header with collapse toggle
├─────────────────────────────────────────┤
│ ┌─────────────────────────────────────┐ │
│ │ 🎄 Christmas Cruise                 │ │  <- Draggable card
│ │    23 days remaining                │ │
│ └─────────────────────────────────────┘ │
│ ┌─────────────────────────────────────┐ │
│ │ 🎁 Birthday Party                   │ │
│ │    45 days remaining                │ │
│ └─────────────────────────────────────┘ │
│                                         │
│         + Add Card                      │  <- Quick add button
└─────────────────────────────────────────┘
```

#### Drag & Drop Implementation
- Use egui's drag-and-drop capabilities
- Show drop indicator line between cards
- Update `manual_position` on drop
- If sort_mode is 'date', warn user that manual reorder will switch to 'manual' mode

### Files to Create/Modify

#### New Files
- `src/ui_egui/app/countdown/container.rs` - Container UI component
- `src/services/countdown/container_service.rs` - Container CRUD operations

#### Modified Files
- `src/services/database/schema.rs` - Add new table, migrations
- `src/services/countdown/persistence.rs` - Update to include container_id
- `src/services/countdown/service.rs` - Container-aware card management
- `src/ui_egui/app/countdown/mod.rs` - Integrate container component
- `src/ui_egui/app/sidebar.rs` - Render containers instead of flat list

### Migration Strategy
1. Create `countdown_containers` table
2. Create default container "My Countdowns" with id=1
3. Set all existing cards' `container_id` to 1
4. Set `manual_position` based on current order

### API Design

```rust
pub struct CountdownContainer {
    pub id: Option<i64>,
    pub name: String,
    pub position: i32,
    pub is_collapsed: bool,
    pub sort_mode: SortMode,
    pub cards: Vec<CountdownCard>,
}

pub enum SortMode {
    Date,      // Sort by event start date
    Manual,    // User-defined order
    Priority,  // Future: by priority/importance
}

impl CountdownContainerService {
    pub fn list_all(&self) -> Result<Vec<CountdownContainer>>;
    pub fn create(&self, name: &str) -> Result<CountdownContainer>;
    pub fn update(&self, container: &CountdownContainer) -> Result<()>;
    pub fn delete(&self, id: i64) -> Result<()>;
    pub fn reorder_card(&self, card_id: i64, new_position: i32) -> Result<()>;
    pub fn move_card_to_container(&self, card_id: i64, container_id: i64) -> Result<()>;
}
```

### Future Extensions
- Multiple containers for categorization (Work, Personal, Travel, etc.)
- Container colors/themes
- Drag cards between containers
- Export/share containers
- Container templates

---

## 3. Countdown Card Tooltip - Show Date Range

**Priority:** Low  
**Complexity:** Simple  
**Status:** Planned

### Current Behavior
- Countdown card tooltips/flyovers show basic event info
- Date range of the event is not displayed

### Desired Behavior
- Tooltip should include the event's start and end date/time
- Format examples:
  - Single day timed: "Nov 27, 2025 • 2:00 PM - 4:00 PM"
  - Single day all-day: "Nov 27, 2025 • All Day"
  - Multi-day: "Nov 24 - Nov 27, 2025"
  - Multi-day with times: "Nov 24, 2:00 PM - Nov 27, 6:00 PM"

### Implementation Notes

#### Files to Modify
- `src/ui_egui/app/countdown/mod.rs` - Card rendering with tooltip

#### Display Logic
```rust
fn format_date_range(start: DateTime<Local>, end: DateTime<Local>, all_day: bool) -> String {
    let start_date = start.date_naive();
    let end_date = end.date_naive();
    
    if start_date == end_date {
        // Same day
        if all_day {
            format!("{} • All Day", start.format("%b %d, %Y"))
        } else {
            format!("{} • {} - {}", 
                start.format("%b %d, %Y"),
                start.format("%I:%M %p"),
                end.format("%I:%M %p"))
        }
    } else {
        // Multi-day
        if all_day {
            format!("{} - {}", 
                start.format("%b %d"),
                end.format("%b %d, %Y"))
        } else {
            format!("{}, {} - {}, {}",
                start.format("%b %d"),
                start.format("%I:%M %p"),
                end.format("%b %d"),
                end.format("%I:%M %p"))
        }
    }
}
```

#### Tooltip Content
```
┌─────────────────────────────────┐
│ 🎄 Christmas Cruise             │
│ Dec 18 - Dec 25, 2025           │  <- NEW: Date range
│ 23 days remaining               │
│ Location: Sydney Harbour        │  <- If available
└─────────────────────────────────┘
```

---

## Implementation Order

1. **Dim Past All-Day Events** - Quick win, 1-2 hours
2. **Countdown Card Tooltip Date Range** - Quick win, 30 mins
3. **Countdown Card Container** - Larger feature, 4-8 hours
   - Phase 1: Database schema & migrations
   - Phase 2: Container service layer
   - Phase 3: Container UI component
   - Phase 4: Drag & drop reordering
   - Phase 5: Multiple containers (optional)

---

*Last Updated: November 25, 2025*
