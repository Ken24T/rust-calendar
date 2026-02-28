# My Day Panel & Multi-Day Event Ribbon Features

> **Status: Implemented** — Both features have been built and shipped. This document is the original design specification, retained for reference. Actual implementation may differ in details.

## Overview

Two complementary UI features that enhance calendar usability:
1. **My Day Panel** - A sidebar showing the selected day's events
2. **Multi-Day Event Ribbon** - A horizontal strip displaying multi-day events

These features work together to provide focused daily views while keeping long-running events visible without cluttering the main calendar grid.

---

## My Day Panel

### Purpose

Provides a dedicated, always-visible list of events for the currently selected date, enabling quick reference without switching calendar views.

### Key Benefits

✅ **Quick Daily Overview**: See all events for selected day at a glance  
✅ **Navigation Aid**: Acts as a "mini agenda" for the active date  
✅ **Space Efficient**: Compact sidebar doesn't obscure calendar  
✅ **Drag Source**: Easy drag-and-drop to calendar or desktop  
✅ **Customizable**: Adjustable position, width, and display options  

### Features

#### Display

- **Header**: Shows selected date (e.g., "Thu, May 15, 2025")
- **All-Day Events**: Listed at top with special styling
- **Timed Events**: Chronological list with start/end times
- **Event Details**:
  - Color-coded indicator dot matching event category
  - Event title
  - Time range (e.g., "8:00 AM - 9:30 AM")
  - Location (optional, configurable)
  - Duration badge (optional, configurable)

#### Positioning Options

| Position | Description | Use Case |
|----------|-------------|----------|
| **Left** | Panel on left side (default) | Standard layout, right-handed mouse users |
| **Right** | Panel on right side | Left-handed users, preference |
| **Hidden** | Panel completely hidden | Maximize calendar space |

#### Width

- **Default**: 250px
- **Minimum**: 180px (prevents content truncation)
- **Maximum**: 400px (prevents excessive space usage)
- **Adjustment**: Drag handle on panel edge
- **Persistence**: Width saved in preferences

### Visual Design

```
┌─────────────────────────┐
│ Thu, May 15, 2025    🗓 │  ← Date header with icon
├─────────────────────────┤
│ All Day                 │  ← Section header
│ 🔵 Birthday Party       │  ← All-day event
├─────────────────────────┤
│ 8:00 AM - 9:30 AM       │  ← Time range
│ 🟢 Team Standup         │  ← Event with color
│   📍 Conference Room A  │  ← Location (optional)
├─────────────────────────┤
│ 10:00 AM - 10:30 AM     │
│ 🟡 Coffee with Sarah    │
│   📍 Cafe Downtown      │
│   ⏱ 30 min             │  ← Duration (optional)
├─────────────────────────┤
│ 12:00 PM - 1:00 PM      │
│ 🔴 Lunch Break          │
├─────────────────────────┤
│ 2:00 PM - 3:00 PM       │
│ 🟣 Project Review       │
│   💻 Virtual (Zoom)     │
├─────────────────────────┤
│ 5:30 PM - 7:00 PM       │
│ 🟠 Gym Session          │
│   📍 Fitness Center     │
└─────────────────────────┘
       ↕ Resize handle
```

### Interactions

#### Event Interactions

| Action | Result |
|--------|--------|
| **Click** | Selects event (highlights in calendar) |
| **Double-click** | Opens event editor dialog |
| **Right-click** | Shows context menu:<br>- Edit Event<br>- Delete Event<br>- Duplicate Event<br>- Create Countdown Timer<br>- Copy Event Link |
| **Drag** | Initiates drag operation:<br>- To calendar: Move/reschedule event<br>- To desktop: Create countdown timer |

#### Panel Controls

- **Date Header**:
  - Click: Opens date picker for quick navigation
  - Shows day name and full date
  
- **Resize Handle**:
  - Appears on edge when hovering near panel border
  - Drag left/right to adjust width
  - Visual feedback during resize
  - Snaps to min/max boundaries

