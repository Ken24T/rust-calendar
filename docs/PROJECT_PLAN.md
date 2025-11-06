# Rust Calendar Application - Project Plan

## Project Overview
A native Windows desktop calendar application built with Rust, featuring a modern GUI with theming support, event management with single and repeating events, and configurable reminders.

## Core Features

### 1. User Interface
- **GUI Framework**: Use `iced` or `slint` for native Windows GUI
  - `iced` - Pure Rust, cross-platform, modern reactive UI
  - `slint` - Declarative UI with excellent performance
- **Main Layout Components**:
  - **My Day Panel** - Sidebar showing selected day's events
    - Configurable position (left/right/hidden)
    - Adjustable width (180-400px, default 250px)
    - Chronological event list with times
    - Auto-updates when calendar date changes
    - Drag events from panel to calendar or desktop
  - **Multi-Day Event Ribbon** - Top banner for multi-day events
    - Shows events spanning 2+ days
    - Configurable modes (compact/expanded/auto)
    - Progress indicators for ongoing events
    - Adjustable height and position (top/bottom)
    - Prevents long events from cluttering main calendar
  - **Calendar View Area** - Main calendar display
- **Multiple Calendar Views**:
  - Day view - Single day detailed schedule
  - Work week view - Monday through Friday
  - Full week view - Sunday through Saturday
  - Month view - Traditional monthly calendar grid
  - Quarter view - 3-month overview
  - Year view - 12-month annual overview
  - Agenda view - Linear list of upcoming events
  - Quick view switching with keyboard shortcuts
- **UI Customization**:
  - Adjustable column widths (drag to resize)
  - Configurable font family, size, and style
  - Adjustable row heights
  - **Configurable time granularity**: 15/30/60 minute blocks (default: 60 min)
  - **Default event duration**: Configurable (default: 45 minutes)
  - Week start day preference (Sunday/Monday)
  - All UI preferences persisted between sessions
- **Drag and Drop Support**:
  - Drag .ics files onto calendar to import events
  - Drag events between time slots to reschedule
  - Drag event edges to adjust duration
  - **Drag (tear) events to desktop to create countdown timer widgets**
  - **Drag events from My Day panel to calendar or desktop**
  - Visual feedback during drag operations
- **Desktop Countdown Timers**:
  - Tear events from calendar to desktop
  - Live countdown timer widget on desktop
  - Always-on-top, movable window
  - Original event remains in calendar
  - Customizable countdown display (design TBD)
  - Auto-dismiss when event starts (optional)
- **Theme Support**:
  - Light and Dark mode
  - Customizable color schemes
  - Theme persistence across sessions
  - Smooth theme transitions

### 2. Event Management
- **Single Events**:
  - Title, description, location
  - Start and end date/time
  - All-day event support
  - Color coding/categorization
  - Attachments/notes
  
- **Repeating Events**:
  - Daily, Weekly, Fortnightly, Monthly, Quarterly, Yearly recurrence
  - Custom recurrence patterns (e.g., "every 2nd Tuesday")
  - Recurrence end date or occurrence count
  - Exception handling (skip specific occurrences)
  - Edit single occurrence or entire series
  - Fortnightly: Every 2 weeks on same day(s)
  - Quarterly: Every 3 months (business quarter aligned)

### 3. Reminders
- **Configurable Reminder System**:
  - Multiple reminders per event
  - Time-based reminders (minutes/hours/days before)
  - Custom reminder times
  - Windows notification integration
  - Snooze functionality
  - Audio alerts (optional)

### 4. Data Storage
- **Local Database**: SQLite for persistent storage
  - Events table with recurrence rules
  - Reminders table
  - Settings/preferences table
  - Theme configurations
  - UI preferences (view settings, column widths, fonts)
- **Data Export/Import**: 
  - iCalendar (.ics) format support
  - Drag-and-drop .ics file import
  - Batch import from multiple .ics files
  - Export selected events or date ranges

## Technical Architecture

### Design Principles

**Modularity First**
- Each file should have a single, clear responsibility
- Maximum file size: ~200-300 lines of code
- Split large modules into smaller, focused submodules
- Use Rust's module system to organize related functionality
- Prefer composition over large inheritance hierarchies

**Comprehensive Testing**
- Every module must have corresponding tests
- Test files mirror source structure (1:1 mapping)
- Unit tests for all business logic
- Integration tests for component interactions
- Property-based testing for complex logic (recurrence rules)

