# Rust Calendar Enhancement Plan

**Created:** November 19, 2025  
**Target Completion:** 7-10 days  
**Current Branch:** feature/notification-system

## Overview

This document outlines the implementation plan for 20 improvements to the rust-calendar application. Features are organized into 5 phases based on priority, dependencies, and implementation complexity.

---

## Phase 1: Quick Wins & Safety Features (Days 1-2)

**Goal:** Implement high-impact features with minimal complexity that improve safety and usability.

### 1. Database Backup System ⭐ CRITICAL

**Priority:** Highest - Prevents data loss  
**Estimated Time:** 3-4 hours  
**Branch:** `feat/database-backup`

**Implementation:**
- Create `src/services/backup/mod.rs`
- Add `BackupService` with methods:
  - `create_backup(db_path: &Path, backup_dir: &Path) -> Result<PathBuf>`
  - `restore_backup(backup_path: &Path, db_path: &Path) -> Result<()>`
  - `list_backups(backup_dir: &Path) -> Result<Vec<BackupInfo>>`
  - `auto_backup_on_startup(db_path: &Path) -> Result<()>`
- Add File menu items:
  - "Backup Database..." → File dialog to save backup
  - "Restore from Backup..." → File picker + confirmation dialog
  - "Manage Backups..." → New dialog showing backup list with restore/delete actions
- Implement auto-backup on startup (keep last 5 backups)
- Store backups in `%AppData%/rust-calendar/backups/` with timestamp naming

**Files to Modify:**
- `src/services/mod.rs` - Add backup module
- `src/ui_egui/app.rs` - Add File menu items, call auto_backup in `new()`
- Create `src/ui_egui/dialogs/backup_manager.rs` - Backup management dialog

**Testing:**
- Create backup, verify file exists and is valid SQLite
- Restore backup, verify data restored correctly
- Test auto-backup creates files on startup
- Test backup rotation (keeps only last 5)

---

### 2. Keyboard Shortcuts

**Priority:** High - Improves efficiency  
**Estimated Time:** 2-3 hours  
**Branch:** `feat/keyboard-shortcuts`

**Shortcuts to Implement:**
- `Ctrl+N` - New Event (open event dialog)
- `Ctrl+T` - Today (navigate to current date)
- `Ctrl+S` - Settings
- `Ctrl+F` - Focus search bar (Phase 2)
- `Ctrl+B` - Backup Database
- `Arrow Left/Right` - Navigate days/weeks/months
- `Arrow Up/Down` - Navigate weeks/months (contextual)
- `Ctrl+Z` - Undo (Phase 3)
- `Ctrl+Shift+Z` - Redo (Phase 3)

**Implementation:**
- Extend keyboard handling in `src/ui_egui/app.rs::update()`
- Use existing pattern: `ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::N))`
- Add shortcut hints to menu labels: "New Event    Ctrl+N"
- Document shortcuts in `README.md`

**Files to Modify:**
- `src/ui_egui/app.rs` - Add keyboard handlers in `update()` method
- Menu button labels - Append shortcut text

**Testing:**
- Test each shortcut performs correct action
- Test modifiers (Ctrl, Shift, Alt) work correctly
- Test shortcuts work when dialogs are closed
- Verify Escape still closes dialogs (priority order)

---

### 3. Export Menu Integration

**Priority:** High - Completes existing feature  
**Estimated Time:** 2 hours  
**Branch:** `feat/export-menu`

**Implementation:**
- Add File menu items:
  - "Export Event..." (on event context menu, right-click)
  - "Export All Events..." → File dialog, save all events as .ics
  - "Export Date Range..." → Date picker dialog, export subset
- Use existing `ICalendarService::single()` and `multiple()`
- Use `rfd::FileDialog::new().add_filter("iCalendar", &["ics"]).save_file()`
- Show success toast notification after export

**Files to Modify:**
- `src/ui_egui/app.rs` - Add File menu items
- Event context menus in views (day/week/month) - Add "Export Event"
- Create `src/ui_egui/dialogs/export_dialog.rs` - Date range picker for export

