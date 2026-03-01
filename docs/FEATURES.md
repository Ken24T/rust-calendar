# Rust Calendar — Features

A summary of the features available in Rust Calendar.

## Calendar Views

- **Day** — hourly schedule for a single day with time grid
- **Work Week** — Monday through Friday with time slots
- **Week** — full 7-day view with time slots
- **Month** — traditional monthly calendar grid
- **Quarter** — 3-month overview

## Event Management

- Create, edit, and delete events with title, location, description, category,
  and custom colour
- **Recurring events**: daily, weekly, fortnightly, monthly, quarterly, yearly,
  and custom intervals
- Edit single occurrences or entire series
- Overlap detection warnings
- Colour-coded categories with default and custom categories
- Event templates for frequently-used configurations
- Event validation (title required, date constraints)

## My Day Sidebar

- Mini calendar for quick date navigation
- Today's events list (up to 5, with click-to-navigate)
- Upcoming events within 30 days
- Resizable (150–300 px), dockable left or right
- Toggle with `Ctrl+\`

## Multi-Day Event Ribbon

- Top banner for events spanning two or more days
- Compact, expanded, and auto display modes
- Progress indicators for events in progress
- Keeps the main calendar grid uncluttered

## Countdown Timers

- Desktop countdown widgets for upcoming events
- Individual floating windows or combined container mode
- Customisable accent colours, font sizes, and dimensions
- Always-on-top option
- Auto-create on ICS import (optional)

## Drag and Drop

- Move events between time slots and days (Day, Week, Work Week views)
- Drag `.ics` files from the file manager to import events
- Event duration preserved during moves

## Resize

- Drag the top or bottom edge of an event to change its start or end time
- Works in Day, Week, and Work Week views

## Import and Export

- **iCalendar (.ics)**: import and export individual events, filtered sets, date
  ranges, or all events
- **PDF export**: month view, week view, or full event list
- **Database backup**: timestamped SQLite copies with one-click restore

## Themes

- Built-in light and dark themes (TOML-based)
- Custom theme creator with live preview
- System theme auto-detection (follow OS light/dark preference)

## Settings

- First day of week and work week boundaries
- 12-hour or 24-hour time format
- Date format (DD/MM/YYYY, MM/DD/YYYY, YYYY-MM-DD)
- Default event duration and start time
- Default calendar view
- Sidebar position and visibility
- ISO week numbers
- Countdown card default dimensions
- Google Calendar sync configuration

## System Tray

- Minimise to system tray on close (optional setting)
- Tray context menu: Show Calendar, Exit
- Left-click tray icon to restore the main window
- Countdown cards remain visible while the main window is hidden
- Cross-platform: Windows (off-screen + Win32) and Linux (GTK/libappindicator)

## Notifications

- Cross-platform desktop notifications for event reminders
- Multiple reminders per event
- Snooze functionality

## Undo / Redo

- Undo and redo for event modifications
- `Ctrl+Z` / `Ctrl+Y` or `Ctrl+Shift+Z`

## Search

- Full-text search across event titles and descriptions (`Ctrl+F`)

## Keyboard-Driven Navigation

- Single-key view switching (`D`, `W`, `K`, `M`)
- Arrow key period navigation
- Shortcut keys for all common actions

## Cross-Platform

- Runs natively on Linux and Windows from a single codebase
- Local-first — no cloud, no accounts, no telemetry
- SQLite database with automatic backup support