### Project Structure
```
rust-calendar/
├── src/
│   ├── main.rs                 # Application entry point (~50 lines)
│   ├── app.rs                  # Main app coordinator (~150 lines)
│   │
│   ├── ui/
│   │   ├── mod.rs              # UI module exports
│   │   ├── state.rs            # Application state management
│   │   ├── messages.rs         # UI message types
│   │   │
│   │   ├── views/
│   │   │   ├── mod.rs          # View exports
│   │   │   ├── calendar/
│   │   │   │   ├── mod.rs      # Calendar view module
│   │   │   │   ├── day_view.rs      # Single day detailed view
│   │   │   │   ├── work_week_view.rs # Monday-Friday view
│   │   │   │   ├── week_view.rs     # Full 7-day week view
│   │   │   │   ├── month_view.rs    # Monthly grid display
│   │   │   │   ├── quarter_view.rs  # 3-month overview
│   │   │   │   ├── year_view.rs     # 12-month annual view
│   │   │   │   ├── agenda_view.rs   # Linear list view
│   │   │   │   ├── navigation.rs    # Calendar navigation controls
│   │   │   │   └── view_switcher.rs # Switch between views
│   │   │   ├── event/
│   │   │   │   ├── mod.rs      # Event view module
│   │   │   │   ├── form.rs     # Event creation/edit form
│   │   │   │   ├── details.rs  # Event details display
│   │   │   │   ├── list.rs     # Event list view
│   │   │   │   └── drag_handler.rs  # Drag-and-drop logic
│   │   │   ├── settings/
│   │   │   │   ├── mod.rs      # Settings view module
│   │   │   │   ├── general.rs  # General settings
│   │   │   │   ├── themes.rs   # Theme preferences
│   │   │   │   ├── view_prefs.rs    # View preferences
│   │   │   │   ├── font_settings.rs # Font customization
│   │   │   │   └── notifications.rs # Notification settings
│   │   │   └── reminder/
│   │   │       ├── mod.rs      # Reminder view module
│   │   │       ├── list.rs     # Reminder list
│   │   │       └── config.rs   # Reminder configuration
│   │   │
│   │   ├── components/
│   │   │   ├── mod.rs          # Component exports
│   │   │   ├── date_picker.rs  # Date selection widget
│   │   │   ├── time_picker.rs  # Time selection widget
│   │   │   ├── event_card.rs   # Event display component
│   │   │   ├── theme_selector.rs    # Theme switching UI
│   │   │   ├── recurrence_picker.rs # Recurrence pattern selector
│   │   │   ├── color_picker.rs      # Color selection widget
│   │   │   ├── resizable_column.rs  # Adjustable column widths
│   │   │   ├── font_picker.rs       # Font selection widget
│   │   │   ├── drop_zone.rs         # Drag-and-drop target
│   │   │   ├── countdown_timer.rs   # Desktop countdown widget
│   │   │   ├── my_day_panel.rs      # My Day sidebar panel
│   │   │   └── ribbon.rs            # Multi-day event ribbon
│   │   │
│   │   └── theme/
│   │       ├── mod.rs          # Theme module
│   │       ├── types.rs        # Theme type definitions
│   │       ├── loader.rs       # Theme file loading
│   │       └── applier.rs      # Theme application logic
│   │
│   ├── models/
│   │   ├── mod.rs              # Model exports
│   │   ├── event/
│   │   │   ├── mod.rs          # Event module
│   │   │   ├── event.rs        # Core event structure
│   │   │   ├── builder.rs      # Event builder pattern
│   │   │   └── validator.rs    # Event validation
│   │   ├── recurrence/
│   │   │   ├── mod.rs          # Recurrence module
│   │   │   ├── rule.rs         # Recurrence rule structure
│   │   │   ├── frequency.rs    # Frequency types
│   │   │   ├── calculator.rs   # Occurrence calculation
│   │   │   ├── exceptions.rs   # Exception handling
│   │   │   └── patterns.rs     # Common recurrence patterns
│   │   ├── reminder/
│   │   │   ├── mod.rs          # Reminder module
│   │   │   ├── reminder.rs     # Reminder structure
│   │   │   └── trigger.rs      # Reminder trigger logic
│   │   ├── settings/
│   │   │   ├── mod.rs          # Settings module
│   │   │   ├── settings.rs     # Settings structure
│   │   │   ├── defaults.rs     # Default settings
│   │   │   └── ui_preferences.rs    # UI preferences structure
│   │   └── ui/
│   │       ├── mod.rs          # UI models
│   │       ├── view_config.rs  # View configuration
│   │       ├── font_config.rs  # Font settings
│   │       ├── layout_config.rs # Column widths, row heights
│   │       ├── countdown_config.rs  # Countdown timer settings
│   │       ├── my_day_config.rs     # My Day panel preferences
│   │       └── ribbon_config.rs     # Multi-day ribbon settings
│   │
│   ├── services/
│   │   ├── mod.rs              # Service exports
│   │   ├── database/
│   │   │   ├── mod.rs          # Database module
│   │   │   ├── connection.rs   # Connection management
│   │   │   ├── migrations.rs   # Schema migrations
│   │   │   ├── events_repo.rs  # Event repository
│   │   │   ├── reminders_repo.rs    # Reminder repository
│   │   │   ├── settings_repo.rs     # Settings repository
│   │   │   └── ui_prefs_repo.rs     # UI preferences repository
│   │   ├── event/
│   │   │   ├── mod.rs          # Event service module
│   │   │   ├── crud.rs         # CRUD operations
│   │   │   ├── query.rs        # Event queries
│   │   │   ├── recurrence_handler.rs  # Recurrence logic
│   │   │   └── conflict_detector.rs   # Event conflict detection
│   │   ├── reminder/
│   │   │   ├── mod.rs          # Reminder service module
│   │   │   ├── scheduler.rs    # Reminder scheduling
│   │   │   ├── checker.rs      # Background reminder checker
│   │   │   └── snooze.rs       # Snooze handling
│   │   ├── notification/
│   │   │   ├── mod.rs          # Notification module
│   │   │   ├── windows.rs      # Windows notifications
│   │   │   └── manager.rs      # Notification manager
│   │   ├── ical/
│   │   │   ├── mod.rs          # iCalendar module
│   │   │   ├── importer.rs     # iCal import
│   │   │   ├── exporter.rs     # iCal export
│   │   │   ├── parser.rs       # iCal parsing
│   │   │   └── drag_drop_handler.rs # Handle dropped .ics files
│   │   ├── preferences/
│   │   │   ├── mod.rs          # Preferences service
│   │   │   ├── persistence.rs  # Save/load preferences
│   │   │   └── defaults.rs     # Default preference values
│   │   └── countdown/
│   │       ├── mod.rs          # Countdown timer service
│   │       ├── manager.rs      # Manage countdown windows
│   │       └── window.rs       # Countdown window creation
│   │
│   └── utils/
│       ├── mod.rs              # Utility exports
│       ├── date/
│       │   ├── mod.rs          # Date utilities module
│       │   ├── formatting.rs   # Date formatting
│       │   ├── parsing.rs      # Date parsing
│       │   └── calculations.rs # Date calculations
│       ├── validation/
│       │   ├── mod.rs          # Validation module
│       │   ├── event.rs        # Event validation
│       │   └── input.rs        # Input validation
│       └── error.rs            # Error types
│
├── tests/
│   ├── unit/                   # Unit tests (mirror src structure)
│   │   ├── models/
│   │   │   ├── event_tests.rs
│   │   │   ├── recurrence_tests.rs
│   │   │   └── reminder_tests.rs
│   │   ├── services/
│   │   │   ├── event_service_tests.rs
│   │   │   ├── reminder_service_tests.rs
│   │   │   └── database_tests.rs
│   │   └── utils/
│   │       ├── date_utils_tests.rs
│   │       └── validation_tests.rs
│   ├── integration/
│   │   ├── event_workflow_tests.rs     # End-to-end event tests
│   │   ├── reminder_workflow_tests.rs  # End-to-end reminder tests
│   │   ├── database_integration_tests.rs
│   │   └── ical_integration_tests.rs
│   ├── property/               # Property-based tests
│   │   ├── recurrence_properties.rs    # Recurrence edge cases
│   │   └── date_properties.rs          # Date calculation properties
│   └── fixtures/
│       ├── mod.rs              # Test fixtures
│       ├── events.rs           # Sample events
│       └── ical_files/         # Sample .ics files
│
├── benches/                    # Benchmarks
│   ├── recurrence_bench.rs     # Recurrence performance
│   └── database_bench.rs       # Database query performance
│
├── assets/
│   ├── themes/
│   │   ├── light.toml          # Light theme definition
│   │   ├── dark.toml           # Dark theme definition
│   │   └── custom/             # User custom themes
│   └── icons/
│       └── app.ico             # Application icon
│
├── docs/
│   ├── PROJECT_PLAN.md         # This file
│   ├── ARCHITECTURE.md         # Detailed architecture docs
│   ├── TESTING.md              # Testing guidelines
│   ├── MODULARITY.md           # Modularity guidelines
│   └── USER_GUIDE.md           # End-user documentation
│
├── Cargo.toml                  # Dependencies and metadata
├── README.md                   # Project overview
└── .gitignore                  # Git ignore rules
```