**Testing:**
- Export single event, verify .ics file valid
- Export all events, verify all included
- Export date range, verify only events in range included
- Test file overwrite confirmation works

---

### 4. Event Validation Enhancements

**Priority:** Medium - Improves data quality  
**Estimated Time:** 2 hours  
**Branch:** `feat/validation-improvements`

**Validation Rules:**
- ✅ Title not empty (already implemented)
- ✅ End after start (already implemented)
- ✅ Color hex format (already implemented)
- 🆕 Recurrence rule syntax validation before save
- 🆕 Overlap detection (warn only, don't block)
- 🆕 All-day event must have start/end on day boundaries
- 🆕 Max title length (255 chars)
- 🆕 Date not in distant past (warn if > 5 years ago)

**Implementation:**
- Add `EventService::check_overlaps(event: &Event) -> Result<Vec<Event>>` - Returns overlapping events
- Add `validate_recurrence_rule(rule: &str) -> Result<()>` using rrule parser
- Show warnings in event dialog as yellow text (non-blocking)
- Show errors in red text (blocking save)

**Files to Modify:**
- `src/services/event/service.rs` - Add validation methods
- `src/ui_egui/event_dialog.rs` - Display warnings and errors
- `src/models/event/mod.rs` - Add validation to builder

**Testing:**
- Test overlapping events show warning but allow save
- Test invalid RRULE blocks save with error message
- Test all validation rules with edge cases

---

## Phase 2: Search & Organization (Days 3-4)

**Goal:** Enable users to find and organize events efficiently.

### 5. Event Search

**Priority:** High - Major productivity improvement  
**Estimated Time:** 4 hours  
**Branch:** `feat/event-search`

**Implementation:**
- Add search text box to top panel in `app.rs` (next to view selector)
- Create `EventService::search(query: &str, date_range: Option<(NaiveDate, NaiveDate)>) -> Result<Vec<Event>>`
  - Search title, description, location, category (case-insensitive)
  - Use SQL LIKE: `WHERE title LIKE '%query%' OR description LIKE '%query%' OR ...`
- Display results in popup window/panel below search box
  - Group by date
  - Click result to navigate to that date and highlight event
  - Show event time, title, location
- Clear search with X button or Escape
- Bind Ctrl+F to focus search box

**Files to Modify:**
- `src/services/event/service.rs` - Add `search()` method
- `src/ui_egui/app.rs` - Add search UI to top panel, handle results

**Testing:**
- Search for partial title matches
- Search for text in description and location
- Test case-insensitive search
- Test empty search shows no results
- Test navigation to search result

---

### 6. Category Management & Filtering

**Priority:** High - Organizational feature  
**Estimated Time:** 5 hours  
**Branch:** `feat/category-management`

**Database Changes:**
```sql
CREATE TABLE categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL,  -- Hex color for category
    icon TEXT,  -- Optional emoji or icon
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Populate default categories
INSERT INTO categories (name, color, icon) VALUES
    ('Work', '#3B82F6', '💼'),
    ('Personal', '#10B981', '🏠'),
    ('Birthday', '#F59E0B', '🎂'),
    ('Holiday', '#EF4444', '🎉'),
    ('Meeting', '#8B5CF6', '👥'),
    ('Deadline', '#DC2626', '⏰');
```

**Implementation:**
- Add Settings → Categories tab with CRUD operations
- Add category dropdown filter in menu bar (All, Work, Personal, etc.)
- Store selected filter in `CalendarApp.active_category_filter: Option<String>`
- Update all view rendering to filter events by category
- Event dialog shows categories as dropdown (not free text)
- Show category icon/color badge on events in all views

**Files to Modify:**
- `src/services/database/mod.rs` - Add categories table to schema
- Create `src/services/category.rs` - CategoryService for CRUD
- `src/models/category/mod.rs` - Category model
- `src/ui_egui/dialogs/settings.rs` - Add Categories tab
- `src/ui_egui/app.rs` - Add category filter dropdown
- `src/ui_egui/event_dialog.rs` - Change category to dropdown
- All view files - Apply category filter when rendering

**Testing:**
- Create/edit/delete categories in settings
- Assign category to event, verify it displays
- Filter by category, verify only matching events shown
- Test "All Categories" shows everything

---

### 7. Event Templates

**Priority:** Medium - Time-saving feature  
**Estimated Time:** 4 hours  
**Branch:** `feat/event-templates`

**Database Changes:**
```sql
CREATE TABLE event_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    description TEXT,
    location TEXT,
    duration INTEGER NOT NULL,  -- Duration in minutes
    all_day INTEGER NOT NULL DEFAULT 0,
    category TEXT,
    color TEXT,
    recurrence_rule TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**Implementation:**
- Add File → Templates menu with:
  - "Save Current Event as Template..." (when editing event)
  - "Manage Templates..." → Dialog to view/edit/delete templates
  - Template list submenu showing all templates (quick create)
- Create template from event: saves all properties except dates
- Apply template: opens event dialog with template values prefilled, user sets date/time
- Templates dialog shows list with preview and actions

**Files to Modify:**
- `src/services/database/mod.rs` - Add event_templates table
- Create `src/services/template.rs` - TemplateService
- `src/models/template/mod.rs` - EventTemplate model
- `src/ui_egui/app.rs` - Add File → Templates menu
- Create `src/ui_egui/dialogs/template_manager.rs` - Template CRUD dialog
- `src/ui_egui/event_dialog.rs` - Add "Save as Template" button

**Testing:**
- Save event as template, verify stored correctly
- Create event from template, verify all fields prefilled
- Edit template, verify changes apply to new events
- Delete template, verify removed from menu

---

### 8. Week Numbers

**Priority:** Low - Nice to have  
**Estimated Time:** 1 hour  
**Branch:** `feat/week-numbers`

**Implementation:**
- Add `show_week_numbers: bool` to Settings model and database
- Add toggle in Settings dialog → General tab
- Calculate ISO week number using `chrono::Datelike::iso_week()`
- Display in week view header: "Week 47 - Nov 18-24, 2025"
- Display in month view: small number in corner of each week row

**Files to Modify:**
- `src/models/settings/mod.rs` - Add show_week_numbers field
- `src/services/database/mod.rs` - Add column migration
- `src/ui_egui/dialogs/settings.rs` - Add checkbox
- `src/ui_egui/views/week_view.rs` - Show week number in header
- `src/ui_egui/views/month_view.rs` - Show week numbers in sidebar

**Testing:**
- Enable week numbers, verify displayed correctly
- Verify ISO week calculation matches calendar standards
- Test transition across year boundary (week 52/53 → week 1)

---

## Phase 3: Advanced Editing (Days 5-6)

**Goal:** Enable power users to work faster with advanced editing features.

### 9. Undo/Redo System

**Priority:** High - Safety net for mistakes  
**Estimated Time:** 6 hours  
**Branch:** `feat/undo-redo`

**Implementation:**
- Create command pattern:
```rust
trait Command {
    fn execute(&self, app: &mut CalendarApp) -> Result<()>;
    fn undo(&self, app: &mut CalendarApp) -> Result<()>;
    fn description(&self) -> String;
}

struct CreateEventCommand { event: Event }
struct UpdateEventCommand { old: Event, new: Event }
struct DeleteEventCommand { event: Event }
struct MoveEventCommand { event_id: i64, old_start: DateTime, new_start: DateTime }
```

- Add to `CalendarApp`:
```rust
undo_stack: Vec<Box<dyn Command>>,
redo_stack: Vec<Box<dyn Command>>,
max_undo_history: usize = 50,
```

- Wrap all event modifications in commands
- Add Edit menu with "Undo" and "Redo" items (show command description)
- Bind Ctrl+Z and Ctrl+Shift+Z
- Clear redo stack when new command executed
- Limit undo stack to 50 operations (FIFO)

**Files to Modify:**
- Create `src/ui_egui/commands/mod.rs` - Command trait and implementations
- `src/ui_egui/app.rs` - Add undo/redo stacks, Edit menu, keyboard shortcuts
- Wrap all event CRUD operations in command execution

**Testing:**
- Create event, undo, verify event removed
- Delete event, undo, verify event restored
- Multiple undo/redo operations
- Test redo stack clears on new operation
- Test max history limit (51st operation removes oldest)

---

### 10. Multi-Select Events

**Priority:** Medium - Bulk operations  
**Estimated Time:** 5 hours  
**Branch:** `feat/multi-select-events`

**Implementation:**
- Add to `CalendarApp`:
```rust
selected_event_ids: HashSet<i64>,
multi_select_mode: bool,
```

- Selection interactions:
  - `Ctrl+Click` on event: Toggle selection
  - `Shift+Click` on event: Select range between last selected and clicked
  - `Escape`: Clear selection
  - Visual: Highlight selected events with border or background tint

- Context menu on selected events:
  - "Delete Selected (N events)"
  - "Move Selected to..." → Date picker
  - "Set Category for Selected..." → Category picker
  - "Export Selected..."

- Add Edit menu: "Select All Events on Date", "Clear Selection"

**Files to Modify:**
- `src/ui_egui/app.rs` - Add selection state
- All view files (`day_view.rs`, `week_view.rs`, `month_view.rs`) - Handle selection clicks, visual highlight
- Add context menu for multi-select operations
- Create `src/ui_egui/dialogs/bulk_operations.rs` - Bulk edit dialogs

**Testing:**
- Select multiple events with Ctrl+Click
- Select range with Shift+Click
- Bulk delete selected events
- Bulk move to different date
- Bulk category assignment

---

### 11. Drag-to-Resize Events

**Priority:** Medium - Intuitive editing  
**Estimated Time:** 4 hours  
**Branch:** `feat/drag-resize-events`

**Implementation:**
- Extend `DragManager` with `DragMode` enum:
```rust
enum DragMode {
    Move,
    ResizeStart,  // Dragging top edge
    ResizeEnd,    // Dragging bottom edge
}
```

- Detect cursor position on event:
  - Top 5px → ResizeStart cursor (↕)
  - Bottom 5px → ResizeEnd cursor (↕)
  - Middle → Move cursor (✋)

- Implement resize logic:
  - Snap to time slot intervals (15/30/60 min based on settings)
  - Update event start/end via `EventService::update()`
  - Visual feedback during drag (ghost event)

- Only allow resize for non-recurring events (too complex for recurring)

**Files to Modify:**
- `src/ui_egui/drag.rs` - Add DragMode, edge detection, resize logic
- `src/ui_egui/views/day_view.rs` - Handle resize interactions
- `src/ui_egui/views/week_view.rs` - Handle resize interactions
- `src/ui_egui/views/workweek_view.rs` - Handle resize interactions

**Testing:**
- Drag top edge to change start time
- Drag bottom edge to change end time
- Verify snapping to time slots
- Test minimum duration (15 minutes)
- Verify recurring events cannot be resized

---

### 12. Countdown Card Templates

**Priority:** Low - Nice to have  
**Estimated Time:** 3 hours  
**Branch:** `feat/countdown-templates`

**Implementation:**
- Add table:
```sql
CREATE TABLE countdown_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    visuals_json TEXT NOT NULL,  -- Serialized CountdownCardVisuals
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

- Add to countdown card settings dialog:
  - "Save as Template" button → Name input → Save current visuals
  - "Load Template" dropdown → Apply template visuals to card
  - "Manage Templates..." button → Template CRUD dialog

- Create `CountdownTemplateService` for database operations

**Files to Modify:**
- `src/services/database/mod.rs` - Add countdown_templates table
- Create `src/services/countdown_template.rs` - Template service
- `src/ui_egui/app/countdown/settings.rs` - Add template UI
- Create `src/ui_egui/dialogs/countdown_template_manager.rs` - Template manager

**Testing:**
- Save countdown visuals as template
- Apply template to different card
- Manage templates (rename, delete)
- Verify template doesn't include position/size (only colors/fonts)

---

## Phase 4: Polish & Reliability (Days 7-8)

**Goal:** Improve stability, configurability, and user experience.

### 13. Notification Settings UI

**Priority:** High - Complete notification feature  
**Estimated Time:** 3 hours  
**Branch:** `feat/notification-settings-ui`

**Implementation:**
- Add "Notifications" tab to Settings dialog
- Expose `CountdownNotificationConfig` fields:
  - Enable/disable notifications checkbox
  - Use visual warnings checkbox
  - Use system notifications checkbox
  - Warning thresholds:
    - Approaching: slider 1-72 hours (default 24)
    - Imminent: slider 0.5-24 hours (default 1)
    - Critical: slider 1-60 minutes (default 5)
  - Auto-dismiss settings:
    - Enable auto-dismiss checkbox
    - Dismiss on event start checkbox
    - Dismiss on event end checkbox
    - Delay after start: slider 0-60 minutes

- Save to `CountdownService` persistence
- Apply changes immediately to all cards

**Files to Modify:**
- `src/ui_egui/dialogs/settings.rs` - Add Notifications tab
- `src/ui_egui/app.rs` - Pass notification config to settings dialog
- `src/services/countdown/service.rs` - Expose config getters/setters (already implemented)

**Testing:**
- Change thresholds, verify visual warnings change
- Disable notifications, verify no alerts shown
- Test auto-dismiss settings
- Verify settings persist across app restarts

---

### 14. Crash Recovery

**Priority:** Medium - Improves reliability  
**Estimated Time:** 2 hours  
**Branch:** `feat/crash-recovery`

**Implementation:**
- Create `app_state.json` in AppData directory:
```json
{
    "window_position": {"x": 100, "y": 100},
    "window_size": {"width": 1200, "height": 800},
    "current_view": "Month",
    "current_date": "2025-11-19",
    "theme": "light",
    "last_saved": "2025-11-19T10:30:00Z"
}
```

- Save state on every view change, date navigation
- Load state in `CalendarApp::new()`, with fallback to defaults
- Handle corrupt JSON gracefully (log error, use defaults)
- Store separately from database for independence

**Files to Modify:**
- Create `src/services/app_state.rs` - State persistence service
- `src/ui_egui/app.rs` - Save state on changes, load in `new()`

**Testing:**
- Close app, reopen, verify window position/size restored
- Navigate to different view/date, close, verify restored
- Corrupt JSON file, verify app starts with defaults
- Delete JSON file, verify app starts normally

---

### 15. Database Migration System

**Priority:** Medium - Technical debt  
**Estimated Time:** 4 hours  
**Branch:** `feat/migration-system`

**Implementation:**
- Create `migrations/` folder with numbered SQL files:
  - `001_initial_schema.sql`
  - `002_add_categories_table.sql`
  - `003_add_templates_table.sql`
  - etc.

- Add `schema_version` table:
```sql
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

- Create `MigrationManager`:
  - `get_current_version() -> i32`
  - `get_pending_migrations() -> Vec<Migration>`
  - `apply_migration(migration: &Migration) -> Result<()>`
  - `apply_all_pending() -> Result<()>`

- Run migrations on startup before any database operations
- Log migration applications

**Files to Modify:**
- Create `migrations/` directory with SQL files
- Create `src/services/migration.rs` - Migration manager
- `src/services/database/mod.rs` - Call migration manager on init
- `src/main.rs` - Run migrations before UI startup

**Testing:**
- Fresh database applies all migrations
- Existing database applies only new migrations
- Failed migration rolls back and reports error
- Verify idempotency (applying twice doesn't break)

---

### 16. Event Caching

**Priority:** Low - Performance optimization  
**Estimated Time:** 3 hours  
**Branch:** `feat/event-caching`

**Implementation:**
- Add to `CalendarApp`:
```rust
event_cache: HashMap<NaiveDate, Vec<Event>>,
cache_valid_until: Option<Instant>,
cache_ttl: Duration = Duration::from_secs(60),
```

- Cache strategy:
  - Key: Date being viewed
  - Value: Expanded recurring events for that date
  - Invalidate entire cache on any CRUD operation
  - Invalidate on TTL expiry (60 seconds)
  - Pre-cache visible date range (current week/month)

- Add `EventService::get_cached_events(date: NaiveDate, cache: &mut HashMap) -> Vec<Event>`

**Files to Modify:**
- `src/ui_egui/app.rs` - Add cache fields and invalidation logic
- `src/services/event/service.rs` - Add cache-aware query methods
- All view files - Use cached events

**Testing:**
- Verify cache hit improves performance (benchmark)
- Create event, verify cache invalidates
- Wait for TTL, verify cache expires
- Test with 100+ recurring events

---

## Phase 5: Nice-to-Have Features (Days 9-10)

**Goal:** Additional enhancements that significantly improve user experience.

### 17. Natural Language Event Input

**Priority:** Low - Advanced feature  
**Estimated Time:** 6 hours  
**Branch:** `feat/natural-language-input`

**Implementation:**
- Add "Quick Add" text box above main view area
- Parse patterns:
  - Date: "tomorrow", "next monday", "12/25", "Dec 25"
  - Time: "2pm", "14:00", "2:30pm"
  - Duration: "1h", "30min", "2 hours"
  - Title: Remaining text
  - Examples:
    - "Meeting tomorrow at 2pm for 1 hour"
    - "Dentist next monday 9am"
    - "Lunch Dec 25 12:30pm 1h"

- Use regex patterns + chrono parsing:
```rust
struct ParsedEvent {
    title: String,
    date: Option<NaiveDate>,
    time: Option<NaiveTime>,
    duration: Option<Duration>,
}
```

- On Enter: Parse text, prefill event dialog or create directly
- Show parse errors in red text below input

**Files to Modify:**
- Create `src/services/natural_language.rs` - Parser
- `src/ui_egui/app.rs` - Add quick add input box
- Add unit tests for parser in `tests/unit/services/natural_language.rs`

**Testing:**
- Test various date formats
- Test time parsing (12h and 24h)
- Test duration extraction
- Test edge cases (invalid dates, ambiguous input)

---

### 18. Event Attachments/Notes

**Priority:** Low - Document management  
**Estimated Time:** 5 hours  
**Branch:** `feat/event-attachments`

**Database Changes:**
```sql
ALTER TABLE events ADD COLUMN notes TEXT;

CREATE TABLE event_attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    added_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
);
```

**Implementation:**
- Add "Notes" tab to event dialog with multi-line text area
- Add "Attachments" section with:
  - List of attached files with remove button
  - "Add Attachment" button → File picker
  - Click filename to open with system default app

- Store file paths, not file contents (reference only)
- Warn if file path becomes invalid (file moved/deleted)

**Files to Modify:**
- `src/models/event/mod.rs` - Add notes field
- `src/services/database/mod.rs` - Add attachments table
- Create `src/services/attachment.rs` - Attachment CRUD
- `src/ui_egui/event_dialog.rs` - Add Notes tab and Attachments section

**Testing:**
- Add note to event, verify saved and displayed
- Attach file, verify link stored
- Click attachment, verify file opens
- Delete attachment, verify removed from database
- Test with moved/deleted files (show error)

---

### 19. Notification Sound

**Priority:** Low - Audio feedback  
**Estimated Time:** 3 hours  
**Branch:** `feat/notification-sound`

**Dependencies:**
- Add `rodio = "0.17"` to Cargo.toml for audio playback

**Implementation:**
- Add sound fields to `CountdownNotificationConfig`:
```rust
pub enable_sound: bool,
pub sound_file: Option<PathBuf>,
```

- Add to Notifications settings tab:
  - Enable sound checkbox
  - Sound file picker button
  - "Test Sound" button
  - Default to system notification sound if None

- Play sound when showing countdown alerts
- Support .wav and .mp3 files

**Files to Modify:**
- `src/services/countdown/models.rs` - Add sound fields to config
- `src/services/notification/mod.rs` - Add sound playback
- `src/ui_egui/dialogs/settings.rs` - Add sound UI to Notifications tab

**Testing:**
- Select sound file, verify plays on notification
- Test with .wav and .mp3 files
- Disable sound, verify no audio plays
- Test default system sound

---

### 20. Dark/Light Mode Auto-Switch

**Priority:** Low - Convenience feature  
**Estimated Time:** 3 hours  
**Branch:** `feat/auto-theme-switch`

**Dependencies:**
- Add `dark-light = "1.0"` to Cargo.toml for system theme detection

**Implementation:**
- Add theme mode to Settings:
```rust
pub enum ThemeMode {
    Manual,        // User selected theme
    FollowSystem,  // Match OS theme
    Scheduled,     // Switch at times
}

