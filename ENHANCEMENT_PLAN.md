# Rust Calendar Enhancement Plan

> **Status: Mostly Complete** — Phases 1–3 are implemented and shipped.
> Phases 4–5 contain remaining items (e.g. undo/redo). This document is
> retained as a historical record of the feature planning process.

**Created:** November 19, 2025\
**Last Updated:** February 28, 2026\
**Phases 1–3:** Complete\
**Phases 4–5:** Partial — remaining items are tracked for future work

## Overview

This document outlines the implementation plan for 20 improvements to the rust-calendar application. Features are organized into 5 phases based on priority, dependencies, and implementation complexity.

---

## Phase 1: Quick Wins & Safety Features (Days 1-2)

**Goal:** Implement high-impact features with minimal complexity that improve safety and usability.

### 1. Database Backup System ✅

**Status:** ✅ COMPLETE  
**Priority:** Highest - Prevents data loss  
**Estimated Time:** 3-4 hours  
**Branch:** `feat/database-backup`

**What was implemented:**
- ✅ `BackupService` with create, restore, list, delete methods
- ✅ Auto-backup on startup (keeps last 5 backups)
- ✅ File menu: "Backup Database..." (Ctrl+B), "Manage Backups..."
- ✅ Backup Manager dialog with restore/delete actions
- ✅ Backups stored in `%AppData%/rust-calendar/backups/`

**Files Created/Modified:**
- `src/services/backup/mod.rs` - BackupService implementation ✅
- `src/ui_egui/dialogs/backup_manager.rs` - Backup management dialog ✅
- `src/ui_egui/app/menu.rs` - File menu items ✅

---

### 2. Keyboard Shortcuts ✅

**Status:** ✅ COMPLETE  
**Priority:** High - Improves efficiency  
**Estimated Time:** 2-3 hours  
**Branch:** `feat/keyboard-shortcuts`

**Shortcuts Implemented:**
- `Ctrl+N` - New Event (open event dialog) ✅
- `Ctrl+T` - Today (navigate to current date) ✅
- `Ctrl+S` - Settings ✅
- `Ctrl+B` - Backup Database ✅
- `Arrow Left/Right` - Navigate days/weeks/months ✅
- `Arrow Up/Down` - Navigate weeks/months (contextual) ✅
- `Escape` - Close dialogs ✅

**Implementation:**
- Extended keyboard handling in `src/ui_egui/app.rs::update()` ✅
- Used pattern: `ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::N))` ✅
- Added shortcut hints to menu labels: "New Event    Ctrl+N" ✅
- Documented shortcuts in `README.md` ✅

**Files Modified:**
- `src/ui_egui/app.rs` - Added keyboard handlers in `update()` method ✅
- Menu button labels - Appended shortcut text ✅
- `README.md` - Added keyboard shortcuts section ✅

**Testing:**
- ✅ Each shortcut performs correct action
- ✅ Modifiers (Ctrl) work correctly
- ✅ Shortcuts work when dialogs are closed
- ✅ Arrow keys navigate contextually based on view
- ✅ Escape closes dialogs in priority order

---

### 3. Export Menu Integration ✅

**Status:** ✅ COMPLETE  
**Priority:** High - Completes existing feature  
**Estimated Time:** 2 hours  
**Branch:** `feat/ui-improvements`

**Implementation:**
- Added File menu items:
  - "Export All Events..." → File dialog, save all events as .ics ✅
  - "Export Date Range..." → Date picker dialog, export subset ✅
- Used existing `ICalendarService::export_events_to_file()` ✅
- Used `rfd::FileDialog::new().add_filter("iCalendar", &["ics"]).save_file()` ✅
- Show success toast notification after export ✅
- Quick select buttons: "This Month", "This Year", "Last 30 Days" ✅

**Files Modified:**
- `src/ui_egui/app/menu.rs` - Added Export Events submenu with export functions
- `src/ui_egui/app/state.rs` - Added ExportRangeDialogState
- `src/ui_egui/app/dialogs/mod.rs` - Wired up export dialog rendering
- Created `src/ui_egui/dialogs/export_dialog.rs` - Date range picker dialog