### Key Dependencies (Cargo.toml)

```toml
[dependencies]
# GUI Framework (choose one)
iced = "0.12"                    # Modern reactive UI
# OR
slint = "1.4"                    # Declarative UI framework

# Database
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Date/Time handling
chrono = "0.4"
chrono-tz = "0.8"

# Recurrence rules
rrule = "0.11"                   # RFC 5545 recurrence rules

# Windows notifications
windows = { version = "0.54", features = ["Win32_UI_Notifications"] }
notify-rust = "4.10"             # Cross-platform notifications

# iCalendar support
ical = "0.10"

# Configuration
toml = "0.8"
directories = "5.0"              # Standard directories for config/data

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Async runtime
tokio = { version = "1.36", features = ["full"] }

# Logging
log = "0.4"
env_logger = "0.11"

[dev-dependencies]
# Testing
mockall = "0.12"                 # Mocking framework
tempfile = "3.10"                # Temporary files for tests
proptest = "1.4"                 # Property-based testing
test-case = "3.3"                # Parameterized tests

# Benchmarking
criterion = "0.5"                # Benchmarking framework

# Test utilities
pretty_assertions = "1.4"        # Better assertion output
serial_test = "3.0"              # Sequential test execution
```

## Implementation Phases

### Phase 1: Foundation (Weeks 1-2)
- [ ] Set up project structure and dependencies
- [ ] Initialize SQLite database schema
- [ ] Create basic data models (Event, Reminder, Settings)
- [ ] Implement date/time utilities
- [ ] Set up logging and error handling

