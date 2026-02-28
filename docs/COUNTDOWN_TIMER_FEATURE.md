# Desktop Countdown Timer Feature

> **Status: Implemented** — This feature has been built and shipped. This document is the original design specification, retained for reference. Actual implementation may differ in details.

## Overview

The Desktop Countdown Timer feature allows users to "tear" events from the calendar and place them on their desktop as persistent, always-on-top countdown widgets. This provides at-a-glance visibility of upcoming events without needing to keep the calendar open.

## Feature Summary

**Core Functionality**:
- Drag any future event from the calendar to the desktop
- Creates a persistent countdown timer window
- Original event remains in calendar
- Always-on-top, movable, customizable widget
- Live countdown updates every second
- Multiple timers can coexist
- Auto-dismiss when event starts (configurable)

## User Workflow

### Creating a Countdown Timer

**Method 1: Drag and Drop (Primary)**
```
1. User clicks and holds event in calendar
2. Drags cursor outside calendar window
3. Desktop drop zone indicator appears
4. Releases mouse at desired position on desktop
5. Countdown timer window appears
6. Event remains in calendar unchanged
```

**Method 2: Context Menu (Alternative)**
```
1. User right-clicks event in calendar
2. Selects "Create Desktop Countdown"
3. Timer appears at default position
```

### Countdown Timer States

```
┌─────────────────────────────────────────┐
│           COUNTDOWN LIFECYCLE            │
└─────────────────────────────────────────┘
        │
        ▼
   [CREATED]
   Window opens, countdown begins
        │
        ▼
   [ACTIVE]
   Normal countdown display
   Updates every second
        │
        ▼
   [WARNING] (< 5 min)
   Visual warning indicator
   Flash/blink/color change
        │
        ▼
   [STARTED]
   Event start time reached
   Optional notification
        │
        ▼
   [DISMISSED]
   Window closed (auto or manual)
```

## Technical Implementation

### Database Schema

**countdown_timers table**:
```sql
CREATE TABLE countdown_timers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    position_x INTEGER NOT NULL,    -- Desktop X coordinate
    position_y INTEGER NOT NULL,    -- Desktop Y coordinate
    width INTEGER NOT NULL DEFAULT 300,
    height INTEGER NOT NULL DEFAULT 150,
    auto_dismiss BOOLEAN NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
);
```

### Data Models

**CountdownTimer Model**:
```rust
pub struct CountdownTimer {
    pub id: Option<i64>,
    pub event_id: i64,
    pub position: Position,
    pub size: Size,
    pub auto_dismiss: bool,
    pub created_at: DateTime<Local>,
}

pub struct Position {
    pub x: i32,
    pub y: i32,
}

pub struct Size {
    pub width: u32,
    pub height: u32,
}
```

**CountdownState Enum**:
```rust
pub enum CountdownState {
    Active,         // Normal countdown
    Warning,        // Less than warning threshold
    Started,        // Event has started
    Past,          // Event has ended
}

impl CountdownState {
    pub fn from_time_remaining(duration: Duration, warning_min: u32) -> Self {
        if duration.num_seconds() < 0 {
            CountdownState::Past
        } else if duration.num_seconds() == 0 {
            CountdownState::Started
        } else if duration.num_minutes() < warning_min as i64 {
            CountdownState::Warning
        } else {
            CountdownState::Active
        }
    }
}
```

### Module Structure

**New Files**:
```
src/
├── ui/components/
│   └── countdown_timer.rs          # Countdown window component
├── models/ui/
│   └── countdown_config.rs         # Countdown configuration
└── services/countdown/
    ├── mod.rs                      # Countdown service exports
    ├── manager.rs                  # Countdown timer manager
    └── window.rs                   # Window creation/management
```

### Countdown Service

**CountdownService Implementation**:
```rust
pub struct CountdownService {
    db: Arc<Database>,
    active_timers: HashMap<i64, CountdownWindow>,
    preferences: CountdownPreferences,
}

impl CountdownService {
    /// Create a new countdown timer for an event
    pub fn create_countdown(
        &mut self,
        event: &Event,
        position: Position
    ) -> Result<i64> {
        // 1. Validate event is in the future
        // 2. Create timer record in database
        // 3. Create countdown window
        // 4. Add to active timers map
        // 5. Return timer ID
    }
    
    /// Remove a countdown timer
    pub fn dismiss_countdown(&mut self, timer_id: i64) -> Result<()> {
        // 1. Remove from active timers
        // 2. Close window
        // 3. Delete from database
    }
    
    /// Update all active countdown displays
    /// Called every second by background thread
    pub fn update_all_countdowns(&mut self) {
        let now = Local::now();
        let mut to_dismiss = Vec::new();
        
        for (id, window) in &mut self.active_timers {
            let event_start = window.event.start;
            let remaining = event_start.signed_duration_since(now);
            
            // Update display
            window.update_countdown(remaining);
            
            // Check if should auto-dismiss
            if self.preferences.auto_dismiss_on_start && remaining.num_seconds() <= 0 {
                to_dismiss.push(*id);
            }
        }
        
        // Dismiss expired timers
        for id in to_dismiss {
            let _ = self.dismiss_countdown(id);
        }
    }
    
    /// Restore timers from database on app startup
    pub fn restore_timers(&mut self) -> Result<()> {
        let timers = self.db.get_all_countdown_timers()?;
        
        for timer in timers {
            let event = self.db.get_event(timer.event_id)?;
            
            // Skip if event is in the past
            if event.start < Local::now() {
                self.db.delete_countdown_timer(timer.id.unwrap())?;
                continue;
            }
            
            // Recreate window
            let window = self.create_window(&event, timer.position)?;
            self.active_timers.insert(timer.id.unwrap(), window);
        }
        
        Ok(())
    }
    
    /// Handle event update - refresh countdown if exists
    pub fn on_event_updated(&mut self, event_id: i64) {
        // Find countdown with this event_id
        // Update event data
        // Refresh display
    }
    
    /// Handle event deletion - remove countdown
    pub fn on_event_deleted(&mut self, event_id: i64) {
        // Find countdown with this event_id
        // Dismiss countdown
    }
}
```

