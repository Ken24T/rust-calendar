# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.4.3] - 2026-02-27

### Added
- Stage 1 S5 settings UI controls for Google Calendar read-only sync:
  - Add source form (name, private ICS URL, poll interval).
  - Per-source update/delete actions.
  - Manual “Sync now” action with sync result feedback.
  - Status display for last sync outcome and last error.

### Changed
- Added persistent settings dialog UI state so source drafts can be edited safely across frames.

## [1.4.2] - 2026-02-27

### Added
- Stage 1 S4 sync engine foundations:
  - New `CalendarSyncEngine` orchestration for per-source sync runs and batch sync across enabled sources.
  - Source-scoped upsert by (`source_id`, `external_uid`) using `event_sync_map` identity mapping.
  - Deletion reconciliation for mapped events not present in the latest feed payload.
  - Sync run result counters for created/updated/deleted/skipped events.

### Changed
- Extended mapping service with source listing support used by reconciliation (`list_by_source_id`).

## [1.4.1] - 2026-02-27

### Added
- Stage 1 S3 sync pipeline improvements:
  - New cross-platform ICS fetcher (`reqwest` + `rustls`) with timeout, retries, payload size guard, and URL redaction for logs.
  - Metadata-aware ICS import path that captures event `UID` and raw `LAST-MODIFIED` while preserving existing event import APIs.

### Changed
- Improved iCalendar datetime parsing to better handle UTC (`Z`) and `TZID` forms, reducing timezone interpretation issues across Windows and Linux.

## [1.4.0] - 2026-02-27

### Added
- Stage 1 Google Calendar read-only sync foundations:
  - New `calendar_sources` persistence model, validation, and CRUD service for managing multiple Google ICS feeds.
  - New `event_sync_map` model and table with unique (`source_id`, `external_uid`) mapping for deterministic external identity tracking.
  - New `EventSyncMapService` methods for mapping create/lookup/touch/delete operations.
- New schema coverage and service tests for `calendar_sources` and `event_sync_map` tables and mapping behaviour.

## [1.3.0] - 2026-02-27

### Changed
- TCTBP status workflow now includes explicit next-step recommendations (for example `handback`, `handoff`, `ship`, `abort`, or `none`) based on repo state.
- Test code was refactored to satisfy strict clippy checks by using struct initialisers with `..Default::default()` instead of field reassignment after default construction.

## [1.2.0] - 2026-02-27

### Removed
- **Compact mode** from countdown cards — the setting was wired through the UI, database, and service layers but never actually affected rendering. Removed the field from the data model, all SQL queries, the countdown settings dialog, the event editor dialog, and the service API. The database column is retained with its default value to avoid a migration.

## [1.0.21] - 2025-11-29

### Added
- **Undo/Redo System** - Full undo/redo support for event operations:
  - Undo/Redo items in Edit menu with dynamic descriptions
  - Ctrl+Z for undo, Ctrl+Y or Ctrl+Shift+Z for redo
  - Toast notifications showing what was undone/redone
  - Command pattern architecture for extensibility
  - Supports: Create Event, Update Event, Delete Event
  - History limit of 50 operations
  - 6 new unit tests for command system

### Changed
- Edit menu reorganized with Undo/Redo at top, separator before settings

## [1.0.16] - 2025-11-28

### Added
- **Context Menu Template Access** - Create events from templates directly in calendar views:
  - Right-click on empty space in Month, Week, or Day view to see "📋 From Template" submenu
  - Templates listed with hover tooltips showing title and duration
  - Clicking a template creates an event for that specific date/time

### Fixed
- **Keyboard Shortcuts** - View shortcuts (D, M, W, K) no longer interfere while typing:
  - Fixed issue where pressing 'd', 'm', 'w' etc. would change views while typing in dialogs
  - Shortcuts now properly detect when a text input has focus

## [1.0.15] - 2025-11-28

### Added
- **Event Templates** - Save and reuse common event configurations:
  - Templates submenu in Events menu with quick access to saved templates
  - "Manage Templates..." dialog for creating, editing, and deleting templates
  - Templates store: name, title, description, location, duration, category, and color
  - Click a template to instantly create a new event prefilled with template values
  - Database table `event_templates` for persistent storage

## [1.0.14] - 2025-11-28