### Phase 2: Core Event Management (Weeks 3-4)
- [ ] Implement event CRUD operations (modular: create.rs, read.rs, update.rs, delete.rs)
- [ ] Build recurrence rule engine with frequency types module
- [ ] Implement fortnightly recurrence patterns with comprehensive tests
- [ ] Implement quarterly recurrence patterns with comprehensive tests
- [ ] Create event service layer (split into focused modules)
- [ ] Write unit tests for all event logic modules (1:1 test coverage)
- [ ] Write property-based tests for recurrence edge cases
- [ ] Implement iCalendar import/export (separate modules for import/export)

### Phase 3: Basic UI (Weeks 5-6)
- [ ] Set up GUI framework
- [ ] Create main application window with menu bar
- [ ] Implement calendar view system architecture
- [ ] Build month view (initial default view)
- [ ] Add view switcher component (day/week/month/quarter/year/agenda)
- [ ] Implement basic navigation (next/previous period)
- [ ] Build event creation/editing forms
- [ ] Add resizable column support
- [ ] Implement basic drag-and-drop for event rescheduling

### Phase 4: Reminder System (Week 7)
- [ ] Implement reminder scheduling
- [ ] Integrate Windows notifications
- [ ] Add reminder service background worker
- [ ] Create reminder UI components
- [ ] Test notification delivery

### Phase 5: Theming (Week 8)
- [ ] Design theme system architecture
- [ ] Create light and dark themes
- [ ] Implement theme switching logic
- [ ] Add theme persistence
- [ ] Build theme customization UI

### Phase 6: Advanced Features (Weeks 9-10)
- [ ] Implement all remaining calendar views:
  - [ ] Day view with hourly time slots
  - [ ] Work week view (Monday-Friday)
  - [ ] Full week view (7 days)
  - [ ] Quarter view (3-month overview)
  - [ ] Year view (12-month grid)
  - [ ] Agenda view (linear event list)
- [ ] Add drag-and-drop .ics file import
- [ ] Implement font customization UI
- [ ] Add column width persistence
- [ ] Implement time granularity settings (15/30/60 min)
- [ ] Add default event duration setting
- [ ] **Implement desktop countdown timer feature**:
  - [ ] Drag event to desktop to create countdown
  - [ ] Always-on-top countdown window
  - [ ] Live countdown display
  - [ ] Window positioning and persistence
- [ ] Implement search functionality
- [ ] Add event categories/colors
- [ ] Create settings panel with all UI preferences
- [ ] Implement data backup/restore
- [ ] Add keyboard shortcuts for view switching

### Phase 7: Polish & Testing (Weeks 11-12)
- [ ] Run comprehensive test suite (unit, integration, property-based)
- [ ] Ensure >90% code coverage across all modules
- [ ] Performance optimization guided by benchmarks
- [ ] Code review for modularity (ensure no file >300 lines)
- [ ] UI/UX refinements
- [ ] Documentation completion (doc comments for all public APIs)
- [ ] User documentation with examples
- [ ] Windows installer creation

## Database Schema

### Events Table
```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    description TEXT,
    location TEXT,
    start_datetime TEXT NOT NULL,  -- ISO 8601 format
    end_datetime TEXT NOT NULL,
    is_all_day BOOLEAN NOT NULL DEFAULT 0,
    category TEXT,
    color TEXT,
    recurrence_rule TEXT,          -- RRULE string (RFC 5545)
    recurrence_exceptions TEXT,    -- JSON array of exception dates
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### Reminders Table
```sql
CREATE TABLE reminders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    minutes_before INTEGER NOT NULL,
    custom_time TEXT,              -- Optional custom reminder time
    is_enabled BOOLEAN NOT NULL DEFAULT 1,
    last_triggered TEXT,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
);
```

### Settings Table
```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### UI Preferences Table
```sql
CREATE TABLE ui_preferences (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- Singleton table
    current_view TEXT NOT NULL DEFAULT 'month',  -- day, work_week, week, month, quarter, year, agenda
    week_start_day INTEGER NOT NULL DEFAULT 0,   -- 0=Sunday, 1=Monday
    time_interval INTEGER NOT NULL DEFAULT 60,   -- minutes per grid slot (15/30/60)
    default_event_duration INTEGER NOT NULL DEFAULT 45,  -- default duration for new events (minutes)
    font_family TEXT NOT NULL DEFAULT 'Segoe UI',
    font_size INTEGER NOT NULL DEFAULT 14,
    font_weight TEXT NOT NULL DEFAULT 'normal',  -- normal, bold
    font_style TEXT NOT NULL DEFAULT 'normal',   -- normal, italic
    show_weekends BOOLEAN NOT NULL DEFAULT 1,
    show_week_numbers BOOLEAN NOT NULL DEFAULT 0,
    -- My Day panel
    my_day_visible BOOLEAN NOT NULL DEFAULT 1,
    my_day_position TEXT NOT NULL DEFAULT 'left',  -- left, right, hidden
    my_day_width INTEGER NOT NULL DEFAULT 250,     -- pixels
    my_day_show_location BOOLEAN NOT NULL DEFAULT 1,
    my_day_show_duration BOOLEAN NOT NULL DEFAULT 1,
    my_day_font_size INTEGER NOT NULL DEFAULT 13,
    -- Multi-day ribbon
    ribbon_visible BOOLEAN NOT NULL DEFAULT 1,
    ribbon_mode TEXT NOT NULL DEFAULT 'auto',      -- compact, expanded, auto
    ribbon_position TEXT NOT NULL DEFAULT 'top',   -- top, bottom, hidden
    ribbon_compact_height INTEGER NOT NULL DEFAULT 60,
    ribbon_expanded_height INTEGER NOT NULL DEFAULT 120,
    ribbon_min_days INTEGER NOT NULL DEFAULT 2,    -- minimum days to show in ribbon
    ribbon_show_progress BOOLEAN NOT NULL DEFAULT 1,
    ribbon_show_icons BOOLEAN NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL
);

CREATE TABLE column_widths (
    view_type TEXT NOT NULL,        -- day, work_week, week, etc.
    column_index INTEGER NOT NULL,
    width_pixels INTEGER NOT NULL,
    PRIMARY KEY (view_type, column_index)
);

CREATE TABLE row_heights (
    view_type TEXT NOT NULL,        -- day, work_week, week, etc.
    height_pixels INTEGER NOT NULL,
    PRIMARY KEY (view_type)
);

CREATE TABLE countdown_timers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    position_x INTEGER NOT NULL,    -- Desktop position
    position_y INTEGER NOT NULL,
    width INTEGER NOT NULL DEFAULT 300,
    height INTEGER NOT NULL DEFAULT 150,
    auto_dismiss BOOLEAN NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
);
```