## Time Granularity & Default Duration Settings

### Time Interval Configuration

**Purpose**: Control the granularity of time slots in day/week views.

**Options**:
- **15 minutes**: Fine-grained scheduling
  - Use case: Dense schedules, frequent meetings
  - 4 slots per hour = 48 rows for 12-hour day
  
- **30 minutes**: Balanced view
  - Use case: Moderate scheduling
  - 2 slots per hour = 24 rows for 12-hour day
  
- **60 minutes**: Default, clean view
  - Use case: High-level overview
  - 1 slot per hour = 12 rows for 12-hour day

**Database Storage**:
```sql
-- In ui_preferences table
time_interval INTEGER NOT NULL DEFAULT 60  -- 15, 30, or 60
```

**Settings UI**:
```
┌──────────────────────────────────────┐
│ Time Display Settings                │
├──────────────────────────────────────┤
│ Time Slot Interval:                  │
│   ○ 15 minutes                       │
│   ○ 30 minutes                       │
│   ● 60 minutes (Hourly)              │
│                                      │
│ Preview: 12-hour day                 │
│   15 min: 48 rows                    │
│   30 min: 24 rows                    │
│   60 min: 12 rows  ← Current         │
└──────────────────────────────────────┘
```

### Default Event Duration

**Purpose**: Set the default duration when creating new events.

**Default Value**: 45 minutes
- Most meetings are 30-60 minutes
- 45 minutes is a common meeting length
- User can adjust per-event after creation

**Configuration**:
```sql
-- In ui_preferences table
default_event_duration INTEGER NOT NULL DEFAULT 45  -- minutes
```

**Usage**:
1. User drags on calendar to create new event
2. Event is created with default duration (45 min)
3. User can immediately resize if needed
4. Or adjust in event edit dialog

**Settings UI**:
```
┌──────────────────────────────────────┐
│ Event Settings                       │
├──────────────────────────────────────┤
│ Default Event Duration:              │
│   [45] minutes                       │
│   ◄────────●────────►                │
│   15      45      120                │
│                                      │
│ Common durations:                    │
│   [15 min] [30 min] [45 min]        │
│   [60 min] [90 min] [120 min]       │
└──────────────────────────────────────┘
```

**Implementation**:
```rust
pub struct ViewPreferences {
    pub time_interval: u32,          // 15, 30, or 60 minutes
    pub default_event_duration: u32, // Default: 45 minutes
    // ... other fields
}

impl ViewPreferences {
    pub fn create_default_event(&self, start: DateTime<Local>) -> Event {
        let duration = Duration::minutes(self.default_event_duration as i64);
        let end = start + duration;
        
        Event::builder()
            .start(start)
            .end(end)
            .build()
    }
}
```

## Countdown Display Format

### Time Remaining Formats

**Format Rules**:
```rust
pub fn format_countdown(remaining: Duration) -> String {
    let total_seconds = remaining.num_seconds();
    
    if total_seconds <= 0 {
        return "Starting now!".to_string();
    }
    
    let days = remaining.num_days();
    let hours = remaining.num_hours() % 24;
    let minutes = remaining.num_minutes() % 60;
    let seconds = remaining.num_seconds() % 60;
    
    match days {
        d if d > 1 => format!("{} days, {} hours", d, hours),
        1 => format!("1 day, {} hours", hours),
        0 => {
            if hours > 0 {
                format!("{} hours, {} minutes", hours, minutes)
            } else if minutes > 0 {
                format!("{} minutes, {} seconds", minutes, seconds)
            } else {
                format!("{} seconds", seconds)
            }
        }
        _ => "Starting now!".to_string(),
    }
}
```

**Display Examples**:
- `"5 days, 3 hours"`
- `"1 day, 8 hours"`
- `"3 hours, 45 minutes"`
- `"45 minutes, 30 seconds"`
- `"30 seconds"`
- `"Starting now!"` (flashing)

### Visual States