pub theme_mode: ThemeMode,
pub light_theme_time: NaiveTime,  // e.g., 07:00
pub dark_theme_time: NaiveTime,   // e.g., 19:00
```

- Check in `CalendarApp::update()`:
  - If FollowSystem: Check `dark_light::detect()` every 5 seconds
  - If Scheduled: Check current time against schedule
  - Switch theme via `ThemeService` if needed

- Add UI in Settings → Appearance:
  - Radio buttons: Manual / Follow System / Scheduled
  - Time pickers for scheduled mode

**Files to Modify:**
- `src/models/settings/mod.rs` - Add theme mode fields
- `src/ui_egui/dialogs/settings.rs` - Add theme mode UI
- `src/ui_egui/app.rs` - Add auto-switch logic in update loop

**Testing:**
- Test follow system mode with OS theme changes
- Test scheduled mode switches at correct times
- Verify manual mode unaffected
- Test across midnight boundary

---

## Implementation Guidelines

### General Principles

1. **One Feature Per Branch** - Create feature branch off `feature/notification-system`, merge back when complete
2. **Commit Frequently** - Commit after each logical unit of work with descriptive messages
3. **Test Before Commit** - Run `cargo test` and `cargo run` to verify no regressions
4. **Update Documentation** - Update README.md and inline docs for user-facing features
5. **Preserve Existing Functionality** - Don't break working features

### Commit Message Format

```
<type>: <short description>