## Calendar View System

### View Types and Implementations

#### 1. Day View (`day_view.rs`)
- **Purpose**: Detailed schedule for a single day
- **Layout**: Vertical time slots (e.g., 6 AM - 11 PM)
- **Features**:
  - Hourly/half-hourly time divisions
  - All-day events banner at top
  - Overlapping events displayed side-by-side
  - Current time indicator line
  - Drag to create new events
  - Drag events to reschedule
  - Resize events to adjust duration
- **Customization**: 
  - Adjustable time slot height
  - Start/end time of day range
  - Time interval (15/30/60 minutes)

#### 2. Work Week View (`work_week_view.rs`)
- **Purpose**: Monday through Friday overview
- **Layout**: 5 columns (one per weekday) with time slots
- **Features**:
  - Similar to day view but 5 columns
  - Compact view of work week
  - Cross-day event visualization
  - Weekend events hidden (or shown in separate section)
- **Customization**:
  - Column width adjustable
  - Which days constitute "work week" (configurable)

#### 3. Full Week View (`week_view.rs`)
- **Purpose**: Complete 7-day week overview
- **Layout**: 7 columns (Sunday-Saturday or Monday-Sunday)
- **Features**:
  - All days of week visible
  - Time-based vertical slots
  - Weekend days optionally highlighted
  - Week number display
- **Customization**:
  - Week start day (Sunday/Monday)
  - Column widths independently adjustable
  - Weekend highlighting color

#### 4. Month View (`month_view.rs`)
- **Purpose**: Traditional monthly calendar grid
- **Layout**: 6 weeks × 7 days grid
- **Features**:
  - Previous/next month days shown in muted color
  - Multiple events per day shown as list
  - Today highlighted
  - Event dots/indicators when too many to display
  - Click day to see all events
  - Drag events between days
- **Customization**:
  - Show/hide week numbers
  - Show/hide events from adjacent months
  - Adjustable cell height

#### 5. Quarter View (`quarter_view.rs`)
- **Purpose**: 3-month overview (one business quarter)
- **Layout**: 3 mini-month grids side by side
- **Features**:
  - Q1 (Jan-Mar), Q2 (Apr-Jun), Q3 (Jul-Sep), Q4 (Oct-Dec)
  - Simplified event display (dots/counts)
  - Quick navigation between quarters
  - Overview for planning
- **Customization**:
  - Compact or expanded month layouts
  - Event indicator style (dots, numbers, bars)

#### 6. Year View (`year_view.rs`)
- **Purpose**: Full annual overview
- **Layout**: 12 mini-month grids (3×4 or 4×3 layout)
- **Features**:
  - High-level view of entire year
  - Event density indicators
  - Quick year-at-a-glance
  - Click month to zoom to month view
  - Holidays and important dates highlighted
- **Customization**:
  - Grid layout (3×4, 4×3, 2×6)
  - Event density visualization style

#### 7. Agenda View (`agenda_view.rs`)
- **Purpose**: Linear list of upcoming events
- **Layout**: Scrollable list grouped by date
- **Features**:
  - Chronological event listing
  - Date headers separating days
  - Past events optionally shown
  - Search/filter capabilities
  - Event details inline
  - No time slot visualization
- **Customization**:
  - Days ahead to show (7, 14, 30, 90 days)
  - Include past events toggle
  - Grouping options (by day, week, month)

### View Switching

**Navigation Methods**:
- Toolbar buttons with icons
- Dropdown menu selector
- Keyboard shortcuts:
  - `Ctrl+1` - Day view
  - `Ctrl+2` - Work week view
  - `Ctrl+3` - Full week view
  - `Ctrl+4` - Month view
  - `Ctrl+5` - Quarter view
  - `Ctrl+6` - Year view
  - `Ctrl+7` - Agenda view
