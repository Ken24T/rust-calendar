# UI System Documentation

## Overview

The Rust Calendar application features a comprehensive, highly customizable user interface with multiple view types, persistent preferences, and drag-and-drop functionality.

## Main UI Layout

The application window consists of three primary areas:

1. **Multi-Day Event Ribbon** (Top) - Displays events spanning multiple days/weeks
2. **Calendar View** (Center) - The main calendar display area
3. **My Day Panel** (Left or Right) - Shows selected day's events in chronological order

```
┌──────────────────────────────────────────────────────────────────────────┐
│  [Multi-Day Event Ribbon - Configurable height]                         │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Vacation (May 15-22) │ Conference (May 20-21) │ Project Alpha...  │ │
│  └────────────────────────────────────────────────────────────────────┘ │
├──────────────────────────────────────────────────────────────────────────┤
│ ┌────────────┬────────────────────────────────────────────────────────┐ │
│ │  My Day    │                                                        │ │
│ │  Panel     │         Calendar View Area                            │ │
│ │            │         (Day/Week/Month/Quarter/Year/Agenda)          │ │
│ │ 8:00 AM    │                                                        │ │
│ │ ┌────────┐ │                                                        │ │
│ │ │Meeting │ │                                                        │ │
│ │ └────────┘ │                                                        │ │
│ │            │                                                        │ │
│ │ 10:30 AM   │                                                        │ │
│ │ ┌────────┐ │                                                        │ │
│ │ │Coffee  │ │                                                        │ │
│ │ └────────┘ │                                                        │ │
│ └────────────┴────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────┘
```

**Note**: My Day panel can be positioned on left or right side, or hidden entirely.

## Calendar Views

### View Types

| View | Purpose | Best For | Keyboard Shortcut |
|------|---------|----------|-------------------|
| **Day** | Single day with hourly time slots | Detailed daily planning | `Ctrl+1` |
| **Work Week** | Monday-Friday overview | Work schedule focus | `Ctrl+2` |
| **Full Week** | Complete 7-day week | Weekly planning | `Ctrl+3` |
| **Month** | Traditional monthly grid | Monthly overview | `Ctrl+4` |
| **Quarter** | 3-month business quarter view | Quarterly planning | `Ctrl+5` |
| **Year** | 12-month annual overview | Long-term planning | `Ctrl+6` |
| **Agenda** | Chronological event list | Upcoming events focus | `Ctrl+7` |

### View-Specific Features

#### Day View
```
┌─────────────────────────────────┐
│ [All-Day Events Banner]         │
├─────────────────────────────────┤
│ 6:00 AM │                       │
│ 7:00 AM │ ┌─────────────────┐  │
│ 8:00 AM │ │ Team Meeting    │  │
│ 9:00 AM │ │ 8:00-9:30 AM    │  │
│         │ └─────────────────┘  │
│10:00 AM │                       │
│11:00 AM │ ┌─────────────────┐  │
│12:00 PM │ │ Lunch Break     │  │
│         │ └─────────────────┘  │
│ 1:00 PM │                       │
└─────────────────────────────────┘
```

**Features**:
- Configurable time range (e.g., 6 AM - 11 PM)
- **Adjustable time granularity**: 15/30/60 minute intervals (default: 60 min)
- Current time indicator (red line)
- Overlapping events displayed side-by-side
- Drag to create new events (default duration: 45 minutes)
- Drag events to reschedule
- Resize events by dragging edges
- **Drag events to desktop to create countdown timer**

**Customization**:
- Time slot height (compact/normal/comfortable)
- Time interval granularity (15/30/60 minutes)
- Default event duration (configurable, default 45 min)
- Start hour of day
- End hour of day
- Show/hide all-day event banner

#### Work Week View
```
┌──────────┬──────────┬──────────┬──────────┬──────────┐
│  Monday  │ Tuesday  │Wednesday │ Thursday │  Friday  │
├──────────┼──────────┼──────────┼──────────┼──────────┤
│ 8:00 AM  │          │          │          │          │
│ 9:00 AM  │ [Event]  │          │ [Event]  │          │
│10:00 AM  │          │ [Event]  │          │          │
│11:00 AM  │          │          │          │ [Event]  │
│12:00 PM  │ [Lunch]  │ [Lunch]  │ [Lunch]  │ [Lunch]  │
│ 1:00 PM  │          │          │          │          │
│ 2:00 PM  │          │ [Event]  │          │ [Event]  │
└──────────┴──────────┴──────────┴──────────┴──────────┘
```

