# Rust Calendar

A modern, feature-rich cross-platform desktop calendar application built with Rust.

## Features

🎨 **Modern UI with Theming**

- Light and Dark mode support
- Customizable themes
- Smooth transitions and animations
- Native look and feel on each platform
- **Multiple calendar views**:
  - Day view - Detailed hourly schedule
  - Work week - Monday through Friday
  - Full week - Complete 7-day view
  - Month view - Traditional calendar grid
  - Quarter view - 3-month overview
  - Year view - Annual 12-month display
  - Agenda view - Linear event list
- **My Day Panel** - Sidebar displaying selected day's events
  - Configurable positioning (left/right/hidden)
  - Adjustable width (180-400px)
  - Chronological event listing
  - Auto-updates with calendar navigation
- **Multi-Day Event Ribbon** - Top banner for spanning events
  - Shows events crossing 2+ days
  - Multiple display modes (compact/expanded/auto)
  - Progress indicators for ongoing events
  - Keeps main calendar grid uncluttered
- **Full UI Customization**:
  - Adjustable column widths (drag to resize)
  - Customizable fonts (family, size, weight, style)
  - Resizable row heights
  - **Configurable time granularity** (15/30/60 minute intervals)
  - **Adjustable default event duration** (default: 45 minutes)
  - All preferences saved between sessions

📅 **Powerful Event Management**

- Create single and repeating events
- Support for complex recurrence patterns:
  - Daily, Weekly, **Fortnightly**, Monthly, **Quarterly**, Yearly
  - Custom patterns (e.g., "every 2nd Tuesday")
- Edit individual occurrences or entire series
- Color-coded categories
- Event notes and location tracking
- **Drag events to desktop to create countdown timer widgets**
- Drag and drop .ics files to import events

⏰ **Smart Reminders**

- Multiple configurable reminders per event
- Cross-platform desktop notifications
- Snooze functionality
- Custom reminder times

💾 **Reliable Data Storage**

- Local SQLite database
- Import/Export iCalendar (.ics) format
- **Drag-and-drop .ics file import**
- Automatic data backup
- UI preferences persistence

⌨️ **Keyboard Shortcuts**

- `Ctrl+N` - Create new event
- `Ctrl+T` - Navigate to today
- `Ctrl+S` - Open settings
- `Ctrl+B` - Backup database
- `Arrow Left/Right` - Navigate previous/next period (day/week/month)
- `Arrow Up/Down` - Navigate vertically (contextual to current view)
- `Escape` - Close dialogs

## Project Status

🚧 **Currently in Planning Phase** 🚧

This project is in active development. See `docs/PROJECT_PLAN.md` for the detailed development roadmap.

## Requirements

- **Linux**: X11 or Wayland desktop environment, Rust 1.75+
- **Windows**: Windows 10/11, Rust 1.75+, Visual Studio Build Tools
- **macOS**: macOS 11+, Rust 1.75+ (untested but should work)

## Getting Started

### Prerequisites

1. Install Rust from [rustup.rs](https://rustup.rs/)
2. **Linux**: Install system dependencies:

   ```bash
   # Debian/Ubuntu/Mint
   sudo apt install build-essential libgtk-3-dev libxcb-render0-dev \
     libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
   ```

3. **Windows**: Install Visual Studio Build Tools with C++ development tools

### Building from Source

```bash
# Clone the repository
git clone https://github.com/Ken24T/rust-calendar.git
cd rust-calendar

# Build the project
cargo build --release

# Run the application
cargo run --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage report
cargo install cargo-tarpaulin
cargo tarpaulin --out Html

# Run benchmarks
cargo bench
```

## Project Structure

```text
rust-calendar/
├── src/              # Source code (modular, small files <300 lines)
│   ├── ui/           # User interface components
│   ├── models/       # Data models (event, recurrence, reminder)
│   ├── services/     # Business logic services
│   └── utils/        # Utility functions
├── tests/            # Comprehensive test suite
│   ├── unit/         # Unit tests (mirror src/ structure)
│   ├── integration/  # Integration tests
│   ├── property/     # Property-based tests
│   └── fixtures/     # Test data and helpers
├── benches/          # Performance benchmarks
├── assets/           # Themes, icons, and resources
├── docs/             # Documentation
└── Cargo.toml        # Project configuration
```

## Design Principles

🎯 **Modularity First**

- Small, focused files (max 300 lines)
- Single responsibility per module
- Highly composable architecture

🧪 **Comprehensive Testing**

- >90% code coverage requirement
- Unit, integration, and property-based tests
- Every module has corresponding tests
- Test-driven development approach

## Documentation

- [Project Plan](docs/PROJECT_PLAN.md) - Comprehensive project roadmap and technical details
- [Architecture Guide](docs/ARCHITECTURE.md) - System architecture and design patterns
- [Modularity Guidelines](docs/MODULARITY.md) - Code organization and best practices
- [Testing Guidelines](docs/TESTING.md) - Testing strategy and requirements
- [UI System](docs/UI_SYSTEM.md) - Complete UI system documentation with views and customization
- User Guide - Coming soon

## Technology Stack

- **GUI Framework**: [egui/eframe](https://github.com/emilk/egui) - Cross-platform immediate-mode UI
- **Database**: SQLite via [rusqlite](https://github.com/rusqlite/rusqlite) (bundled)
- **Date/Time**: [chrono](https://github.com/chronotope/chrono)
- **Recurrence**: [rrule](https://github.com/fmeringdal/rust-rrule) - RFC 5545 compliant
- **Notifications**: [notify-rust](https://github.com/hoodie/notify-rust) - Cross-platform desktop notifications
- **Data Paths**: [directories](https://github.com/dirs-dev/directories-rs) - XDG/AppData path resolution

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is dual-licensed under MIT OR Apache-2.0.

## Roadmap

### Phase 1: Foundation ✅

- [x] Project structure setup
- [x] Dependencies configuration
- [ ] Database schema implementation
- [ ] Core data models

### Phase 2: Event Management 🚧

- [ ] Event CRUD operations
- [ ] Recurrence rule engine
- [ ] iCalendar import/export

### Phase 3: User Interface 📋

- [ ] Main application window
- [ ] Calendar views (month/week/day)
- [ ] Event creation and editing

### Phase 4: Reminders 📋

- [ ] Reminder scheduling
- [ ] Cross-platform desktop notifications
- [ ] Background reminder service

### Phase 5: Theming 📋

- [ ] Theme system
- [ ] Light/Dark modes
- [ ] Theme customization

### Phase 6: Polish 📋

- [ ] Testing and optimization
- [ ] Documentation
- [ ] Linux .desktop integration and packaging

## Acknowledgments

Built with ❤️ using Rust and the amazing open-source ecosystem.

---

**Note**: This is a personal project currently under development. Features and documentation will be updated as the project progresses.
