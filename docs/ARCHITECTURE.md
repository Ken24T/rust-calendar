# Architecture

This document describes the architecture of Rust Calendar as of v2.0.17.

## High-Level Overview

Rust Calendar is a desktop application built with egui/eframe (immediate-mode
GUI) and SQLite (via rusqlite, bundled). It follows a layered architecture with
strict separation between domain models, business logic, and presentation.

```text
┌──────────────────────────────────────────┐
│              UI Layer (egui)             │
│         src/ui_egui/                     │
│  Views · Dialogs · Event Editor · Theme  │
└────────────────┬─────────────────────────┘
                 │ calls
┌────────────────▼─────────────────────────┐
│           Service Layer                  │
│         src/services/                    │
│  Events · Settings · Themes · Backup     │
│  Countdown · iCal · PDF · Sync           │
└────────────────┬─────────────────────────┘
                 │ reads/writes
┌────────────────▼─────────────────────────┐
│           Database Layer                 │
│      src/services/database/              │
│        SQLite (rusqlite)                 │
└──────────────────────────────────────────┘

┌──────────────────────────────────────────┐
│           Domain Models                  │
│         src/models/                      │
│  Event · Recurrence · Settings · UI      │
│  Category · Template · Reminder          │
└──────────────────────────────────────────┘
```

## Layer Rules

1. **Models are UI-agnostic** — no egui types in `src/models/`.
2. **Services are UI-agnostic** — no egui types in `src/services/`.
3. **UI calls services, never the reverse.**
4. **All database access** goes through `src/services/database/`.
5. **Utils** (`src/utils/`) are shared helpers available to every layer.

## Module Layout

### Models (`src/models/`)

Pure data types with `#[derive(Clone, Debug, Serialize, Deserialize)]` where
appropriate. No side effects, no I/O.

- `event` — `Event` struct (title, start/end, all-day, category, colour,
  recurrence rule, exceptions)
- `recurrence` — `Frequency` enum (Daily, Weekly, Fortnightly, Monthly,
  Quarterly, Yearly) and recurrence-rule parsing
- `settings` — `Settings` struct (theme, format preferences, view config, card
  dimensions, sync delay)
- `ui` — `ViewConfig`, `ViewType` enum (Day, WorkWeek, Week, Month, Quarter)
- `category` — event category with colour
- `template` — reusable event template
- `reminder` — reminder model
- `calendar_source` — external calendar feed descriptor
- `event_sync_map` — external-to-local event ID mapping

### Services (`src/services/`)

Business logic that operates on models and the database. Each service borrows a
`&rusqlite::Connection` and exposes domain operations.

- `database/` — `Database` struct (wraps `rusqlite::Connection`), schema
  creation, migrations
- `event/` — `EventService` with `crud.rs`, `queries.rs`, and `recurrence/`
  (expansion into concrete occurrences: daily, weekly, monthly, yearly parsers)
- `settings/` — load/save `Settings` to/from the database
- `theme/` — TOML-based theme loading from `assets/themes/`
- `backup/` — database backup and restore
- `countdown/` — countdown timer state, persistence, layout, visuals,
  notifications, sync, category management
- `calendar_sync/` — external ICS feed sync engine, fetcher, mapping, scheduler
- `icalendar/` — iCalendar import/export (`.ics` files)
- `pdf/` — PDF calendar export
- `notification/` — cross-platform desktop notifications
- `reminder/` — reminder scheduling
- `category/` — category CRUD
- `template/` — event template CRUD

### UI (`src/ui_egui/`)

The presentation layer. Everything here is egui-specific.

- `app.rs` — `CalendarApp` struct implementing `eframe::App`
- `app/` — app shell, split by concern:
  - `lifecycle.rs` — `new()`, `on_exit()`
  - `context.rs` — `AppContext` (shared DB reference + services)
  - `state.rs` — `AppState` (current date, view, selected events)
  - `menu.rs`, `menu_export.rs`, `menu_help.rs` — menu bar
  - `navigation.rs` — date/view navigation
  - `sidebar.rs` — left sidebar
  - `shortcuts.rs` — keyboard shortcuts
  - `geometry.rs` — window geometry persistence
  - `status_bar.rs`, `toast.rs`, `confirm.rs`
  - `countdown/` — countdown UI state, rendering, container layout
  - `dialogs/` — app-level dialog triggers
  - `views/` — view dispatch and date picker