**Active State** (Normal):
- Steady display
- Standard colors
- Updates every second

**Warning State** (< 5 minutes):
- Flashing/pulsing animation
- Color change (yellow/orange)
- More prominent display
- Sound notification (optional)

**Started State** (Event time reached):
- Bright flash
- "Starting now!" text
- Sound alert (optional)
- Auto-dismiss after X seconds (if enabled)

## User Preferences

### CountdownPreferences Structure

```rust
pub struct CountdownPreferences {
    // Auto-dismiss
    pub auto_dismiss_on_start: bool,     // Default: true
    pub auto_dismiss_delay: u32,         // Seconds to wait (default: 10)
    
    // Display
    pub show_seconds: bool,              // Show seconds in countdown
    pub show_date: bool,                 // Show event date
    pub show_location: bool,             // Show event location
    
    // Warnings
    pub warning_minutes: u32,            // Warning threshold (default: 5)
    pub flash_warning: bool,             // Flash window when imminent
    pub warning_sound: bool,             // Play sound at warning
    pub start_sound: bool,               // Play sound when event starts
    
    // Window
    pub default_width: u32,              // Default: 300px
    pub default_height: u32,             // Default: 150px
    pub opacity: f32,                    // 0.0-1.0 (default: 0.95)
    pub always_on_top: bool,             // Default: true
    
    // Positioning
    pub position_mode: PositionMode,     // Default: Smart
    pub smart_offset: u32,               // Pixels between timers (default: 20)
}

pub enum PositionMode {
    Smart,      // Auto-position to avoid overlaps
    Manual,     // Always use drop position
    Remember,   // Remember position per event
}
```

### Settings UI

```
┌─────────────────────────────────────────────┐
│ Countdown Timer Settings                    │
├─────────────────────────────────────────────┤
│ Auto-Dismiss:                               │
│   ☑ Auto-dismiss when event starts          │
│   Delay: [10] seconds                       │
│                                             │
│ Display Options:                            │
│   ☑ Show seconds in countdown               │
│   ☑ Show event date                         │
│   ☑ Show event location                     │
│                                             │
│ Warnings:                                   │
│   Warning threshold: [5] minutes            │
│   ☑ Flash window when imminent              │
│   ☑ Play sound at warning                   │
│   ☑ Play sound when event starts            │
│                                             │
│ Window Appearance:                          │
│   Default size: [300] × [150] pixels        │
│   Opacity: ◄────────●─────► 95%            │
│   ☑ Always on top                           │
│                                             │
│ Positioning:                                │
│   ● Smart (auto-position)                   │
│   ○ Manual (use drop position)              │
│   ○ Remember (per-event memory)             │
│                                             │
│ [Save]  [Cancel]  [Reset to Defaults]      │
└─────────────────────────────────────────────┘
```

## Testing Requirements

### Unit Tests

**Countdown Formatting**:
- Test time remaining formatting at various intervals
- Test edge cases (0 seconds, negative time, etc.)
- Test state transitions

**State Management**:
- Test state calculation from time remaining
- Test warning threshold detection
- Test auto-dismiss logic

### Integration Tests

**Timer Creation**:
- Test drag-and-drop to desktop
- Test context menu creation
- Test position validation
- Test multiple timer creation

**Timer Lifecycle**:
- Test timer updates every second
- Test warning state activation
- Test auto-dismiss behavior
- Test manual dismiss

**Persistence**:
- Test saving timer to database
- Test restoring timers on startup
- Test removing expired timers

### UI Tests

**Drag Operation**:
- Test drag from calendar to desktop
- Test visual feedback during drag
- Test invalid drop zones
- Test drop position calculation

**Window Behavior**:
- Test always-on-top functionality
- Test window movement
- Test window resizing (if enabled)
- Test click to open in calendar

## Future Enhancements

**Potential Features**:
- [ ] Custom countdown timer appearance themes
- [ ] Countdown timer groups (folders)
- [ ] Countdown timer templates
- [ ] Export countdown as image
- [ ] Share countdown timer link (if cloud sync added)
- [ ] Countdown timer widgets for multiple monitors
- [ ] Countdown timer for recurring event series
- [ ] Integration with system tray (minimize to tray)
- [ ] Countdown timer history

## User Benefits

✅ **At-a-Glance Visibility**: See important event countdowns without opening calendar
✅ **Multi-Tasking**: Keep working while monitoring multiple event countdowns
✅ **Meeting Preparation**: Visual reminder to prepare for upcoming meetings
✅ **Deadline Awareness**: Constant visibility of project deadlines
✅ **Customizable**: Configure behavior to match personal preferences
✅ **Non-Intrusive**: Small, movable windows that don't block workflow
✅ **Persistent**: Survives application restarts
✅ **Integrated**: Changes to events update countdown automatically

---

**Status**: Design complete, awaiting user specification of visual appearance
**Next Steps**: 
1. User to specify countdown timer visual design
2. Implement drag-to-desktop functionality
3. Create countdown window component
4. Implement countdown service
5. Add persistence and restoration
6. Comprehensive testing