### Added
- **Event Validation Enhancements** - Non-blocking warnings in the event dialog:
  - Overlap detection: Warns when the event overlaps with existing events
  - Distant past warning: Warns when creating events more than 5 years in the past
  - Warnings displayed in orange/amber (non-blocking - save still allowed)

### Changed
- Export dialog now uses calendar-style date picker matching the event dialog

## [1.0.13] - 2025-11-28

### Added
- **Export Events to iCalendar (.ics)** - Events menu now has Export submenu:
  - "Export All Events..." - Exports all calendar events to a single .ics file
  - "Export Date Range..." - Opens a dialog to select start/end dates for export
  - Quick select buttons for "This Month", "This Year", and "Last 30 Days"
  - Toast notifications for success/failure feedback
- New `export_dialog.rs` module for the date range picker dialog

### Changed
- Reorganized Events menu to group Import and Export operations

## [Unreleased]

### Planning Phase

#### Added
- Initial project structure and build configuration
- Comprehensive documentation:
  - PROJECT_PLAN.md - 12-week implementation roadmap
  - ARCHITECTURE.md - System design and patterns
  - MODULARITY.md - Code organization guidelines (max 300 lines per file)
  - TESTING.md - Testing strategy (>90% coverage requirement)
  - UI_SYSTEM.md - Complete UI specifications
  - COUNTDOWN_TIMER_FEATURE.md - Desktop countdown timer widget specs
  - MY_DAY_AND_RIBBON_FEATURES.md - My Day panel and multi-day ribbon specs
- Cargo.toml with all dependencies configured
- Database schema design (7 tables):
  - events - Event storage with recurrence rules
  - reminders - Configurable event reminders
  - settings - Application settings
  - ui_preferences - UI customization and layout preferences
  - column_widths - Per-view column width persistence
  - row_heights - Per-view row height persistence
  - countdown_timers - Desktop countdown timer widget state
- Test infrastructure:
  - Unit test examples (recurrence_frequency_tests.rs)
  - Property-based test examples (recurrence_properties.rs)
  - Test fixtures (mod.rs)
  - Benchmark examples (recurrence_bench.rs)
- Theme assets (light.toml, dark.toml)
- .gitignore configuration
- Dual licensing (MIT OR Apache-2.0)

#### Features (Planned)
- Multiple calendar views:
  - Day view - Detailed hourly schedule
  - Work week view - Monday through Friday
  - Full week view - Complete 7-day view
  - Month view - Traditional calendar grid
  - Quarter view - 3-month overview
  - Year view - Annual 12-month display
  - Agenda view - Linear event list
- My Day panel:
  - Sidebar showing selected day's events
  - Configurable position (left/right/hidden)
  - Adjustable width (180-400px)
  - Auto-updates with calendar navigation
- Multi-day event ribbon:
  - Horizontal strip for events spanning 2+ days
  - Multiple display modes (compact/expanded/auto)
  - Progress indicators for ongoing events
  - Keeps main calendar grid uncluttered
- Desktop countdown timer widgets:
  - Drag events to desktop to create countdown timers
  - Always-on-top, movable windows
  - Live countdown updates
  - Auto-dismiss when event starts (optional)
- Event management:
  - Single and repeating events
  - Recurrence patterns: Daily, Weekly, Fortnightly, Monthly, Quarterly, Yearly
  - All-day events
  - Event categories with color coding
- UI customization:
  - Adjustable column widths (drag to resize)
  - Customizable fonts (family, size, weight, style)
  - Resizable row heights
  - Configurable time granularity (15/30/60 minute intervals)
  - Adjustable default event duration (default: 45 minutes)
  - All preferences persist between sessions
- Reminder system:
  - Multiple reminders per event
  - Windows native notifications
  - Snooze functionality
- Drag-and-drop:
  - .ics file import
  - Event rescheduling
  - Event duration adjustment
  - Tear events to desktop for countdown timers
- Theme support:
  - Light and Dark modes
  - Customizable color schemes
  - Theme persistence

## [0.1.0] - 2025-11-06

### Added
- Initial project setup
- Git repository initialized
- Documentation framework established
- Project structure created

---

**Note**: This project is currently in the planning phase. Implementation will begin with Phase 1 (Foundation) following the 12-week roadmap outlined in PROJECT_PLAN.md.
