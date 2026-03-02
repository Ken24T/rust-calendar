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
**Status:** ✅ Shipped (v2.1.0–v2.4.0)

Category containers, drag-and-drop reordering, collapsible headers, sort modes,
and card templates have all been shipped. See
[COUNTDOWN_TIMER_FEATURE.md](COUNTDOWN_TIMER_FEATURE.md) for the current
implementation.

### Remaining Future Extensions


- Export/share containers
- Priority-based sort mode

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

```text
┌─────────────────────────────────┐
│ 🎄 Christmas Cruise             │
│ Dec 18 - Dec 25, 2025           │  <- NEW: Date range
│ 23 days remaining               │
│ Location: Sydney Harbour        │  <- If available
└─────────────────────────────────┘
```

---

## Implementation Order

1. **Dim Past All-Day Events** — Quick win, 1–2 hours
2. **Countdown Card Tooltip Date Range** — Quick win, 30 mins

---

<!-- Last Updated: March 2, 2026 -->