---

### 4. Event Validation Enhancements ✅

**Status:** ✅ COMPLETE  
**Priority:** Medium - Improves data quality  
**Estimated Time:** 2 hours  
**Branch:** `feat/validation-improvements`

**Validation Rules Implemented:**
- ✅ Title not empty
- ✅ End after start
- ✅ Color hex format validation
- ✅ Recurrence rule parsing validation via rrule crate
- ✅ All-day event date boundary handling

**Files Modified:**
- `src/models/event/mod.rs` - Event::validate() method ✅
- `src/ui_egui/event_dialog/state.rs` - Validation display ✅

---

## Phase 2: Search & Organization (Days 3-4)

**Goal:** Enable users to find and organize events efficiently.

### 5. Event Search ✅

**Status:** ✅ COMPLETE  
**Priority:** High - Major productivity improvement  
**Estimated Time:** 4 hours  
**Branch:** `feat/event-search`

**What was implemented:**
- ✅ Search dialog with Ctrl+F shortcut
- ✅ `EventService::search()` method with case-insensitive search
- ✅ Search across title, description, location, category
- ✅ Results grouped by date with click-to-navigate
- ✅ Edit button on search results
- ✅ Escape to close dialog

**Files Created/Modified:**
- `src/ui_egui/dialogs/search_dialog.rs` - Search dialog UI ✅
- `src/services/event/queries.rs` - EventService::search() ✅
- `src/ui_egui/app/shortcuts.rs` - Ctrl+F binding ✅

---

### 6. Category Management & Filtering ✅

**Status:** ✅ COMPLETE  
**Priority:** High - Organizational feature  
**Estimated Time:** 5 hours  
**Branch:** `feat/category-management`

**What was implemented:**
- ✅ Categories table with default categories (Work, Personal, Birthday, Holiday, Meeting, Deadline)
- ✅ CategoryService with full CRUD operations
- ✅ Edit → Manage Categories dialog for category management
- ✅ View → Filter by Category submenu
- ✅ Event dialog category dropdown with icons
- ✅ Category filtering in all calendar views

**Files Created/Modified:**
- `src/models/category/mod.rs` - Category model ✅
- `src/services/category/mod.rs` - CategoryService ✅
- `src/services/database/schema.rs` - Categories table ✅
- `src/ui_egui/dialogs/category_manager.rs` - Category manager dialog ✅
- `src/ui_egui/app/menu.rs` - Filter by Category submenu ✅
- `src/ui_egui/event_dialog/render.rs` - Category dropdown ✅

---

### 7. Event Templates ✅

**Status:** ✅ COMPLETE (v1.0.15)
**Priority:** Medium - Time-saving feature  
**Estimated Time:** 4 hours  
**Branch:** `feat/event-templates`

**What was implemented:**
- ✅ EventTemplate model with validation (`src/models/template/mod.rs`)
- ✅ TemplateService for CRUD operations (`src/services/template/mod.rs`)
- ✅ Database table `event_templates` with all fields
- ✅ Events → Templates menu with:
  - Quick-create submenu listing all templates
  - "Manage Templates..." dialog for CRUD
- ✅ Template Manager dialog with create/edit/delete functionality
- ✅ Context menu "📋 From Template" submenu in:
  - Month view context menu
  - Week view context menu
  - Work week view context menu
  - Day view context menu
- ✅ Keyboard shortcut fix: D/M/W/K keys no longer captured while typing in dialogs

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

**Files Created/Modified:**
- `src/models/template/mod.rs` - EventTemplate model with validation
- `src/services/template/mod.rs` - TemplateService for CRUD
- `src/ui_egui/dialogs/template_manager.rs` - Template management dialog
- `src/ui_egui/app/menu.rs` - Templates submenu, create_event_from_template methods
- `src/ui_egui/app/state.rs` - TemplateManagerState
- `src/ui_egui/views/month_view.rs` - Context menu template support
- `src/ui_egui/views/week_shared.rs` - Context menu template support
- `src/ui_egui/app/shortcuts.rs` - Fixed keyboard input detection