<optional longer description>
<optional breaking changes>

Examples:
feat: add database backup and restore functionality
fix: prevent crash when loading corrupt JSON state
chore: update dependencies to latest versions
docs: document keyboard shortcuts in README
```

### Testing Strategy

1. **Unit Tests** - Test pure logic functions (parsers, validators)
2. **Integration Tests** - Test service layer interactions with database
3. **Manual Testing** - Test UI interactions and visual feedback
4. **Regression Testing** - Verify existing features still work

### Code Style

- Follow existing Rust conventions
- Use `rustfmt` for formatting
- Use `clippy` for linting
- Document public APIs with doc comments
- Keep functions focused and small

### Error Handling

- Use `Result<T, Error>` for fallible operations
- Use `anyhow::Result` for application errors
- Display user-friendly error messages in UI
- Log technical errors to console

### Database Changes

- Always use migrations for schema changes
- Never modify data in migrations (data migrations separate)
- Test migrations on copy of production database
- Document migration rationale in SQL comments

---

## Progress Tracking

### Phase 1 Status
- [x] Database Backup System
- [ ] Keyboard Shortcuts
- [ ] Export Menu Integration
- [ ] Event Validation Enhancements

### Phase 2 Status
- [ ] Event Search
- [ ] Category Management & Filtering
- [ ] Event Templates
- [ ] Week Numbers

### Phase 3 Status
- [ ] Undo/Redo System
- [ ] Multi-Select Events
- [ ] Drag-to-Resize Events
- [ ] Countdown Card Templates

### Phase 4 Status
- [ ] Notification Settings UI
- [ ] Crash Recovery
- [ ] Database Migration System
- [ ] Event Caching

### Phase 5 Status
- [ ] Natural Language Event Input
- [ ] Event Attachments/Notes
- [ ] Notification Sound
- [ ] Dark/Light Mode Auto-Switch

---

## Notes

- Estimated total time: 70-80 hours over 7-10 days
- Phases can be worked in parallel if multiple developers
- Some features depend on earlier phases (e.g., undo depends on command pattern)
- User feedback after Phase 1 and 2 recommended before continuing
- Performance testing critical after Phase 4 (caching)

---

## Future Enhancements (Beyond This Plan)

- Mobile app (separate project)
- Calendar sharing/collaboration
- Time zone support
- Print view
- CSV import/export
- Event conflicts with scheduling suggestions
- Calendar subscription (read-only remote calendars)
- Plugin system for extensions
- REST API for third-party integrations
