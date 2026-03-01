# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.3.3] - 2026-03-02

### Fixed

- System tray menu items (Show Calendar, Exit) and left-click restore now work
  reliably across multiple hide/show cycles on Windows. Previously, the second
  cycle would fail because destroying and re-creating the tray icon produced
  event handlers whose `ctx.request_repaint()` could no longer wake the winit
  event loop.
- Tray icon is now kept alive across hide/show cycles; only destroyed when the
  setting is disabled or the application exits.
- Added self-sustaining `request_repaint_after(200ms)` loop while hidden to tray,
  ensuring `poll_tray_events()` always runs even when the window is off-screen.

## [2.2.1] - 2026-03-01

### Changed

- Template category field is now a dropdown (ComboBox) with colour swatches,
  matching the event dialog's category picker.
- "Manage Templates..." moved from Events > Templates submenu to the Edit menu
  for consistency with other management dialogs.
- Replaced all `→` (U+2192) characters in UI text with ASCII-safe alternatives
  to fix missing-glyph rendering (□) on systems without arrow glyphs.

## [2.2.0] - 2026-03-01

### Added

- Apply/Cancel semantics for countdown card settings: changes preview live on
  the card but must be confirmed with "Apply" to persist. "Cancel" or closing
  the window reverts to the pre-edit state.
- `CardSettingsSnapshot` struct captures card visuals, title, comment, and date
  when the settings dialog opens, enabling clean revert on cancel.
- Bundled NotoSansSymbols2 fallback font subset (1.6 KB) for UI triangle/arrow
  glyphs (⏵ ⏷ ▶ ▼ etc.) that were missing on some Linux systems.

### Changed

- Renamed "Save" button to "Apply" in card settings to clarify commit semantics.
- Container management moved to Edit menu; removed from View > Countdown Cards
  submenu.
- Added separator in Edit menu between Settings and theme/category management.

### Fixed

- Container default settings (colours, fonts, dimensions) are now honoured when
  creating new cards. Previously, `create_card_in_category()` always used global
  defaults instead of the category's own defaults when `use_global_defaults` was
  disabled.

## [2.1.8] - 2026-03-01

### Added

- Container card defaults editor in the Category Manager dialog. When editing a
  category, expanding the "Card Defaults" section allows configuring:
  - "Inherit global defaults" toggle (uses global visual settings when enabled)
  - Default card width and height sliders (60–400 px)
  - Per-container colour defaults: title background, title text, body background,
    and days text — each with an egui colour picker
  - Per-container font size defaults: title size (10–48 pt) and days number size
    (32–220 pt)
- Visual settings are disabled when "Inherit global defaults" is enabled,
  providing clear feedback about the three-tier inheritance model.
- All container defaults are persisted to the SQLite database on save.

## [2.1.7] - 2026-03-01

### Added

- Category submenu on context-menu "Create Countdown" items across all calendar
  views (day, week, work-week, month). When multiple countdown categories
  exist, right-clicking an event and choosing "⏱ Create Countdown" now shows a
  submenu listing every category so the user can pick the target container.
  Single-category setups retain the simple one-click button.
- `CountdownCategoriesCache` per-frame cache stored via egui temp data, avoiding
  the need to thread categories through multiple rendering layers.
- `render_countdown_menu_items` shared helper for rendering the countdown
  context menu item(s) consistently.

## [2.1.6] - 2026-03-01

### Added

- Category/container selector when creating countdown cards from the event dialog.
  When "Create countdown card after saving" is checked and multiple categories
  exist, a "Container" dropdown appears letting the user choose which category
  the new card is assigned to (defaults to General).
- `category_id` field on `CountdownRequest` so the creation flow can target a
  specific category instead of always using the default.
- `countdown_category_id` field on `EventDialogState` to track the user's
  container selection.

## [2.1.5] - 2026-03-01

### Added

- Container collapse/expand: click the ▶/▼ toggle in the category container
  header to collapse or expand, hiding all cards and leaving just the header bar.
  Collapse state persists in the database.
- Sort mode per category: toggle between Date (📅, cards sorted by start date)
  and Manual (✋, user-defined drag order) via the header toolbar. Sort mode
  persists in the database.
- Quick-add button (➕) in category container headers: creates a new standalone
  countdown card seven days from now, assigned to that category.
- Card count badge in collapsed and expanded container headers.
- `ContainerSortMode` enum (`Date`, `Manual`) on `CountdownCategory` model.
- `is_collapsed` and `sort_mode` database columns with safe migrations.
- `toggle_category_collapsed()`, `set_category_sort_mode()` service methods.

## [2.1.4] - 2026-03-01