- **Toggle Button** (in main toolbar):
  - Icon: Panel with checkmark
  - Toggles panel visibility
  - Keyboard shortcut: `Ctrl+Shift+D`

- **Position Button** (in preferences):
  - Radio buttons: Left / Right / Hidden
  - Takes effect immediately

#### Selection Synchronization

**Bidirectional Sync**:
```
Calendar → My Day Panel
  User clicks date in calendar
  → Panel updates to show that date's events

My Day Panel → Calendar
  User clicks event in panel
  → Event highlights in calendar
  → Calendar scrolls to event time (if needed)
```

### Empty State

When selected day has no events:

```
┌─────────────────────────┐
│ Thu, May 15, 2025       │
├─────────────────────────┤
│                         │
│         📅              │  ← Large calendar icon
│                         │
│   No events today       │  ← Message
│                         │
│  Click + to add event   │  ← Call-to-action
│                         │
│  or drag to create      │  ← Alternative action
│                         │
└─────────────────────────┘
```

### Configuration

**Preferences Dialog - My Day Tab**:

```
┌─────────────────────────────────────────────┐
│ My Day Panel Settings                       │
├─────────────────────────────────────────────┤
│                                             │
│ ⚙ Position                                  │
│   ● Left side                               │
│   ○ Right side                              │
│   ○ Hidden                                  │
│                                             │
│ ⚙ Panel Width                               │
│   [250] pixels                              │
│   ◄─────────●──────────►                    │
│   180              400                      │
│                                             │
│ ⚙ Display Options                           │
│   ☑ Show event location                     │
│   ☑ Show event duration badge               │
│   ☑ Show all-day events at top              │
│   ☑ Show empty state message                │
│   ☑ Color-code events by category           │
│   ☑ Show event icons (location, video, etc) │
│                                             │
│ ⚙ Font Settings                             │
│   Font Size: [13] px                        │
│   ◄─────●──────►                            │
│   10         16                             │
│                                             │
│   Font Weight:                              │
│   ○ Light  ● Regular  ○ Bold                │
│                                             │
│ ⚙ Scrolling                                 │
│   ☑ Smooth scroll animation                 │
│   ☑ Auto-scroll to first event              │
│                                             │
├─────────────────────────────────────────────┤
│         [Apply]  [Cancel]  [Reset]          │
└─────────────────────────────────────────────┘
```

### Data Model

```rust
pub struct MyDayPanelConfig {
    pub visible: bool,
    pub position: PanelPosition,
    pub width: u32,              // pixels, 180-400
    pub show_location: bool,
    pub show_duration: bool,
    pub show_all_day_at_top: bool,
    pub show_empty_state: bool,
    pub color_code_events: bool,
    pub show_icons: bool,
    pub font_size: u32,          // pixels, 10-16
    pub font_weight: FontWeight, // Light, Regular, Bold
    pub smooth_scroll: bool,
    pub auto_scroll_to_first: bool,
}

pub enum PanelPosition {
    Left,
    Right,
    Hidden,
}

pub enum FontWeight {
    Light,
    Regular,
    Bold,
}
```

### Database Schema

Added to `ui_preferences` table:

```sql
-- My Day panel preferences
my_day_visible BOOLEAN NOT NULL DEFAULT 1,
my_day_position TEXT NOT NULL DEFAULT 'left',  -- left, right, hidden
my_day_width INTEGER NOT NULL DEFAULT 250,     -- pixels
my_day_show_location BOOLEAN NOT NULL DEFAULT 1,
my_day_show_duration BOOLEAN NOT NULL DEFAULT 1,
my_day_font_size INTEGER NOT NULL DEFAULT 13,
```

---

## Multi-Day Event Ribbon

### Purpose

Displays events spanning multiple days in a dedicated horizontal strip, preventing them from dominating the main calendar grid and obscuring single-day events.

### Key Benefits