**Features**:
- Focus on business week (M-F)
- Weekend events optionally shown in sidebar
- Column-per-day layout
- Time slots synchronized across columns

**Customization**:
- Define which days are "work week" (configurable beyond M-F)
- Individual column widths
- Show/hide weekends in side panel

#### Full Week View
Similar to Work Week but includes Saturday and Sunday columns.

**Customization**:
- Week start day (Sunday or Monday)
- Weekend highlighting
- 7 independently resizable columns

#### Month View
```
┌────────────────────────────────────────────────┐
│              January 2025                      │
├───────┬───────┬───────┬───────┬───────┬───────┤
│  Sun  │  Mon  │  Tue  │  Wed  │  Thu  │  Fri  │
├───────┼───────┼───────┼───────┼───────┼───────┤
│       │       │       │   1   │   2   │   3   │
│       │       │       │  • •  │   •   │       │
├───────┼───────┼───────┼───────┼───────┼───────┤
│   4   │   5   │   6   │   7   │   8   │   9   │
│   •   │ • • • │   •   │       │   •   │  • •  │
├───────┼───────┼───────┼───────┼───────┼───────┤
│  ...  │  ...  │  ...  │  ...  │  ...  │  ...  │
└───────┴───────┴───────┴───────┴───────┴───────┘
```

**Features**:
- Traditional calendar grid (6 weeks × 7 days)
- Previous/next month days shown in muted color
- Event indicators (dots, lines, or counts)
- Click day to see all events
- Drag events between days
- Today highlighted

**Customization**:
- Cell height (more space = more visible events)
- Show/hide week numbers
- Show/hide adjacent month dates
- Event display style (dots, bars, numbers)

#### Quarter View
```
┌─────────────┬─────────────┬─────────────┐
│   January   │  February   │    March    │
│  S M T W T  │  S M T W T  │  S M T W T  │
│  • • • • •  │  • • • • •  │  • • • • •  │
│  • • • • •  │  • • • • •  │  • • • • •  │
└─────────────┴─────────────┴─────────────┘
```

**Features**:
- Three months side-by-side
- Q1, Q2, Q3, Q4 navigation
- Event density visualization
- Quick quarter overview

**Customization**:
- Compact or expanded month size
- Event indicator style

#### Year View
```
┌──────┬──────┬──────┬──────┐
│ Jan  │ Feb  │ Mar  │ Apr  │
├──────┼──────┼──────┼──────┤
│ May  │ Jun  │ Jul  │ Aug  │
├──────┼──────┼──────┼──────┤
│ Sep  │ Oct  │ Nov  │ Dec  │
└──────┴──────┴──────┴──────┘
```

**Features**:
- All 12 months visible
- Click month to zoom to month view
- Event density heat map
- Annual planning overview

**Customization**:
- Grid layout (3×4, 4×3, 2×6)
- Color intensity for event density

#### Agenda View
```
┌─────────────────────────────────────┐
│ Monday, January 6, 2025             │
├─────────────────────────────────────┤
│ 8:00 AM - 9:30 AM                   │
│ Team Standup                        │
│ Conference Room A                   │
├─────────────────────────────────────┤
│ 2:00 PM - 3:00 PM                   │
│ Client Call                         │
│ Virtual                             │
├─────────────────────────────────────┤
│ Tuesday, January 7, 2025            │
├─────────────────────────────────────┤
│ ...                                 │
└─────────────────────────────────────┘
```

**Features**:
- Linear chronological list
- Date headers separate days
- Event details inline
- Search and filter
- Scroll through time

**Customization**:
- Days to show ahead (7, 14, 30, 90, 365)
- Include past events
- Grouping (by day, week, month)
- Compact or detailed event display

## UI Customization

### Font Settings

**Global Font Configuration**:
```rust
pub struct FontConfig {
    pub family: String,        // "Segoe UI", "Arial", "Calibri", etc.
    pub size: u16,            // 8-72 points
    pub weight: FontWeight,   // Normal, Bold
    pub style: FontStyle,     // Normal, Italic
}
```

**Per-Element Font Settings**:
- Event titles
- Event details
- Time labels
- Date headers
- Navigation text