### Added

- Cross-category drag-and-drop in Category Containers mode.
  - When dragging a card, a drop-zone strip appears at the bottom of the container
    showing other categories as coloured targets.
  - Dropping on a target button moves the card to that category.
  - Highlighted feedback when the pointer hovers over a drop-zone target.
  - `drop_zone_hit_test()` helper for deterministic hit detection, shared between
    drag-end processing and visual rendering.
  - Single-container mode unaffected (drop zones only appear in CategoryContainers mode).

## [2.1.3] - 2026-03-01

### Added

- Three-tier visual inheritance: Global → Category → Card.
  - Cards with `use_default_*` flags now inherit from their category's visual defaults
    (or the global defaults when the category has `use_global_defaults = true`).
  - `effective_visual_defaults_for(category_id)` and `effective_visual_defaults_map()`
    service methods resolve the inheritance chain.
  - All three display modes (Individual Windows, Container, Category Containers) now
    honour the three-tier defaults.
  - Individual windows mode now correctly resolves `use_default_*` flags (previously
    bypassed defaults entirely).
- Three new unit tests covering the visual inheritance resolution.

## [2.1.2] - 2026-03-01

### Added

- Category Containers display mode — renders one container window per countdown category.
  - Each category gets its own resizable, draggable container with independent geometry persistence.
  - Per-category layout and drag state tracked independently.
  - Closing any category container switches back to Individual Windows mode.
  - Container window titles show the category name (e.g. "⏱ General").
- `render_container_window` now accepts custom `window_title` and `viewport_id_suffix` parameters for reuse across display modes.
- `update_category_container_geometry` service method for persisting per-category container geometry.

## [2.1.1] - 2026-03-01

### Added

- "Move to category" context menu on countdown cards (individual windows and container modes).
  - Submenu lists all categories; current category shown with checkmark.
  - Only appears when more than one category exists.
  - Card's category is updated immediately and persisted on next save cycle.

## [2.1.0] - 2026-02-28

### Added

- Countdown category manager dialog for creating, editing, and deleting countdown card categories.
  - Two-column layout: scrollable category list (with card counts) and editor panel.
  - Create/rename/reorder categories with duplicate name validation.
  - Delete with confirmation; cards reassigned to the default "General" category.
  - Accessible from View → Countdown Cards → Manage Categories menu.
- Data layer for countdown card categories (from prior commit):
  - `CountdownCategory` model with visual defaults, container geometry, and display ordering.
  - `countdown_categories` database table with "General" seed row.
  - `category_id` column on `countdown_cards` table.
  - Full repository CRUD (`repository_categories.rs`) and service methods.
  - `CategoryContainers` display mode variant.

## [2.0.17] - 2026-02-28

### Refactored

- Extracted single time cell rendering from `views/time_grid.rs` into `views/time_grid_cell.rs` (~490 lines), reducing `time_grid.rs` from 672 to 191 lines.
  - Moved `render_time_cell` and `TimeCellConfig` with all drag/drop, resize, tooltip, and context menu logic.
- Extracted individual view rendering methods from `app/views/mod.rs` into `app/views/view_rendering.rs` (~310 lines), reducing `mod.rs` from 574 to 270 lines.
  - Moved `render_day_view`, `render_week_view`, `render_workweek_view`, `render_month_view`, `handle_timed_view_result`, and `handle_delete_confirm_request`.

## [2.0.16] - 2026-02-28

### Refactored

- Extracted time slot rendering from `views/day_view.rs` into `views/day_time_slot.rs` (~490 lines), reducing `day_view.rs` from 733 to 305 lines.
  - Moved `render_time_slot` with all drag/drop, resize handle, context menu, tooltip, and event interaction logic.
- Extracted event helper functions from `views/mod.rs` into `views/event_helpers.rs` (~520 lines), reducing `mod.rs` from 651 to 83 lines.
  - Moved `event_time_segment_for_date`, `countdown_menu_state`, `filter_events_by_category`, `is_ribbon_event`, `event_display_end_date`, `event_covers_date`, `build_ribbon_lanes`, `load_synced_event_ids`, `is_synced_event`, `filter_events_by_sync_scope`, `CountdownRequest::from_event` impl, and all associated tests.

## [2.0.15] - 2026-02-28

### Refactored

- Extracted validation and persistence methods from `event_dialog/state.rs` into `event_dialog/state_persistence.rs` (234 lines), reducing `state.rs` from 681 to 385 lines.
  - Moved `save`, `start_end_datetimes`, `build_rrule`, `validate`, `check_warnings`, and `to_event` methods.
