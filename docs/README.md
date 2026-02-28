# Rust Calendar — Documentation Index

This folder contains design documents, architecture guides, and planning
materials for the Rust Calendar project.

## Current Documentation

- [USER_GUIDE.md](USER_GUIDE.md) — How to use Rust Calendar (end user guide)
- [FEATURES.md](FEATURES.md) — Concise feature summary
- [ARCHITECTURE.md](ARCHITECTURE.md) — Application architecture, module layout, and data flow
- [MODULARITY.md](MODULARITY.md) — Code organisation guidelines and file size limits
- [TESTING.md](TESTING.md) — Testing philosophy, patterns, and coverage requirements
- [UI_SYSTEM.md](UI_SYSTEM.md) — UI system overview: views, customisation, shortcuts
- [FUTURE_ENHANCEMENTS.md](FUTURE_ENHANCEMENTS.md) — Planned future improvements
- [PROJECT_PLAN.md](PROJECT_PLAN.md) — Original project plan (historical; see ARCHITECTURE.md for current design)

## Feature Planning — Google Calendar Sync

- [GOOGLE_CALENDAR_STAGE1_ROADMAP.md](GOOGLE_CALENDAR_STAGE1_ROADMAP.md) — Stage 1 roadmap for read-only ICS feed sync
- [GOOGLE_CALENDAR_STAGE1_SLICES.md](GOOGLE_CALENDAR_STAGE1_SLICES.md) — Implementation slices (S1–S6+) for Stage 1

## Implemented Design Specifications

These documents are the original design specs for features that have been built
and shipped. They are retained for historical reference; actual implementations
may differ in details.

- [COUNTDOWN_TIMER_FEATURE.md](COUNTDOWN_TIMER_FEATURE.md) — Desktop countdown timer widgets
- [countdown-container-plan.md](countdown-container-plan.md) — Countdown container (combined card window)
- [MY_DAY_AND_RIBBON_FEATURES.md](MY_DAY_AND_RIBBON_FEATURES.md) — My Day sidebar panel and multi-day event ribbon
- [UI_FEATURES_UPDATE.md](UI_FEATURES_UPDATE.md) — Calendar views, customisation, drag-and-drop

## Archived (Historical)

Early-development documents preserved in [`archive/`](archive/) for reference:

- [EGUI_MIGRATION.md](archive/EGUI_MIGRATION.md) — iced → egui migration tracker (complete)
- [database-integration.md](archive/database-integration.md) — Initial SQLite integration notes
- [feature-settings-database-summary.md](archive/feature-settings-database-summary.md) — Settings-database feature branch summary
- [UPDATES.md](archive/UPDATES.md) — Early project update: modularity and recurrence