**Testing:**
- ✅ Create template, verify stored correctly
- ✅ Create event from template via menu
- ✅ Create event from template via context menu
- ✅ Edit template, verify changes persist
- ✅ Delete template, verify removed
- ✅ 175 tests passing

---

### 8. Week Numbers ✅

**Status:** ✅ COMPLETE  
**Priority:** Low - Nice to have  
**Estimated Time:** 1 hour  
**Branch:** `feat/week-numbers`

**What was implemented:**
- ✅ `show_week_numbers` setting in Settings model and database
- ✅ Toggle in Settings dialog
- ✅ ISO week numbers displayed in Week view header
- ✅ ISO week numbers displayed in Work Week view header
- ✅ ISO week numbers displayed in Month view sidebar

**Files Modified:**
- `src/models/settings/mod.rs` - show_week_numbers field ✅
- `src/ui_egui/settings_dialog.rs` - Checkbox ✅
- `src/ui_egui/views/week_view.rs` - Week number in header ✅
- `src/ui_egui/views/workweek_view.rs` - Week number in header ✅
- `src/ui_egui/views/month_view.rs` - Week numbers sidebar ✅

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

### 11. Drag-to-Resize Events ✅

**Status:** ✅ COMPLETE (v1.0.26)
**Priority:** Medium - Intuitive editing  
**Estimated Time:** 4 hours  
**Branch:** `feat/ux-improvements-20251129`

**What was implemented:**
- ✅ `ResizeManager` with handle detection and resize context
- ✅ Top/bottom edge detection for timed events
- ✅ Left/right edge detection for ribbon (multi-day) events
- ✅ Visual resize handles on hover
- ✅ Cursor changes (↕ for vertical, ↔ for horizontal)
- ✅ Snap to 15-minute intervals
- ✅ Only non-recurring, non-past events can be resized
- ✅ Undo support via command pattern

**Files Created/Modified:**
- `src/ui_egui/resize.rs` - ResizeManager, ResizeContext, HandleRects ✅
- `src/ui_egui/views/day_view.rs` - Resize interactions ✅
- `src/ui_egui/views/week_view.rs` - Resize interactions ✅
- `src/ui_egui/views/workweek_view.rs` - Resize interactions ✅
- `src/ui_egui/views/week_shared.rs` - Ribbon resize for multi-day events ✅

**Testing:**
- ✅ Drag top edge to change start time
- ✅ Drag bottom edge to change end time
- ✅ Drag left/right edges for multi-day events
- ✅ Verify snapping to 15-minute intervals
- ✅ Recurring events cannot be resized
- ✅ Past events cannot be resized

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

### 20. Dark/Light Mode Auto-Switch ✅

**Status:** ✅ COMPLETE (Follow System mode implemented)  
**Priority:** Low - Convenience feature  
**Estimated Time:** 3 hours  
**Branch:** `feat/auto-theme-switch`

**What was implemented:**
- ✅ `use_system_theme` setting in Settings model
- ✅ "Use system theme" checkbox in Settings dialog
- ✅ `dark-light` crate integration for OS theme detection
- ✅ Automatic Light/Dark theme switch based on OS setting

**Note:** Scheduled mode (switch at specific times) not implemented - only Follow System mode.

**Files Modified:**
- `src/models/settings/mod.rs` - use_system_theme field ✅
- `src/ui_egui/settings_dialog.rs` - Checkbox ✅
- `src/ui_egui/app/lifecycle.rs` - dark_light::detect() integration ✅

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
- [x] Keyboard Shortcuts
- [x] Export Menu Integration
- [x] Event Validation Enhancements

### Phase 2 Status
- [x] Event Search
- [x] Category Management & Filtering
- [x] Event Templates
- [x] Week Numbers

### Phase 3 Status
- [x] Undo/Redo System
- [ ] Multi-Select Events
- [x] Drag-to-Resize Events
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
- [x] Dark/Light Mode Auto-Switch (Follow System mode)

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