**Font Picker UI**:
```
┌──────────────────────────────────────┐
│ Font Family:  [Segoe UI        ▼]   │
│ Size:         [14          ▼]       │
│ Style:        [ ] Bold  [ ] Italic   │
│                                      │
│ Preview:                             │
│ ┌──────────────────────────────────┐ │
│ │ The quick brown fox jumps...     │ │
│ │ Event Title Example              │ │
│ │ 8:00 AM - 9:30 AM                │ │
│ └──────────────────────────────────┘ │
│                                      │
│ [Apply]  [Reset to Defaults]        │
└──────────────────────────────────────┘
```

### Column Width Adjustment

**How to Resize**:
1. Hover over column divider
2. Cursor changes to resize indicator (↔)
3. Click and drag left/right
4. Release to set new width
5. Double-click divider to auto-fit

**Constraints**:
- Minimum width: 50 pixels
- Maximum width: 500 pixels
- Auto-fit calculates based on content

**Persistence**:
- Widths saved per view type
- Restored on view switch
- Saved to database on change

**Reset Options**:
- "Reset This View" - Reset current view's columns
- "Reset All Views" - Reset all views to defaults
- "Auto-fit All" - Auto-fit all columns based on content

### Row Height Adjustment

**Settings**:
- **Time Slot Height** (Day/Week views):
  - Compact: 30px per slot
  - Normal: 45px per slot (default)
  - Comfortable: 60px per slot
  - Custom: User-specified height

- **Event Row Height** (Month view):
  - Minimal: Show event dots only
  - Compact: One line per event
  - Normal: Two lines per event (default)
  - Expanded: Three+ lines per event

### View Preferences

**Saved Per View**:
```rust
pub struct ViewPreferences {
    pub view_type: ViewType,
    pub time_interval: u32,       // 15, 30, 60 minutes (default: 60)
    pub default_event_duration: u32,  // minutes (default: 45)
    pub start_hour: u8,           // 0-23
    pub end_hour: u8,             // 0-23
    pub show_weekends: bool,
    pub show_week_numbers: bool,
    pub column_widths: Vec<u32>,
    pub row_height: u32,
}
```

**Global Preferences**:
```rust
pub struct GlobalPreferences {
    pub week_start_day: Weekday,  // Sunday or Monday
    pub time_format: TimeFormat,  // 12h or 24h
    pub date_format: String,      // e.g., "MM/DD/YYYY"
    pub default_view: ViewType,
    pub font_config: FontConfig,
    pub default_event_duration: u32,  // Default: 45 minutes
}
```

### Time Granularity Settings

**Time Interval Options**:
- **15 minutes**: Fine-grained scheduling, more rows
- **30 minutes**: Balanced detail and overview
- **60 minutes**: Default, clean hourly view

**Configuration UI**:
```
┌──────────────────────────────────────┐
│ Time Display Settings                │
├──────────────────────────────────────┤
│ Time Slot Interval:                  │
│   ○ 15 minutes                       │
│   ○ 30 minutes                       │
│   ● 60 minutes (Hourly)              │
│                                      │
│ Default Event Duration:              │
│   [45] minutes                       │
│   (Used when creating new events)    │
│                                      │
│ Preview:                             │
│ ┌──────────────────────────────────┐ │
│ │ 8:00 AM ─────────────────────    │ │
│ │ 9:00 AM ┌──────────────┐        │ │
│ │ 10:00AM │ New Event    │        │ │
│ │         │ (45 min)     │        │ │
│ │ 11:00AM └──────────────┘        │ │
│ └──────────────────────────────────┘ │
│                                      │
│ [Apply]  [Reset to Defaults]        │
└──────────────────────────────────────┘
```

**Impact on Views**:
- **Day/Week Views**: Changes row density
- **New Event Creation**: Uses default duration
- **Drag Operations**: Snaps to interval grid

## My Day Panel

### Overview

The My Day panel provides a focused view of the currently selected day's events in a compact, chronological list format. The panel updates automatically as the user navigates to different dates in the calendar.

### Features

**Display**:
- Chronological list of events from selected date
- Shows start time, duration, and event title
- All-day events displayed at top
- Empty state message when no events exist
- Scroll support for days with many events
- Color-coded event indicators matching calendar

**Positioning**:
- **Left side** (default)
- **Right side**
- **Hidden** (more screen space for calendar)