✅ **Clean Calendar**: Removes clutter from main grid  
✅ **Persistent Visibility**: Long events always visible  
✅ **Progress Tracking**: Visual indicators for ongoing events  
✅ **Quick Navigation**: Click to jump to event date  
✅ **Flexible Display**: Multiple modes (compact/expanded/auto)  

### Display Criteria

Events appear in ribbon if they match **any** condition:

| Condition | Example | Rationale |
|-----------|---------|-----------|
| **2+ Days** | Conference May 20-21 | Multi-day event |
| **All-Day Multi-Day** | Vacation May 15-22 | Week-long absence |
| **Weekend-Crossing** | Project Fri-Mon | Spans work/personal boundary |
| **Manually Pinned** | Important Deadline | User override |

**Minimum Days Threshold**: Configurable (default: 2 days)

### Visual Layout

```
┌────────────────────────────────────────────────────────────────────────┐
│  [Multi-Day Event Ribbon]                         🔽 ⚙ ✕              │
│                                                   Expand Settings Hide  │
├────────────────────────────────────────────────────────────────────────┤
│ ┌───────────────────────┐ ┌──────────────┐ ┌─────────────────────┐   │
│ │ 🏖 Vacation           │ │ 📊 Conference│ │ 💼 Project Alpha    │   │
│ │ May 15-22 (8 days)    │ │ May 20-21    │ │ May 10 - Jun 15     │   │
│ │ ████████████████████  │ │ ████         │ │ ████████████░░░░░░  │   │
│ │      Current: Day 3   │ │ Upcoming     │ │  Progress: 65%      │   │
│ └───────────────────────┘ └──────────────┘ └─────────────────────┘   │
│ ◀                                                                  ▶   │
└────────────────────────────────────────────────────────────────────────┘
```

### Display Modes

#### 1. Compact Mode (Default)

```
┌──────────────────────────────────────────────────────────────┐
│ ◀ │ Vacation (5/15-22) │ Conf (5/20-21) │ ... (3 more) │ ▶ │
└──────────────────────────────────────────────────────────────┘
```

- **Single row**
- Horizontal scrolling if needed
- Minimalist event cards
- Height: 60px (configurable)

#### 2. Expanded Mode

```
┌─────────────────────────────────────────────────────────────────┐
│ Row 1: │ 🏖 Vacation ████████████████████ May 15-22 (8 days)  │
│ Row 2: │ 📊 Conference ████ May 20-21 (2 days)                 │
│ Row 3: │ 💼 Project Alpha ██████████████████ May 10 - Jun 15  │
│ Row 4: │ 🎓 Training Week ██████ May 18-22 (5 days)           │
└─────────────────────────────────────────────────────────────────┘
```

- **Multiple rows**
- All events visible simultaneously
- Detailed event cards with progress bars
- Height: 120px (configurable)
- No scrolling needed

#### 3. Auto Mode

**Logic**:
```
if event_count <= 3:
    display = Compact
else:
    display = Expanded
```

**Behavior**:
- Automatically switches based on event count
- Smooth transitions between modes
- Best of both worlds

### Event Card Components

#### Card Elements

```
┌──────────────────────────────┐
│ 🏖 Vacation                  │  ← Icon + Title
│ May 15-22 (8 days)           │  ← Date range + duration
│ ████████████████████         │  ← Progress bar (75% filled)
│      Day 6 of 8              │  ← Progress text
└──────────────────────────────┘
```

#### Icon Types

| Icon | Meaning |
|------|---------|
| 🏖 | Vacation/Holiday |
| 📊 | Conference/Business |
| 💼 | Project/Work |
| 🎓 | Training/Education |
| 🔧 | Maintenance/Deployment |
| 📅 | Generic multi-day event |

#### Progress Indicators

**For Ongoing Events**:
- Progress bar shows elapsed/remaining time
- Text: "Day X of Y" or "X days remaining"
- Color coding:
  - Green: Just started (< 25% complete)
  - Blue: In progress (25-75%)
  - Orange: Nearing end (> 75%)

**For Upcoming Events**:
- No progress bar
- Text: "Starts in X days"
- Icon: ⏳

