# Rust Calendar

A modern, feature-rich cross-platform desktop calendar application built with
Rust and [egui](https://github.com/emilk/egui).

Rust Calendar runs natively on **Linux** and **Windows** from a single codebase,
with local-first data storage and no cloud dependencies.

## Features

### Calendar Views

- **Day** — detailed hourly schedule
- **Work Week** — Monday through Friday
- **Week** — full 7-day view
- **Month** — traditional calendar grid
- **Quarter** — 3-month overview

### Event Management

- Create, edit, and delete single or recurring events
- Complex recurrence patterns: daily, weekly, fortnightly, monthly, quarterly,
  yearly, and custom intervals
- Edit individual occurrences or entire series
- Colour-coded categories and event templates
- Event notes and location tracking
- Drag-and-drop `.ics` file import
- iCalendar (`.ics`) import and export

### My Day Panel

- Sidebar showing the selected day's events in chronological order
- Configurable positioning (left, right, or hidden) and adjustable width
- Auto-updates as you navigate the calendar

### Multi-Day Event Ribbon

- Top banner for events spanning two or more days
- Multiple display modes (compact, expanded, auto)
- Progress indicators for ongoing events

### Countdown Timers

- Tear events from the calendar to create desktop countdown widgets
- Individual floating windows, combined container, or category containers mode
- Always-on-top, persistent countdown display
- Category containers with collapsible headers, sort modes, and quick-add
- Four-tier visual inheritance: Global → Template → Category → Card
- Reusable card templates (colours, fonts, default dimensions)
- Per-category layout orientation (Auto, Portrait, Landscape)
- Cross-container drag-and-drop to re-categorise cards
- Choose target container when creating from context menu or event dialog
- Portable layout export/import (JSON) for cross-machine setup transfer

### Customisation

- Light and dark themes with TOML-based theme definitions
- Custom theme creation
- Adjustable column widths (drag to resize)
- Configurable time granularity (15, 30, or 60 minute intervals)
- Adjustable default event duration
- All preferences persisted between sessions

### System Tray

- Optional "Minimise to system tray on close" setting
- Tray context menu (Show Calendar, Exit) and left-click restore
- Countdown cards remain visible while the main window is hidden
- Cross-platform support (Windows and Linux)

### Reminders and Notifications

- Multiple configurable reminders per event
- Cross-platform desktop notifications
- Snooze functionality

### Keyboard Shortcuts

- `Ctrl+N` — new event
- `Ctrl+T` — go to today
- `Ctrl+S` — open settings
- `Ctrl+B` — backup database
- `Left` / `Right` — navigate previous/next period
- `Up` / `Down` — navigate vertically (contextual)
- `Escape` — close dialogs

## Getting Started

### Prerequisites

1. Install Rust (1.75+) from [rustup.rs](https://rustup.rs/)
2. Platform-specific dependencies:

   **Linux** (Debian/Ubuntu/Mint):

   ```bash
   sudo apt install build-essential libgtk-3-dev libxcb-render0-dev \
     libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev \
     libayatana-appindicator3-dev libxdo-dev
   ```

   **Windows**: Visual Studio Build Tools with C++ development tools

### Build and Run

```bash
git clone https://github.com/Ken24T/rust-calendar.git
cd rust-calendar

cargo build --release
cargo run --release
```

### Running Tests

```bash
cargo test          # unit + integration + doc-tests
cargo clippy        # lint (zero warnings required)
cargo bench         # criterion benchmarks
```

## Project Structure

```text
rust-calendar/
├── src/
│   ├── main.rs          # Binary entry point
│   ├── lib.rs           # Library root
│   ├── models/          # Domain entities (event, recurrence, reminder, settings)
│   ├── services/        # Business logic (database, events, backup, themes, etc.)
│   ├── ui_egui/         # egui UI layer (views, dialogs, event editor, app shell)
│   └── utils/           # Shared helpers (date arithmetic)
├── tests/               # Integration tests
├── benches/             # Criterion performance benchmarks
├── assets/themes/       # Theme definitions (dark.toml, light.toml)
├── docs/                # Architecture, design, and planning documents
├── packaging/           # Linux desktop integration (.desktop file)
└── Cargo.toml           # Project configuration
```

## Technology Stack

- **GUI**: [egui/eframe](https://github.com/emilk/egui) — immediate-mode, cross-platform
- **Database**: SQLite via [rusqlite](https://github.com/rusqlite/rusqlite) (bundled)
- **Date/Time**: [chrono](https://github.com/chronotope/chrono)
- **Recurrence**: [rrule](https://github.com/fmeringdal/rust-rrule) — RFC 5545 compliant
- **Notifications**: [notify-rust](https://github.com/hoodie/notify-rust) (Linux), Windows toast API
- **Data Paths**: [directories](https://github.com/dirs-dev/directories-rs) — XDG/AppData resolution

## Documentation

- [User Guide](docs/USER_GUIDE.md) — how to use the application
- [Features](docs/FEATURES.md) — concise feature summary
- [docs/README.md](docs/README.md) — full index of all project documentation

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding standards,
branching conventions, and the pull request process.

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE),
at your option.