**Width**:
- Adjustable via drag handle on panel edge
- Default: 250px
- Minimum: 180px
- Maximum: 400px
- Width preference persisted

### Visual Layout

```
┌─────────────────────────┐
│ Thu, May 15, 2025       │
├─────────────────────────┤
│ All Day                 │
│ ● Birthday Party        │
├─────────────────────────┤
│ 8:00 AM - 9:30 AM       │
│ ● Team Standup          │
│   Conference Room A     │
├─────────────────────────┤
│ 10:00 AM - 10:30 AM     │
│ ● Coffee with Sarah     │
│   Cafe Downtown         │
├─────────────────────────┤
│ 12:00 PM - 1:00 PM      │
│ ● Lunch Break           │
├─────────────────────────┤
│ 2:00 PM - 3:00 PM       │
│ ● Project Review        │
│   Virtual (Zoom)        │
├─────────────────────────┤
│ 5:30 PM - 7:00 PM       │
│ ● Gym Session           │
│   Fitness Center        │
└─────────────────────────┘
```

### Interactions

**Event Items**:
- **Click**: Select event (highlights in calendar)
- **Double-click**: Open event editor
- **Right-click**: Context menu (Edit / Delete / Duplicate / Create Countdown)
- **Drag**: Drag to calendar or desktop to create countdown timer

**Panel Controls**:
- **Date Header**: Click to open date picker
- **Resize Handle**: Drag to adjust panel width
- **Toggle Button**: Show/hide panel (in toolbar)
- **Position Button**: Switch left/right positioning

**Selection Sync**:
- Selecting a different date in calendar updates My Day panel
- Clicking an event in My Day panel highlights it in the calendar view
- Bidirectional synchronization

### Configuration

**My Day Settings** (in Preferences):
```
┌──────────────────────────────────┐
│ My Day Panel Settings            │
├──────────────────────────────────┤
│ Position:                        │
│   ● Left side                    │
│   ○ Right side                   │
│   ○ Hidden                       │
│                                  │
│ Panel Width: [250] px            │
│   ◄────●─────────►               │
│   180        400                 │
│                                  │
│ Display Options:                 │
│   ☑ Show event location          │
│   ☑ Show event duration          │
│   ☑ Show all-day events at top   │
│   ☑ Show empty state message     │
│   ☑ Color-code by category       │
│                                  │
│ Font Size: [13] px               │
│   ◄────●─────────►               │
│   10         16                  │
│                                  │
│ [Apply]  [Cancel]  [Reset]       │
└──────────────────────────────────┘
```

### Empty State

When the selected day has no events:
```
┌─────────────────────────┐
│ Thu, May 15, 2025       │
├─────────────────────────┤
│                         │
│         📅              │
│                         │
│   No events today       │
│                         │
│  Click + to add event   │
│                         │
└─────────────────────────┘
```

## Multi-Day Event Ribbon

### Overview

The Multi-Day Event Ribbon is a horizontal strip at the top of the calendar that displays events spanning multiple days or weeks. This prevents long events from cluttering the main calendar grid.

### Purpose

**Problem**: Multi-day events (vacations, conferences, projects) can dominate calendar views, obscuring single-day events.

**Solution**: Display multi-day events in a dedicated ribbon above the main calendar, keeping the grid clean.

### Display Criteria

Events shown in ribbon if they meet **any** of these conditions:
- Span 2+ days (48+ hours)
- Marked as "all-day" and span multiple days
- Span across weekend boundaries
- User manually pins event to ribbon

### Visual Layout

```
┌────────────────────────────────────────────────────────────────────────┐
│  [Multi-Day Event Ribbon]                                              │
│  ┌─────────────────────────┬──────────────────┬────────────────────┐  │
│  │ Vacation                │ Conference       │ Project Alpha      │  │
│  │ May 15-22 (8 days)      │ May 20-21        │ May 10 - Jun 15    │  │
│  │ ████████████████████    │ ████             │ ████████████████   │  │
│  └─────────────────────────┴──────────────────┴────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
                                   ↓
┌────────────────────────────────────────────────────────────────────────┐
│                    Main Calendar View                                  │
│  (Clean, shows only single-day and short events)                       │
└────────────────────────────────────────────────────────────────────────┘
```

### Features

**Display Elements**:
- Event title
- Date range (start - end)
- Duration indicator (visual bar)
- Color coding matching event category
- Icon for event type (vacation, conference, etc.)