**For Past Events**:
- Grayed out (optional auto-hide)
- Text: "Ended X days ago"

### Interactions

#### Event Card Actions

| Action | Result |
|--------|--------|
| **Click** | Navigate calendar to event start date<br>Highlights event in calendar view |
| **Double-click** | Opens event editor dialog |
| **Hover** | Shows tooltip with full details:<br>- Complete title<br>- Full date range<br>- Description (first 100 chars)<br>- Location<br>- Attendees count |
| **Right-click** | Context menu:<br>- Edit Event<br>- Delete Event<br>- Unpin from Ribbon<br>- Create Countdown Timer<br>- View in Calendar<br>- Export to .ics |

#### Ribbon Controls

**Top-Right Controls**:
```
🔽  ⚙  ✕
│   │  └─ Hide Ribbon (keyboard: Ctrl+Shift+R)
│   └──── Settings (opens ribbon preferences)
└──────── Expand/Collapse (toggles mode)
```

**Scroll Navigation** (Compact mode):
```
◀                                                 ▶
└─ Previous events        Next events ──────────┘
```

### Configuration

**Preferences Dialog - Ribbon Tab**:

```
┌──────────────────────────────────────────────────┐
│ Multi-Day Event Ribbon Settings                  │
├──────────────────────────────────────────────────┤
│                                                  │
│ ⚙ Display Mode                                   │
│   ○ Compact (single row, scroll if needed)       │
│   ○ Expanded (multiple rows, all visible)        │
│   ● Auto (switches based on event count)         │
│                                                  │
│   Auto expands when: [3] or more events          │
│   ◄────●────►                                    │
│   1        10                                    │
│                                                  │
│ ⚙ Height Settings                                │
│   Compact mode:  [60] px                         │
│   ◄─────●────────►                               │
│   40           100                               │
│                                                  │
│   Expanded mode: [120] px                        │
│   ◄─────●────────►                               │
│   80           200                               │
│                                                  │
│ ⚙ Event Criteria                                 │
│   Show events spanning:                          │
│   ☑ 2+ days                                      │
│   ☑ Multi-day all-day events                     │
│   ☑ Weekend-crossing events                      │
│   ☑ Manually pinned events                       │
│                                                  │
│   Minimum span: [2] days                         │
│   ◄────●─────────►                               │
│   1             7                                │
│                                                  │
│ ⚙ Display Options                                │
│   ☑ Show duration text (e.g., "8 days")          │
│   ☑ Show progress bars for ongoing events        │
│   ☑ Show event type icons                        │
│   ☑ Show current day marker                      │
│   ☑ Color-code by category                       │
│   ☑ Show "Starts in X days" for upcoming         │
│   ☐ Show past events (grayed out)                │
│                                                  │
│ ⚙ Position                                       │
│   ● Top (above calendar)                         │
│   ○ Bottom (below calendar)                      │
│   ○ Hidden                                       │
│                                                  │
│ ⚙ Animation                                      │
│   ☑ Smooth expand/collapse transitions           │
│   ☑ Fade in new events                           │
│   Transition duration: [300] ms                  │
│                                                  │
├──────────────────────────────────────────────────┤
│         [Apply]  [Cancel]  [Reset to Defaults]   │
└──────────────────────────────────────────────────┘
```

### Data Model

```rust
pub struct RibbonConfig {
    pub visible: bool,
    pub mode: RibbonMode,
    pub position: RibbonPosition,
    pub compact_height: u32,       // pixels, 40-100
    pub expanded_height: u32,      // pixels, 80-200
    pub min_days: u32,             // minimum days to show (1-7)
    pub auto_expand_threshold: u32, // event count for auto mode
    pub show_multiday: bool,
    pub show_all_day_multiday: bool,
    pub show_weekend_crossing: bool,
    pub show_pinned: bool,
    pub show_duration_text: bool,
    pub show_progress_bars: bool,
    pub show_icons: bool,
    pub show_current_marker: bool,
    pub color_code: bool,
    pub show_upcoming_countdown: bool,
    pub show_past_events: bool,
    pub smooth_transitions: bool,
    pub fade_new_events: bool,
    pub transition_duration_ms: u32,
}

pub enum RibbonMode {
    Compact,
    Expanded,
    Auto,
}

pub enum RibbonPosition {
    Top,
    Bottom,
    Hidden,
}
```