- View menu in menu bar
- Right-click context menu

**State Preservation**:
- Current view saved to database
- Selected date maintained across view switches
- Zoom level/scale maintained per view

## UI Customization System

### Font Customization

**Settings Structure** (`font_config.rs`):
```rust
pub struct FontConfig {
    pub family: String,           // e.g., "Segoe UI", "Arial", "Calibri"
    pub size: u16,                // Points (8-72)
    pub weight: FontWeight,       // Normal, Bold
    pub style: FontStyle,         // Normal, Italic
}

pub enum FontWeight {
    Normal,
    Bold,
}

pub enum FontStyle {
    Normal,
    Italic,
}
```

**Separate Font Settings**:
- Event titles font
- Event details font
- Time labels font
- Date headers font
- Navigation font

**Font Picker UI**:
- System font enumeration
- Live preview
- Size slider with text input
- Style toggles (Bold, Italic)

### Column Width Customization

**Implementation** (`resizable_column.rs`):
- Hover over column divider shows resize cursor
- Click and drag to resize
- Double-click divider to auto-fit content
- Minimum width constraint (e.g., 50px)
- Maximum width constraint (e.g., 500px)
- Widths saved per view type

**Storage**:
```rust
pub struct ColumnLayout {
    pub view_type: ViewType,
    pub column_widths: Vec<u32>,  // Width in pixels for each column
}
```

**Reset Options**:
- Reset to defaults per view
- Reset all views to defaults
- Auto-fit all columns

### Row Height Customization

**Settings**:
- Time slot height (day/week views)
- Event row height (month view)
- Minimum height to show event details
- Compact/Normal/Comfortable presets

### Preferences Persistence

**Save Triggers**:
- On change (with debouncing)
- On application close
- Manual save button
- Auto-save every 30 seconds if changes exist

**Preferences Service** (`preferences/persistence.rs`):
```rust
pub struct PreferencesService {
    db: Database,
}

impl PreferencesService {
    pub fn load_ui_preferences(&self) -> Result<UiPreferences>;
    pub fn save_ui_preferences(&self, prefs: &UiPreferences) -> Result<()>;
    pub fn load_column_widths(&self, view: ViewType) -> Result<Vec<u32>>;
    pub fn save_column_widths(&self, view: ViewType, widths: &[u32]) -> Result<()>;
    pub fn load_font_config(&self) -> Result<FontConfig>;
    pub fn save_font_config(&self, config: &FontConfig) -> Result<()>;
    pub fn reset_to_defaults(&self) -> Result<()>;
}
```

## Drag-and-Drop System

### .ics File Import

**Drop Zone Component** (`drop_zone.rs`):
- Accept .ics files from file explorer
- Visual feedback during drag-over
- Support for single or multiple files
- Progress indicator for large files
- Error handling for invalid files

**Implementation Flow**:
1. User drags .ics file(s) over application window
2. Drop zone highlights to indicate acceptance
3. User releases mouse - files dropped
4. Parse each .ics file
5. Display import preview dialog:
   - List of events to import
   - Conflict detection (duplicate events)
   - Option to select which events to import
6. User confirms import
7. Events created in database
8. Calendar view refreshes

**Drag-and-Drop Handler** (`drag_drop_handler.rs`):
```rust
pub struct DragDropHandler {
    ical_service: ICalService,
    event_service: EventService,
}

impl DragDropHandler {
    pub fn handle_file_drop(&self, files: Vec<PathBuf>) -> Result<Vec<Event>>;
    pub fn preview_import(&self, events: Vec<Event>) -> ImportPreview;
    pub fn detect_conflicts(&self, events: &[Event]) -> Vec<Conflict>;
    pub fn import_events(&self, events: Vec<Event>) -> Result<Vec<i64>>;
}
```

### Event Rescheduling

**Drag Operations**:
- Drag event to different time slot → reschedule
- Drag event to different day → move date
- Drag event edges → adjust duration
- Drag between views (if multiple calendars in future)

**Visual Feedback**:
- Ghost/preview of event during drag
- Target slot highlights
- Invalid drop targets grayed out
- Cursor changes to indicate valid/invalid drop

**Constraints**:
- Cannot drag past events
- Respect event minimum duration
- Check for conflicts on drop
- Prompt for recurring event handling (this occurrence vs. all)

## Recurrence Implementation

Using the `rrule` crate for RFC 5545 compliant recurrence rules:

```rust
// Example recurrence patterns
"FREQ=DAILY"                           // Every day
"FREQ=WEEKLY;BYDAY=MO,WE,FR"          // Mon, Wed, Fri
"FREQ=WEEKLY;INTERVAL=2"               // Fortnightly (every 2 weeks)
"FREQ=WEEKLY;INTERVAL=2;BYDAY=MO"     // Every other Monday
"FREQ=MONTHLY;BYMONTHDAY=1"           // 1st of each month
"FREQ=MONTHLY;BYDAY=2TU"              // 2nd Tuesday of month
"FREQ=MONTHLY;INTERVAL=3"             // Quarterly (every 3 months)
"FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=1" // First day of each quarter
"FREQ=YEARLY;BYMONTH=12;BYMONTHDAY=25" // Christmas
```