**Visual Indicators**:
- Progress bar showing how much of event has elapsed
- Current day marker if event is ongoing
- Countdown to start if event is upcoming

**Ribbon Modes**:
1. **Compact** (default): Single row, horizontal scroll if needed
2. **Expanded**: Multiple rows, all events visible
3. **Auto**: Expands when >3 events, compact otherwise

### Interactions

**Event Cards**:
- **Click**: Navigate calendar to event start date
- **Double-click**: Open event editor
- **Hover**: Show tooltip with full details (description, location, attendees)
- **Right-click**: Context menu (Edit / Delete / Unpin from Ribbon / Create Countdown)

**Ribbon Controls**:
- **Expand/Collapse Button**: Toggle between compact/expanded modes
- **Scroll Arrows**: Navigate when events exceed visible width
- **Settings Icon**: Configure ribbon preferences
- **Hide Button**: Temporarily hide ribbon (more vertical space)

### Configuration

**Ribbon Settings** (in Preferences):
```
┌──────────────────────────────────────────────────┐
│ Multi-Day Event Ribbon Settings                  │
├──────────────────────────────────────────────────┤
│ Display Mode:                                    │
│   ○ Compact (single row)                         │
│   ● Expanded (multiple rows)                     │
│   ○ Auto (expands when needed)                   │
│                                                  │
│ Height:                                          │
│   Compact:  [60] px                              │
│   Expanded: [120] px                             │
│                                                  │
│ Show events spanning:                            │
│   ☑ 2+ days                                      │
│   ☑ Multi-day all-day events                     │
│   ☑ Weekend-crossing events                      │
│   ☑ Manually pinned events                       │
│                                                  │
│ Minimum span to show: [2] days                   │
│   ◄────●─────────►                               │
│   1          7                                   │
│                                                  │
│ Display Options:                                 │
│   ☑ Show duration text                           │
│   ☑ Show progress bar                            │
│   ☑ Show event icons                             │
│   ☑ Show current day marker                      │
│   ☑ Color-code by category                       │
│                                                  │
│ Ribbon Position:                                 │
│   ● Top (above calendar)                         │
│   ○ Bottom (below calendar)                      │
│   ○ Hidden                                       │
│                                                  │
│ [Apply]  [Cancel]  [Reset to Defaults]           │
└──────────────────────────────────────────────────┘
```

### Examples

**Compact Mode**:
```
┌──────────────────────────────────────────────────────────────┐
│ ◀ │ Vacation (May 15-22) │ Conf (May 20-21) │ ... │ ▶ │ ⚙  │
└──────────────────────────────────────────────────────────────┘
```

**Expanded Mode**:
```
┌─────────────────────────────────────────────────────────────────┐
│ Row 1: │ Vacation ████████████████████ May 15-22 (8 days)     │
│ Row 2: │ Conference ████ May 20-21 (2 days)                    │
│ Row 3: │ Project Alpha ██████████████████ May 10 - Jun 15     │
│ Row 4: │ Training Week ██████ May 18-22 (5 days)              │
└─────────────────────────────────────────────────────────────────┘
```

**Auto Mode** (3 events, stays compact):
```
┌──────────────────────────────────────────────────────────────┐
│ Vacation (May 15-22) │ Conference │ Project Alpha           │
└──────────────────────────────────────────────────────────────┘
```

**Auto Mode** (5+ events, expands automatically):
```
┌─────────────────────────────────────────────────────────────────┐
│ Row 1: │ Vacation │ Conference │ Project Alpha                 │
│ Row 2: │ Training │ Deployment Window                          │
└─────────────────────────────────────────────────────────────────┘
```

## Drag-and-Drop System

### .ics File Import

**User Flow**:
1. Drag .ics file(s) from File Explorer
2. Hover over calendar application
3. Drop zone highlights
4. Release mouse button
5. Import preview dialog appears
6. Review events to import
7. Confirm or cancel
8. Events added to calendar

**Import Preview Dialog**:
```
┌─────────────────────────────────────────────┐
│ Import Events                               │
├─────────────────────────────────────────────┤
│ File: meetings.ics                          │
│ Found 5 events                              │
│                                             │
│ ☑ Team Meeting - Mon 8:00 AM               │
│ ☑ Lunch with Client - Mon 12:00 PM         │
│ ☐ Weekly Review - Fri 3:00 PM (Duplicate)  │
│ ☑ Project Deadline - Wed All Day           │
│ ☑ Conference Call - Thu 2:00 PM            │
│                                             │
│ [Select All] [Deselect All]                │
│ [Import Selected] [Cancel]                 │
└─────────────────────────────────────────────┘
```