- Extracted day cell rendering from `views/month_view.rs` into `views/month_day_cell.rs` (404 lines), reducing `month_view.rs` from 770 to 323 lines.
  - Moved `render_day_cell` and `truncate_single_line_to_width` methods.

## [2.0.14] - 2026-02-28

### Refactored

- Extracted countdown schema functions from `database/schema.rs` into `database/schema_countdown.rs` (253 lines), reducing `schema.rs` from 723 to 475 lines.
  - Moved `create_countdown_tables`, `run_countdown_migrations`, and `migrate_use_default_flags`.
- Extracted global settings operations from `countdown/repository.rs` into `countdown/repository_settings.rs` (291 lines), reducing `repository.rs` from 702 to 425 lines.
  - Moved `CountdownGlobalSettings` struct + `Default` impl, `get_global_settings`, `update_global_settings`, and `update_next_card_id`.

## [2.0.13] - 2026-02-28

### Refactored

- Extracted 8 built-in theme preset constructors from `theme.rs` into `theme_presets.rs` (193 lines), reducing `theme.rs` from 674 to 411 lines.
  - Moved `light()`, `dark()`, `solarized_light()`, `solarized_dark()`, `nord()`, `dracula()`, `high_contrast()`, `sepia()`.
- Extracted theme dialog and creator handling from `app/dialogs/mod.rs` into `dialogs/theme_handling.rs` (206 lines), reducing `dialogs/mod.rs` from 585 to 352 lines.
  - Moved `render_theme_dialog` (all `ThemeDialogAction` variants) and `render_theme_creator`.

## [2.0.12] - 2026-02-28

### Refactored

- Extracted drawing functions from `resize.rs` into `resize_drawing.rs` (227 lines), reducing `resize.rs` from 673 to 470 lines.
  - Moved `draw_handles` and `draw_resize_preview` with re-exports for transparent consumer access.
- Extracted settings-related methods from countdown `state.rs` into `state_settings.rs` (258 lines), reducing `state.rs` from 845 to 544 lines.
  - Moved `render_settings_dialogs`, `apply_settings_command`, and `default_settings_geometry_for`.

## [2.0.11] - 2026-02-28

### Refactored

- Extracted time grid context menu into `time_grid_context_menu.rs` (277 lines), reducing `time_grid.rs` from 783 to 620 lines.
  - Split popup body into focused helpers: `render_event_menu`, `render_delete_buttons`, `render_export_button`, `render_empty_slot_menu`.
- Extracted date picker popup into `views/date_picker.rs` (173 lines), reducing `app/views/mod.rs` from 782 to 526 lines.
  - Split into `render_date_picker_header` and `render_date_picker_grid` helper methods.
  - Moved `shift_month` and `days_in_month` helper functions.
- DRYed triplicated view-result handling in day/week/workweek views into a single `handle_timed_view_result` helper method (~90 lines eliminated).

## [2.0.10] - 2026-02-28

### Removed

- Deleted dead test infrastructure (4 files, 364 lines):
  - `tests/fixtures/mod.rs` — mock types never imported by any test.
  - `tests/property/recurrence_properties.rs` — placeholder testing mock functions, not real code.
  - `tests/unit/models/recurrence_frequency_tests.rs` — testing a local `Frequency` copy, not the real model.
  - None of these files were compiled or executed by `cargo test` (test count unchanged at 298+3+1).

## [2.0.9] - 2026-02-28

### Removed

- Deleted legacy `src/ui/` directory (29 files, 5,332 lines of dead code).
  - Module was commented out in `lib.rs`, not compiled, and not referenced by any active code.
  - Removed stale reference from copilot instructions.

## [2.0.8] - 2026-02-28

### Refactored

- Extracted menu bar into focused modules, reducing `menu.rs` from 858 to 524 lines:
  - `menu_export.rs` (237 lines) — PDF and ICS export functions (month, week, all events, filtered, date range).
  - `menu_help.rs` (106 lines) — Help menu and About dialog.
- Removed unused imports from menu.rs after extraction.

## [2.0.7] - 2026-02-28

### Refactored

- Extracted event dialog rendering into focused modules, reducing `render.rs` from 872 to 437 lines:
  - `render_date_time.rs` (202 lines) — date/time pickers, all-day toggle, quick date buttons.
  - `render_recurrence.rs` (234 lines) — frequency, interval, pattern, BYDAY toggles, weekday shortcuts, end condition.
- Added shared layout helpers to `widgets.rs`: `labeled_row`, `indented_row`, `render_form_label`, `FORM_LABEL_WIDTH`.
- Replaced `hex_to_color32` with existing `parse_hex_color` from widgets module.