### Frequency Types Module (`src/models/recurrence/frequency.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Frequency {
    Daily,
    Weekly,
    Fortnightly,    // Convenience wrapper for Weekly with interval=2
    Monthly,
    Quarterly,      // Convenience wrapper for Monthly with interval=3
    Yearly,
    Custom,         // For complex patterns
}

impl Frequency {
    /// Convert to rrule frequency and interval
    pub fn to_rrule_params(&self) -> (RRuleFrequency, u32) {
        match self {
            Frequency::Daily => (RRuleFrequency::Daily, 1),
            Frequency::Weekly => (RRuleFrequency::Weekly, 1),
            Frequency::Fortnightly => (RRuleFrequency::Weekly, 2),
            Frequency::Monthly => (RRuleFrequency::Monthly, 1),
            Frequency::Quarterly => (RRuleFrequency::Monthly, 3),
            Frequency::Yearly => (RRuleFrequency::Yearly, 1),
            Frequency::Custom => (RRuleFrequency::Daily, 1), // Handled separately
        }
    }
}
```

### Modular Testing Structure

Each module must have comprehensive tests:

**Example: `src/models/recurrence/frequency.rs`** (~50 lines)
**Test: `tests/unit/models/recurrence_frequency_tests.rs`** (~150 lines)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fortnightly_to_rrule_params() {
        let freq = Frequency::Fortnightly;
        let (rrule_freq, interval) = freq.to_rrule_params();
        assert_eq!(interval, 2);
    }
    
    #[test]
    fn test_quarterly_to_rrule_params() {
        let freq = Frequency::Quarterly;
        let (rrule_freq, interval) = freq.to_rrule_params();
        assert_eq!(interval, 3);
    }
}
```

## Reminder Scheduling Strategy

1. **Background Service**: Run a background thread checking for upcoming reminders
2. **Check Interval**: Poll database every 60 seconds for reminders
3. **Trigger Window**: Trigger reminders within 60-second accuracy
4. **Notification**: Use Windows Action Center for notifications
5. **Persistence**: Track triggered reminders to avoid duplicates

## Theme System

### Theme Configuration (TOML)
```toml
[theme]
name = "Dark"
type = "dark"

[colors]
background = "#1e1e1e"
surface = "#252526"
primary = "#007acc"
secondary = "#68217a"
text = "#ffffff"
text_secondary = "#cccccc"
border = "#3e3e42"
accent = "#0e639c"

[calendar]
weekend = "#3e3e42"
today = "#094771"
selected = "#007acc"
event_default = "#68217a"
```

## Windows-Specific Considerations

1. **Native Look**: Use Windows design guidelines for UI elements
2. **System Tray**: Add system tray icon for quick access
3. **Start with Windows**: Optional auto-start configuration
4. **DPI Awareness**: Handle high-DPI displays properly
5. **Windows Notifications**: Use Windows Action Center API
6. **File Associations**: Associate with .ics files for double-click open
7. **Drag-and-Drop**: Native Windows drag-and-drop support for .ics files
8. **Font Rendering**: Use ClearType font rendering for crisp text
9. **Column Resizing**: Native Windows resize cursor and behavior
10. **Preferences Storage**: Store in user's AppData folder

## Testing Strategy

### Testing Philosophy
- **Test-Driven Development (TDD)**: Write tests before implementation when possible
- **Comprehensive Coverage**: Aim for >90% code coverage
- **Fast Tests**: Unit tests should run in milliseconds
- **Isolated Tests**: Each test should be independent and reproducible
- **Clear Test Names**: Use descriptive names that explain what is being tested

### Test Organization

#### 1. Unit Tests (`tests/unit/`)
Mirror the `src/` directory structure exactly:

```
tests/unit/
├── models/
│   ├── event/
│   │   ├── event_tests.rs          # Test Event struct
│   │   ├── builder_tests.rs        # Test Event builder
│   │   └── validator_tests.rs      # Test Event validation
│   ├── recurrence/
│   │   ├── rule_tests.rs           # Test RecurrenceRule
│   │   ├── frequency_tests.rs      # Test Frequency enum
│   │   ├── calculator_tests.rs     # Test occurrence calculation
│   │   ├── fortnightly_tests.rs    # Test fortnightly patterns
│   │   ├── quarterly_tests.rs      # Test quarterly patterns
│   │   └── exceptions_tests.rs     # Test exception handling
│   └── reminder/
│       ├── reminder_tests.rs       # Test Reminder struct
│       └── trigger_tests.rs        # Test trigger logic
├── services/
│   ├── database/
│   │   ├── connection_tests.rs     # Test DB connections
│   │   ├── migrations_tests.rs     # Test migrations
│   │   └── repositories_tests.rs   # Test repositories
│   ├── event/
│   │   ├── crud_tests.rs           # Test CRUD operations
│   │   └── query_tests.rs          # Test queries
│   └── reminder/
│       ├── scheduler_tests.rs      # Test scheduling
│       └── checker_tests.rs        # Test reminder checking
└── utils/
    ├── date/
    │   ├── formatting_tests.rs     # Test date formatting
    │   └── calculations_tests.rs   # Test date math
    └── validation_tests.rs         # Test validation logic
```