**Features**:
- Multiple file drop support
- Duplicate detection
- Conflict highlighting
- Selective import
- Error handling for invalid files
- Progress bar for large files

### Event Drag Operations

**Within Calendar**:

1. **Reschedule Event** (Drag to different time):
   - Click and hold event
   - Drag to new time slot
   - Preview shows where event will land
   - Release to reschedule
   - For recurring events, prompt: "This occurrence" or "All occurrences"

2. **Move Event Date** (Drag to different day):
   - Drag event to different day column/cell
   - Time preserved
   - Date changes

3. **Adjust Duration** (Drag event edges):
   - Hover over top/bottom edge of event
   - Cursor changes to resize (↕)
   - Drag to adjust start or end time
   - Minimum 15-minute duration enforced

4. **Create Desktop Countdown Timer** (Drag to desktop):
   - Click and drag event outside calendar window
   - Drag to desktop area
   - Release to create countdown timer widget
   - Original event remains in calendar
   - New always-on-top countdown window appears

**Visual Feedback**:
- Ghost/preview of event during drag (semi-transparent)
- Target slot highlights in green (valid) or red (invalid)
- Desktop drop zone indicator when dragging outside window
- Cursor changes: move, resize, countdown icon, or forbidden
- Tooltip shows new date/time during drag

**Constraints**:
- Cannot drag events into the past
- Cannot create conflicts (prompt if conflict detected)
- Respect minimum event duration (15 minutes)
- Cannot drag across different calendars (future feature)
- Past events cannot be made into countdown timers

## Desktop Countdown Timer System

### Overview
Create persistent countdown timer widgets by dragging events from the calendar
to your desktop, via context menu, or via the event dialog. Each countdown
timer is a separate window that shows time remaining until the event starts.
Cards can be displayed as individual floating windows or grouped within
category containers.

### Creating a Countdown Timer

**Method 1: Drag to Desktop**
1. Click and hold any future event in the calendar
2. Drag event outside the calendar window
3. Move cursor to desired position on desktop
4. Release mouse button
5. Countdown timer window appears at cursor position

**Method 2: Context Menu**
1. Right-click on a future event in any calendar view
2. Select "⏱ Create Countdown"
3. If multiple categories exist, a submenu appears — pick the target container
4. Countdown window appears at default position

**Method 3: Event Dialog**
1. Create or edit an event
2. Tick "Create countdown card after saving"
3. If multiple categories exist, choose the target container from the dropdown
4. Save the event — a countdown card is created automatically

### Countdown Timer Features