## [2.0.6] - 2026-02-28

### Refactored

- Extracted calendar sync settings into focused module, reducing `settings_dialog.rs` from 880 to 560 lines:
  - `settings_calendar_sync.rs` (364 lines) — Google Calendar ICS source management (add/edit/delete/sync).
- Moved `CalendarSourceDraft`, sync polling, and sync section rendering to dedicated module.

## [2.0.5] - 2026-02-28

### Refactored

- Extracted month view context menu into focused module, reducing `month_view.rs` from 992 to 769 lines:
  - `month_context_menu.rs` (303 lines) — day-cell context menu popup (edit, delete, countdown, templates).
- Replaced duplicate `parse_color` in month view with shared version from `week_shared`.
- Removed duplicate `countdown_request_for_month_event` method.

## [2.0.4] - 2026-02-28

### Refactored

- Extracted day view UI into focused modules, reducing `day_view.rs` from 1103 to 733 lines:
  - `day_event_rendering.rs` (144 lines) — event block and continuation bar painting in day-view time slots.
  - `day_context_menu.rs` (235 lines) — slot context menu popup (edit, delete, countdown, export, templates).

## [2.0.3] - 2026-02-28

### Refactored

- Extracted week-shared UI into focused modules, reducing `week_shared.rs` from 1499 to 432 lines:
  - `event_rendering.rs` (222 lines) — parse_color, individual event block painting, continuation bars, tooltips.
  - `time_grid.rs` (783 lines) — interactive time-slot grid, cell rendering, drag/drop, resize, context menus.
- All items re-exported from `week_shared` for API compatibility.

## [2.0.2] - 2026-02-28

### Refactored

- Extracted container UI into focused modules, reducing `container.rs` from 1305 to 462 lines:
  - `card_rendering.rs` (302 lines) — individual card rendering, tooltip formatting, colour calculations.
  - `container_layout.rs` (427 lines) — layout computation, drag-and-drop state, insertion indicators, 11 unit tests.

## [2.0.1] - 2026-02-28

### Refactored

- Extracted countdown service into focused modules, reducing `service.rs` from 1472 to 346 lines:
  - `palette.rs` (159 lines) — event-derived colour palette for countdown cards.
  - `visuals.rs` (266 lines) — per-card and default visual setters.
  - `storage.rs` (234 lines) — JSON snapshot and SQLite persistence.
  - `sync.rs` (177 lines) — event synchronisation to countdown cards.
  - `notifications.rs` (129 lines) — notification triggers and auto-dismiss logic.
  - `layout.rs` (270 lines) — geometry, display mode, ordering, and position management.

## [2.0.0] - 2026-02-28

### Summary

- Major release consolidating all bug fixes from the fix/bug-fixes branch.

### Fixed

- All fixes from v1.5.0 and v1.5.1 (ribbon event rendering, month view duplicates, all-day event normalisation).

## [1.5.1] - 2026-02-28

### Fixed

- Fixed ribbon events not filling full column cell width in week and workweek views — changed column alignment to left-aligned and made inner margins directional so multi-day ribbons extend edge-to-edge across adjacent day columns.
- Fixed month view showing all-day events on an extra day due to using raw end date instead of `event_display_end_date()` exclusive-end convention.

## [1.5.0] - 2026-02-28

### Fixed

- Fixed ribbon event span bug where single-day all-day events appeared to span across extra days in week and workweek views.
- Added directional corner rounding to ribbon events: first day rounds left, last day rounds right, middle days flat — making multi-day spans visually distinct from adjacent single-day events.
- Normalised all-day event storage to use midnight times and iCal exclusive-end convention, ensuring consistent span calculation across locally created and synced events.
- Fixed event dialog round-trip: loading an all-day event and re-saving no longer adds an extra day to the end date.
- Fixed ribbon resize for all-day events to correctly apply exclusive-end convention.
- Added one-shot database migration to normalise existing all-day events with non-midnight times.

## [1.4.14] - 2026-02-28

### Fixed

- Removed redundant `source_status_message` assignment in settings dialog that triggered a clippy `unused_assignments` warning.
- Fixed CHANGELOG markdown formatting (blank lines around headings and lists) to satisfy MD022/MD032 lint rules.

### Added

- Added `.markdownlint.json` configuration with `siblings_only` for MD024 to support Keep a Changelog duplicate headings.

## [1.4.13] - 2026-02-27

### Changed

- Constrained month-view event chip titles to a single line with width-aware truncation to prevent text overflow across cells.
- Improved month-view right-click interaction so event context menus reliably target the clicked event chip.
- Normalised countdown creation timing for non-recurring multi-day events by resolving canonical event start/end from persistence at countdown-consumption time, ensuring countdowns begin from the first day regardless of where the event was clicked in month/week/workweek views.