- `views/` — calendar view implementations:
  - `day_view.rs`, `week_view.rs`, `workweek_view.rs`, `month_view.rs`,
    `quarter_view.rs`
  - `time_grid.rs`, `time_grid_cell.rs` — shared time axis rendering
  - `event_helpers.rs`, `event_rendering.rs` — event layout calculations
  - `palette.rs` — event colour scheme
- `event_dialog/` — event create/edit dialog (state, rendering, recurrence,
  widgets)
- `dialogs/` — modal dialogs (backup, categories, countdown categories, export,
  search, templates, themes)
- `commands/` — `UndoManager` (undo/redo support)
- `drag.rs` — drag-and-drop handling
- `resize.rs` — event resize interaction
- `theme.rs` — `CalendarTheme` struct
- `settings_dialog.rs` — settings dialog

### Utils (`src/utils/`)

- `date/` — date arithmetic helpers shared across layers

## Application Lifecycle

```text
main.rs
  │  #![windows_subsystem = "windows"]   ← hides console on Windows release
  │  env_logger::init()
  │  eframe::run_native("Rust Calendar", options,
  │      |cc| Ok(Box::new(CalendarApp::new(cc))))
  │
  ▼
CalendarApp::new(cc)                     [ui_egui/app/lifecycle.rs]
  ├─ Database::new(path)                 ← opens/creates SQLite, runs schema
  ├─ Load Settings, CalendarTheme
  ├─ Create AppContext (&'static Database + services)
  ├─ Initialise state: AppState, EventDialogState, CountdownUiState
  ├─ Create CalendarSyncScheduler
  └─ Return CalendarApp

CalendarApp::update(ctx, frame)          [ui_egui/app.rs]
  ├─ Process keyboard shortcuts
  ├─ Render menu bar
  ├─ Render sidebar
  ├─ Render current view (Day/Week/Month/Quarter)
  ├─ Render open dialogs (event editor, settings, etc.)
  ├─ Render countdown windows/container
  └─ Process toast notifications

CalendarApp::on_exit(gl)                 [ui_egui/app/lifecycle.rs]
  ├─ Save settings to database
  ├─ Save window geometry
  └─ Clean up GL resources
```

## Database

SQLite with foreign keys enabled. Schema is created on first run and migrated
forward as needed.

### Tables

- `settings` — singleton (id=1) application preferences
- `events` — calendar events
- `categories` — event categories (seeded with defaults)
- `event_templates` — reusable event templates
- `custom_themes` — user-created theme definitions
- `calendar_sources` — external calendar feed URLs
- `event_sync_map` — external-to-local event ID mapping for sync
- `countdown_cards` — countdown timer cards (linked to events or standalone)
- `countdown_settings` — countdown global settings (visual defaults, display mode)
- `countdown_categories` — countdown card categories with per-container visual
  defaults, display order, collapse/sort state, and default card dimensions

### Access Pattern

All database access flows through `services::database::Database`. The database
connection is created once at startup, leaked as `&'static`, and shared across
services via `AppContext`. Services borrow the connection for individual
operations — no connection pooling or threading is needed for this single-user
desktop application.

## Cross-Platform Support

- **Linux**: primary development platform. Notifications via `notify-rust`.
  Desktop integration via `.desktop` file in `packaging/`.
- **Windows**: full support. Console hidden via `windows_subsystem`. Toast
  notifications via the `windows` crate. Build requires Visual Studio Build
  Tools.
- **macOS**: untested but should compile — no macOS-specific code exists.

Platform-specific code uses `#[cfg(target_os = "...")]` guards. File paths use
`std::path::Path` and the `directories` crate for XDG/AppData resolution.

## Theming

Themes are defined in TOML files under `assets/themes/` (dark and light
presets). Users can also create custom themes via the theme creator dialog; these
are stored in the `custom_themes` database table. The `CalendarTheme` struct
controls all colours used by the UI layer.

## Error Handling

- Application-level errors use `anyhow::Result`
- Library-level error types use `thiserror`
- Runtime diagnostics via `log` macros (`log::info!`, `log::warn!`,
  `log::error!`)
- Database initialisation failures fall back to in-memory SQLite

## Codebase Metrics

As of v2.0.17:

- 144 source files, ~35,000 lines of Rust
- All files under 550 lines (target: 300; see
  [MODULARITY.md](MODULARITY.md) for guidelines)
- 298 unit tests + 3 integration tests + 1 doc-test
- Zero clippy warnings