**Window Properties**:
- Always-on-top (stays visible above other windows)
- Movable (drag to reposition)
- Resizable (optional)
- No taskbar entry (doesn't clutter taskbar)
- Minimal window decorations
- Semi-transparent background (optional)

**Display Elements** (Design TBD - to be specified by user):
- Event title
- Time remaining (dynamic countdown)
- Event start date/time
- Event location (if set)
- Event color/category indicator
- Close button
- "Open in Calendar" button

**Auto-Dismiss Options**:
- Dismiss when event starts (configurable)
- Dismiss when event ends
- Manual dismiss only (stay until closed)
- Flash/blink when event is imminent (5 min warning)

### Countdown Timer Data Model

```rust
pub struct CountdownTimer {
    pub id: i64,
    pub event_id: i64,           // Reference to event
    pub position_x: i32,         // Desktop X coordinate
    pub position_y: i32,         // Desktop Y coordinate
    pub width: u32,              // Window width (default: 300px)
    pub height: u32,             // Window height (default: 150px)
    pub auto_dismiss: bool,      // Auto-close when event starts
    pub created_at: DateTime<Local>,
}
```

### Countdown Display Format

**Time Remaining Formats**:
- **Days away**: "5 days, 3 hours"
- **Hours away**: "3 hours, 45 minutes"
- **Minutes away**: "45 minutes, 30 seconds"
- **Seconds away**: "30 seconds"
- **Imminent**: "Starting now!" (flash/blink)
- **Past**: "Started 5 minutes ago" (if not dismissed)

### Countdown Timer Lifecycle

```
Create → Active → Warning → Started → Dismissed
```

**States**:
1. **Created**: Timer window opens, countdown begins
2. **Active**: Normal countdown display, updates every second
3. **Warning**: < 5 minutes remaining, visual indicator (flash, color change)
4. **Started**: Event start time reached, notification option
5. **Dismissed**: User closes or auto-dismiss triggers, window removed

### Persistence and Restoration

**On Application Close**:
- Save all active countdown timer positions
- Save window dimensions
- Save countdown timer settings

**On Application Startup**:
- Restore all countdown timers
- Update countdown times
- Remove timers for past events (if auto-dismiss enabled)
- Restore window positions (adjust if off-screen)

**Multiple Timers**:
- Support unlimited simultaneous countdown timers
- Smart positioning (avoid overlapping new timers)
- Manager tracks all active timers

### Interaction with Main Calendar

**Bidirectional Link**:
- Countdown timer updates when event changes in calendar
- Click countdown timer to open event in calendar
- Delete event in calendar → countdown timer removed
- Reschedule event → countdown updates immediately

**Visual Indicators**:
- Events with active countdown timers marked in calendar
- Icon or badge on event card
- "Has countdown" filter option

### Countdown Timer Service

```rust
pub struct CountdownService {
    db: Database,
    active_timers: HashMap<i64, CountdownWindow>,
}

impl CountdownService {
    /// Create a new countdown timer for an event
    pub fn create_countdown(&mut self, event: &Event, position: (i32, i32)) -> Result<i64>;
    
    /// Remove a countdown timer
    pub fn dismiss_countdown(&mut self, timer_id: i64) -> Result<()>;
    
    /// Update countdown displays (called every second)
    pub fn update_all_countdowns(&mut self);
    
    /// Get all active countdown timers
    pub fn get_active_timers(&self) -> Vec<&CountdownTimer>;
    
    /// Restore timers from database on startup
    pub fn restore_timers(&mut self) -> Result<()>;
    
    /// Handle event update (refresh countdown if exists)
    pub fn on_event_updated(&mut self, event_id: i64);
    
    /// Handle event deletion (remove countdown)
    pub fn on_event_deleted(&mut self, event_id: i64);
}
```

### Settings and Configuration

**Countdown Preferences**:
```rust
pub struct CountdownPreferences {
    pub auto_dismiss_on_start: bool,    // Default: true
    pub show_seconds: bool,              // Show seconds in countdown
    pub warning_minutes: u32,            // Warning threshold (default: 5)
    pub flash_warning: bool,             // Flash window when imminent
    pub play_sound_on_start: bool,      // Audio notification
    pub default_width: u32,              // Default window width
    pub default_height: u32,             // Default window height
    pub opacity: f32,                    // Window opacity (0.0-1.0)
    pub position_mode: PositionMode,    // Smart, Manual, Remember
}

pub enum PositionMode {
    Smart,      // Auto-position to avoid overlap
    Manual,     // Always at cursor position
    Remember,   // Remember last position per event
}
```

### Use Cases

**Example Scenarios**:
1. **Important Meeting**: Drag meeting to desktop, countdown reminds you as meeting approaches
2. **Project Deadline**: Keep deadline countdown visible while working
3. **Multiple Meetings**: Create countdown for each meeting, arrange on screen
4. **Daily Standup**: Recurring event countdown, always visible in morning
5. **Break Timer**: Drag lunch break to desktop, countdown to break time

### Category Containers

Cards can be organised into named categories. Each category is rendered as a
collapsible container with its own header bar.

**Container Header Bar**:
- ▶/▼ collapse/expand toggle
- Category name
- Card count badge (e.g. `(3)`)
- Sort mode button: 📅 Date or ✋ Manual
- ➕ quick-add button

**Display Modes** (via View → Countdown Cards → Display mode):
- **Individual Windows** — each card is a separate always-on-top window
- **Combined Container** — all cards in one resizable window
- **Category Containers** — cards grouped by category with individual containers

**Category Management** (Edit → Manage Countdown Categories…):
- Create, rename, reorder, and delete categories
- Configure per-container card defaults:
  - Template dropdown — select a card template or "Global defaults"
  - Layout orientation — Auto, Portrait, or Landscape
  - Default card width/height (60–400 px)
  - Read-only template preview (colours, fonts, dimensions)
- The default "General" category cannot be deleted or renamed

**Card Template Management** (Edit → Manage Card Templates…):
- Create, edit, and delete reusable card visual templates
- Each template defines: colours (title BG/FG, body BG, days FG), font sizes
  (title 10–48 pt, days 32–220 pt), and default card dimensions
- The built-in "Default" template cannot be deleted

**Four-Tier Visual Inheritance**:
1. Global defaults — base settings
2. Template — reusable colours, fonts, dimensions (selected per-category)
3. Category — overrides card dimensions; legacy inline visuals for migration
4. Per-card overrides — individual card settings

**Cross-Container Drag-and-Drop**:
- Drag a card from one container and drop it onto another container's header
- The card is re-assigned to the target category

### Visual Design

Cards use a two-panel layout: a coloured title bar (event title) and a body area
(countdown number). All colours and font sizes are configurable via four-tier
inheritance (Global → Template → Category → Card).

**Warning states** provide visual feedback as an event approaches:
- Normal — steady display with standard colours
- Warning — colour change, pulsing animation (configurable threshold)
- Imminent — flashing, "Starting now!" text

## Keyboard Shortcuts

### View Navigation
- `Ctrl+1` - Day view
- `Ctrl+2` - Work week view
- `Ctrl+3` - Full week view
- `Ctrl+4` - Month view
- `Ctrl+5` - Quarter view
- `Ctrl+6` - Year view
- `Ctrl+7` - Agenda view

### Time Navigation
- `→` or `.` - Next period (day/week/month depending on view)
- `←` or `,` - Previous period
- `T` - Go to today
- `PageUp` - Previous year
- `PageDown` - Next year

### Event Operations
- `Ctrl+N` - New event
- `Ctrl+E` - Edit selected event
- `Delete` - Delete selected event
- `Ctrl+C` - Copy selected event
- `Ctrl+V` - Paste event
- `Enter` - Open event details

### View Controls
- `Ctrl++` - Zoom in (increase row height)
- `Ctrl+-` - Zoom out (decrease row height)
- `Ctrl+0` - Reset zoom
- `F11` - Toggle fullscreen

### Application
- `Ctrl+F` - Search events
- `Ctrl+,` - Open settings
- `Ctrl+Q` - Quit application
- `F1` - Help

## Preferences Persistence

### Storage Location
- **Windows**: `%APPDATA%\RustCalendar\preferences.db`
- SQLite database containing all preferences

### Saved Preferences
- Current view type
- View-specific settings (column widths, row heights)
- Font configurations
- Theme selection
- Window size and position
- Last viewed date
- Search history
- Recent color picks

### Save Strategy
- **Auto-save**: Every 30 seconds if changes detected
- **On change**: Immediate save for critical preferences (debounced)
- **On close**: Final save on application exit
- **On view switch**: Save current view state

### Reset Options
- **Reset Current View** - Reset active view to defaults
- **Reset All Views** - Reset all view preferences
- **Reset Fonts** - Reset to system default fonts
- **Reset Window** - Reset window size/position
- **Factory Reset** - Reset everything to defaults (prompt for confirmation)

## Accessibility Features

### High Contrast Support
- Respect Windows high contrast themes
- Adjust colors automatically
- Maintain readability in all contrast modes

### Keyboard Navigation
- Full keyboard navigation support
- Tab through all interactive elements
- Arrow key navigation in calendars
- Escape to close dialogs

### Screen Reader Support
- ARIA labels on all UI elements
- Announcements for view changes
- Event details readable by screen readers

### Scalability
- Support for 100%-300% DPI scaling
- Adjustable font sizes (8-72pt)
- Minimum touch target size (44×44 pixels)
- Zoomable interface

## Performance Considerations

### Rendering Optimization
- Virtual scrolling for agenda view (render visible items only)
- Lazy loading of event details
- Cached rendered elements
- Debounced resize events (300ms)

### Memory Management
- Load only visible date range events
- Unload events outside view window
- Limit cached font renders
- Periodic cleanup of unused resources

### Responsive Design
- Smooth animations (60 FPS target)
- Non-blocking UI operations
- Background thread for heavy calculations
- Progressive rendering for large datasets

---

**Note**: This UI system is designed to be intuitive, customizable, and performant, providing users with complete control over their calendar viewing experience.