## [1.4.12] - 2026-02-27

### Added

- Added a configurable Google sync startup delay setting (default 15 minutes) in Settings, persisted in the database and applied when creating the sync scheduler on app launch.

### Changed

- Extended synced-event filtering to support source-scoped behaviour across day/week/workweek/month views.
- Updated filtering semantics so selecting a source with synced-only disabled shows that source's synced events plus local non-synced events (for example, Den + local).
- Added source-scoped synced ID lookup support in sync mapping services to back view-level filtering and lock/read-only guard behaviour.

## [1.4.11] - 2026-02-27

### Changed

- Unified countdown creation behaviour across day/week/workweek/month/search event surfaces via shared countdown menu-state logic and request handling.
- Added synced-event metadata enrichment for countdown cards by composing description, location, and synced source labels through the existing countdown request pipeline.
- Added one-click countdown creation in Search dialog flows and made synced search results countdown-first (read-only edit disabled, countdown action surfaced directly).
- Extended sync mapping lookups to resolve source names by local event for consistent synced countdown labelling.

## [1.4.10] - 2026-02-27

### Changed

- Reworked week/workweek ribbon layout into stable lane-based rendering so all-day event rows stay consistently aligned across day columns.
- Fixed week view column width calculation to account for scrollbar width, keeping ribbon/header column boundaries aligned with the time grid.
- Constrained ribbon event title rendering to lane width with truncation to prevent long titles from overflowing columns and shifting neighbouring event blocks.
- Normalised ribbon lane slot sizing and top anchoring to remove vertical baseline drift introduced by mixed text/content sizing.

## [1.4.9] - 2026-02-27

### Changed

- Aligned week/workweek ribbon rules to show only all-day events (single-day and multi-day), removing timed-event spillover from the ribbon row.
- Normalised all-day display end-date handling in week/workweek ribbon rendering so exclusive ICS `DTEND` values do not render one extra day.
- Fixed ICS import for events missing `DTEND` by deriving sane defaults from `DTSTART` (timed: +1 hour, all-day: +1 day), preventing malformed multi-year spans from appearing in current weeks.

## [1.4.8] - 2026-02-27

### Changed

- Tightened `🔒 Synced only` filtering so it includes synced events from enabled calendar sources only.
- Fixed recurrence `RRULE` `UNTIL` parsing to handle datetime forms (for example `UNTIL=20210103T023000Z`), preventing expired series from being expanded as active events.

## [1.4.7] - 2026-02-27

### Changed

- Made scheduled Google Calendar sync fully non-blocking for the UI by running scheduler ticks on a background worker with async result polling.
- Made manual `Sync Now` in Settings non-blocking with worker-thread execution and in-dialog progress/result handling.
- Reduced UI-thread overhead by removing high-frequency layout debug logs and throttling scheduler polling/repaint cadence to improve responsiveness.

## [1.4.6] - 2026-02-27

### Added

- New view-level `🔒 Synced only` toggle to show only read-only events imported from calendar sync sources, improving parity checks against Google Calendar during QA.

### Changed

- Improved ICS import robustness for Google feeds by correctly parsing date-only `EXDATE;VALUE=DATE` entries.
- Improved startup UX by delaying the first automatic background sync shortly after app launch, reducing perceived startup lag.

## [1.4.5] - 2026-02-27

### Added

- Stage 1 S7 scheduler and hardening:
  - New background calendar sync scheduler with per-source poll interval handling.
  - Per-source failure isolation so one failing source does not block others.
  - Retry/backoff policy for failing sources with capped delay.
  - Scheduler diagnostics and tests covering interval behaviour, failure isolation, and redaction.

### Changed

- Wired scheduler execution into the app update loop for automatic periodic sync.
- Hardened sync error handling to redact source ICS URLs from stored/propagated error messages.

## [1.4.4] - 2026-02-27

### Added

- Stage 1 S6 synced event read-only protections and visual marking:
  - Synced events now show a lock marker (`🔒`) in day/week/workweek/month event rendering.
  - Synced events now include explicit read-only tooltip/context-menu messaging.
  - Added shared synced-event lookup helpers for UI enforcement paths.

### Changed

- Blocked local edit/delete/move/resize operations for synced events across day/week/workweek/month interactions.
- Added app-level guardrails so synced events cannot be opened for editing from alternate entry points (search, dialog flows, confirmations).
- Added save-time protection and regression tests to prevent synced event updates from dialog state.

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