#### 2. Integration Tests (`tests/integration/`)
Test interactions between components:

- `event_workflow_tests.rs`: Create → Read → Update → Delete events
- `recurrence_workflow_tests.rs`: Create recurring events → Calculate occurrences
- `reminder_workflow_tests.rs`: Set reminder → Trigger → Notify
- `database_integration_tests.rs`: Full database operations
- `ical_integration_tests.rs`: Import/Export workflows

#### 3. Property-Based Tests (`tests/property/`)
Use `proptest` for edge case discovery:

- `recurrence_properties.rs`: Test recurrence calculations with random dates
- `date_properties.rs`: Test date operations don't produce invalid dates
- `event_properties.rs`: Test event invariants hold

#### 4. Benchmark Tests (`benches/`)
Performance testing:

- `recurrence_bench.rs`: Benchmark calculating 10,000 occurrences
- `database_bench.rs`: Benchmark query performance with large datasets
- `ui_render_bench.rs`: Benchmark UI rendering with many events

### Test File Template

Every source file should have a corresponding test file:

**Source: `src/models/recurrence/frequency.rs`** (~80 lines)
```rust
// Implementation code
```

**Test: `tests/unit/models/recurrence_frequency_tests.rs`** (~200 lines)
```rust
use rust_calendar::models::recurrence::Frequency;
use test_case::test_case;

#[test]
fn test_daily_to_rrule_params() {
    let freq = Frequency::Daily;
    let (_, interval) = freq.to_rrule_params();
    assert_eq!(interval, 1);
}

#[test]
fn test_fortnightly_to_rrule_params() {
    let freq = Frequency::Fortnightly;
    let (_, interval) = freq.to_rrule_params();
    assert_eq!(interval, 2);
}

#[test]
fn test_quarterly_to_rrule_params() {
    let freq = Frequency::Quarterly;
    let (_, interval) = freq.to_rrule_params();
    assert_eq!(interval, 3);
}

#[test_case(Frequency::Daily, 1)]
#[test_case(Frequency::Weekly, 1)]
#[test_case(Frequency::Fortnightly, 2)]
#[test_case(Frequency::Monthly, 1)]
#[test_case(Frequency::Quarterly, 3)]
#[test_case(Frequency::Yearly, 1)]
fn test_frequency_intervals(freq: Frequency, expected_interval: u32) {
    let (_, interval) = freq.to_rrule_params();
    assert_eq!(interval, expected_interval);
}
```

### Testing Requirements

1. **Every Function/Method**: Must have at least one test
2. **Every Module**: Must have a corresponding test module
3. **Edge Cases**: Must test boundary conditions
4. **Error Cases**: Must test error handling paths
5. **Integration Points**: Must test component interactions

### Continuous Integration

```yaml
# Example CI checks
- cargo test --all                    # All tests must pass
- cargo test --all-features          # Test with all features
- cargo clippy -- -D warnings        # No clippy warnings
- cargo fmt -- --check               # Code must be formatted
- cargo tarpaulin --out Xml          # Coverage must be >90%
```

## Security Considerations

1. **Input Validation**: Sanitize all user input
2. **SQL Injection**: Use parameterized queries (rusqlite handles this)
3. **Data Integrity**: Validate recurrence rules and dates
4. **Backup**: Provide data backup functionality

## Future Enhancements (Post-MVP)

- [ ] Cloud synchronization (CalDAV support)
- [ ] Mobile companion app
- [ ] Calendar sharing
- [ ] Task/todo list integration
- [ ] Time zone support for events
- [ ] Weather integration
- [ ] Custom event types (birthdays, holidays)
- [ ] Event templates
- [ ] Natural language event creation
- [ ] Advanced keyboard shortcuts
- [ ] Printing support with customizable layouts
- [ ] Custom view creation (save personalized views)
- [ ] Multiple calendar overlay (work/personal/shared)
- [ ] Gantt chart view for project planning
- [ ] Timeline view for historical events
- [ ] Mini-month calendar widget

## Success Criteria

1. **Performance**: UI renders at 60 FPS with 10,000+ events
2. **Reliability**: Zero data loss, robust error handling
3. **Usability**: Intuitive interface, < 30 second learning curve
4. **Accuracy**: Perfect recurrence calculation matching RFC 5545
5. **Notifications**: 100% reliable reminder delivery

## Resources & References

- [RFC 5545 - iCalendar](https://tools.ietf.org/html/rfc5545)
- [Iced GUI Framework](https://github.com/iced-rs/iced)
- [Slint UI Framework](https://slint.dev/)
- [Windows Notifications API](https://learn.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/)
- [Rust SQLite](https://github.com/rusqlite/rusqlite)

## Development Setup

1. Install Rust toolchain (rustup)
2. Install Visual Studio Build Tools (for Windows development)
3. Clone repository
4. Run `cargo build` to fetch dependencies
5. Run `cargo test` to verify setup
6. Run `cargo run` to start application

## License

To be determined (MIT or Apache 2.0 recommended for Rust projects)

---

**Document Version**: 1.0  
**Last Updated**: November 6, 2025  
**Status**: Planning Phase