### Database Schema

Added to `ui_preferences` table:

```sql
-- Multi-day ribbon preferences
ribbon_visible BOOLEAN NOT NULL DEFAULT 1,
ribbon_mode TEXT NOT NULL DEFAULT 'auto',      -- compact, expanded, auto
ribbon_position TEXT NOT NULL DEFAULT 'top',   -- top, bottom, hidden
ribbon_compact_height INTEGER NOT NULL DEFAULT 60,
ribbon_expanded_height INTEGER NOT NULL DEFAULT 120,
ribbon_min_days INTEGER NOT NULL DEFAULT 2,    -- minimum days to show in ribbon
ribbon_show_progress BOOLEAN NOT NULL DEFAULT 1,
ribbon_show_icons BOOLEAN NOT NULL DEFAULT 1,
```

---

## Integration with Calendar Views

### Layout Hierarchy

```
┌────────────────────────────────────────────────────────────────┐
│  Application Window                                            │
│  ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓  │
│  ┃ 1. Multi-Day Event Ribbon (Optional, configurable)       ┃  │
│  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛  │
│  ┌───────────┬────────────────────────────────────────────────┐ │
│  │           │                                                │ │
│  │ My Day    │         Main Calendar View                    │ │
│  │ Panel     │         (Day/Week/Month/Quarter/Year/Agenda)  │ │
│  │ (Optional)│                                                │ │
│  │           │                                                │ │
│  │           │                                                │ │
│  └───────────┴────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

### Responsive Behavior

**Window Width < 800px**:
- My Day panel auto-hides
- Ribbon switches to compact mode
- User can manually toggle panel

**Window Width 800-1200px**:
- My Day panel visible at minimum width (180px)
- Ribbon uses auto mode
- Comfortable viewing

**Window Width > 1200px**:
- My Day panel at preferred width (250px default)
- Ribbon uses expanded mode if many events
- Optimal experience

### View-Specific Considerations

| View | My Day Panel | Multi-Day Ribbon |
|------|--------------|------------------|
| **Day** | Shows selected day | Shows multi-day events overlapping selected day |
| **Week** | Shows selected day (click in week) | Shows multi-day events in week range |
| **Month** | Shows selected day (click in grid) | Shows all multi-day events in month |
| **Quarter** | Shows selected day | Shows multi-day events in quarter |
| **Year** | Shows selected day | Hidden (too many events) |
| **Agenda** | Follows selected event's date | Hidden (redundant with agenda) |

---

## Implementation Notes

### Module Structure

```
src/ui/components/
├── my_day_panel.rs      # My Day panel component
│   ├── header
│   ├── event_list
│   ├── empty_state
│   └── resize_handle
│
└── ribbon.rs            # Multi-day ribbon component
    ├── ribbon_container
    ├── event_card
    ├── progress_bar
    ├── scroll_controls
    └── mode_switcher

src/models/ui/
├── my_day_config.rs     # MyDayPanelConfig struct
└── ribbon_config.rs     # RibbonConfig struct

src/services/
└── event_filter.rs      # Logic to determine ribbon eligibility
    ├── is_multiday()
    ├── spans_weekend()
    ├── meets_criteria()
    └── calculate_progress()
```

### State Management

```rust
pub struct AppState {
    pub selected_date: Date,
    pub my_day_events: Vec<Event>,
    pub ribbon_events: Vec<Event>,
    pub my_day_config: MyDayPanelConfig,
    pub ribbon_config: RibbonConfig,
}

impl AppState {
    pub fn update_selected_date(&mut self, date: Date) {
        self.selected_date = date;
        self.refresh_my_day_events();
        self.refresh_ribbon_events();
    }
    
    fn refresh_my_day_events(&mut self) {
        self.my_day_events = self.event_service
            .get_events_for_date(self.selected_date);
    }
    
    fn refresh_ribbon_events(&mut self) {
        let range = self.current_view_date_range();
        let all_events = self.event_service.get_events_in_range(range);
        
        self.ribbon_events = all_events.into_iter()
            .filter(|e| self.should_show_in_ribbon(e))
            .collect();
    }
    
    fn should_show_in_ribbon(&self, event: &Event) -> bool {
        let duration_days = event.duration_days();
        
        if duration_days < self.ribbon_config.min_days {
            return false;
        }
        
        if event.is_pinned_to_ribbon {
            return true;
        }
        
        if self.ribbon_config.show_multiday && duration_days >= 2 {
            return true;
        }
        
        if self.ribbon_config.show_weekend_crossing && event.spans_weekend() {
            return true;
        }
        
        false
    }
}
```

### Performance Considerations

- **Event Filtering**: Cache ribbon-eligible events
- **Rendering**: Virtualize long event lists in My Day panel
- **Updates**: Debounce rapid date changes
- **Animations**: Use GPU-accelerated transforms
- **Memory**: Limit visible ribbon events (e.g., max 20)

---

## Testing Requirements

### Unit Tests

**My Day Panel**:
- [ ] Event list rendering with various event counts
- [ ] Empty state display
- [ ] Width constraints (min/max enforcement)
- [ ] Date header formatting
- [ ] Event time formatting

**Multi-Day Ribbon**:
- [ ] Event filtering logic (2+ days, weekend-crossing)
- [ ] Progress calculation for ongoing events
- [ ] Mode switching (compact/expanded/auto)
- [ ] Event card rendering

### Integration Tests

- [ ] My Day panel updates when calendar date changes
- [ ] Ribbon updates when calendar view range changes
- [ ] Clicking event in My Day highlights in calendar
- [ ] Clicking ribbon event navigates calendar
- [ ] Drag event from My Day to desktop creates countdown
- [ ] Panel resize persists across app restarts
- [ ] Ribbon configuration persists across app restarts

### UI Tests

- [ ] My Day panel drag handle responds to mouse
- [ ] Ribbon scroll arrows navigate correctly
- [ ] Ribbon expand/collapse transitions smoothly
- [ ] Responsive layout adapts to window resize
- [ ] Tooltips appear on ribbon event hover

---

## User Stories

### My Day Panel

1. **Daily Planning**
   - As a user, I want to see all my events for today in one place
   - So I can plan my day without switching views

2. **Quick Navigation**
   - As a user, I want to click a date in the calendar and see its events
   - So I can explore my schedule efficiently

3. **Space Management**
   - As a user, I want to hide the My Day panel
   - So I have more space for the calendar when needed

### Multi-Day Ribbon

1. **Vacation Tracking**
   - As a user, I want to see my week-long vacation at the top
   - So I know it's happening without it blocking my daily events

2. **Project Awareness**
   - As a user, I want to see ongoing multi-week projects
   - So I'm aware of them while scheduling daily tasks

3. **Conference Visibility**
   - As a user, I want multi-day conferences to stay visible
   - So I don't accidentally schedule conflicts

---

## Future Enhancements

### My Day Panel

- [ ] Collapsible sections (All Day, Morning, Afternoon, Evening)
- [ ] Inline event editing (click to edit fields directly)
- [ ] Mini calendar widget in panel header
- [ ] Task list integration (show tasks alongside events)
- [ ] Weather forecast for selected day

### Multi-Day Ribbon

- [ ] Timeline view (graphical timeline of overlapping events)
- [ ] Event dependencies visualization
- [ ] Milestone markers
- [ ] Team member avatars for shared events
- [ ] Color themes per event category

---

**Status**: Design complete, ready for implementation  
**Dependencies**: Core event system, UI framework, preferences service  
**Estimated Implementation**: Phase 3 (UI development)

