# Rust Calendar – Copilot Instructions

## Project Overview

A modern, feature-rich cross-platform desktop calendar application built with Rust.

- **GUI**: egui/eframe (immediate-mode, cross-platform)
- **Database**: SQLite via rusqlite (bundled)
- **Platforms**: Linux (primary development), Windows, macOS (untested)
- **Persistence**: Local-first, using `directories` crate for XDG/AppData paths

Primary goal: a polished, performant calendar app that runs natively on Linux and Windows from a single codebase.

## Repository Structure

| Folder | Purpose |
|--------|---------|
| `/src/main.rs` | Binary entry point |
| `/src/lib.rs` | Library root (re-exports modules) |
| `/src/models/` | Domain entities: event, recurrence, reminder, settings, category, template, UI state |
| `/src/services/` | Business logic: event CRUD, database, backup, countdown, iCalendar, notifications, PDF, settings, theme |
| `/src/ui_egui/` | egui UI layer: app lifecycle, views, dialogs, event editor, theme, resize, drag |
| `/src/ui_egui/app/` | App shell: lifecycle, menu, navigation, sidebar, shortcuts, state, countdown, dialogs, views |
| `/src/ui_egui/views/` | Calendar views: day, week, workweek, month, quarter |
| `/src/ui_egui/dialogs/` | Modal dialogs: settings, themes, backup, categories, export, search |
| `/src/ui/` | Legacy GTK UI layer (retained, not actively developed) |
| `/src/utils/` | Shared utilities (date helpers) |
| `/tests/` | Integration tests, unit test modules, property tests, fixtures |
| `/benches/` | Criterion benchmarks (recurrence performance) |
| `/assets/themes/` | Theme definitions (TOML: dark, light) |
| `/packaging/` | Linux desktop integration (`.desktop` file) |
| `/docs/` | Architecture, feature, and planning documentation |
| `/.github/` | Copilot guidance and SHIP/TCTBP workflow files |

## Development Commands

```bash
# Build (debug)
cargo build

# Build (release, optimised)
cargo build --release

# Run the application
cargo run

# Run all tests (unit + integration + doc-tests)
cargo test

# Lint (must produce zero warnings before SHIP)
cargo clippy

# Auto-fix simple clippy warnings
cargo clippy --fix --lib --bin rust-calendar --allow-dirty --allow-staged

# Format code
cargo fmt

# Run benchmarks
cargo bench

# Code coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

## Environment and Dependencies

- **Rust toolchain**: Stable (managed via `rust-toolchain.toml`), components: rustfmt, clippy, rust-analyzer
- **Minimum Rust version**: 1.75+
- **Linux build dependencies**: `build-essential libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`
- **Windows build dependencies**: Visual Studio Build Tools with C++ development tools
- **No hard-coded paths**: Use `directories::ProjectDirs` for data/config paths; never hard-code `/home/user` or `C:\Users\`

## Cross-Platform Guidelines

This is a unified codebase targeting Linux and Windows from a single source tree.

- **Platform-specific code**: Use `#[cfg(target_os = "...")]` or `cfg!()` for conditional compilation
- **Platform-specific dependencies**: Place under `[target.'cfg(windows)'.dependencies]` etc. in `Cargo.toml`
- **Fonts**: Use generic family names (`sans-serif`, `monospace`) rather than platform-specific fonts (`Segoe UI`, `Ubuntu`)
- **File paths**: Use `std::path::Path`/`PathBuf` and the `directories` crate; never assume path separators
- **Notifications**: `notify-rust` handles Linux; `windows` crate for Windows toast notifications
- **Desktop integration**: Linux `.desktop` file in `/packaging/`; Windows gets console hiding via `#![windows_subsystem = "windows"]`
- **Always test changes compile on the current platform**; cross-compilation is not required but code must not break other targets

## Architecture and Code Patterns

### Layer Separation

| Layer | Location | Responsibility |
|-------|----------|---------------|
| **Models** | `src/models/` | Domain entities, value types, serialisation |
| **Services** | `src/services/` | Business logic, database access, import/export |
| **UI** | `src/ui_egui/` | Presentation, user interaction, egui rendering |
| **Utils** | `src/utils/` | Shared helpers (date arithmetic) |

### Key Architectural Rules

- **Models are UI-agnostic** — no egui types in `models/`
- **Services are UI-agnostic** — no egui types in `services/`
- **UI calls services, never the reverse**
- **Database access** is exclusively through `services/database/`
- **Theme system**: TOML-based themes in `assets/themes/`, loaded by `services/theme/`

### Coding Style

- Use idiomatic Rust: `Result<T, E>`, `Option<T>`, pattern matching, iterators
- Prefer `anyhow::Result` for application-level errors, `thiserror` for library-level error types
- Use `log` macros (`log::info!`, `log::warn!`, `log::error!`) for runtime diagnostics
- Keep files under approximately 300 lines; split by responsibility
- Prefer `&Path` over `&PathBuf` in function signatures
- Use `.clamp()` instead of `.min().max()` chains
- Use `#[derive(...)]` where appropriate (Default, Clone, Debug, Serialize, Deserialize)
- Keep language in Australian English for user-facing text

## Testing

- **Test command**: `cargo test` (runs unit, integration, and doc-tests)
- **Test location**: Unit tests as `#[cfg(test)] mod tests` within source files or under `tests/unit/`; integration tests under `tests/`
- **Property tests**: `proptest` under `tests/property/`
- **Benchmarks**: `criterion` under `benches/`
- **Test utilities**: `tempfile` for temporary directories, `mockall` for mocking, `pretty_assertions` for readable diffs
- **Prioritise tests** for models, services, and recurrence logic over UI code
- **All tests must pass** before any SHIP

## Quality Gates (Enforced Before SHIP)

1. `cargo test` — 100% pass rate
2. `cargo clippy` — zero warnings
3. VS Code Problems tab — zero issues (includes markdown lint)
4. `cargo build` — clean compilation

## Security and Safety Rules

1. Never commit secrets, tokens, or credentials
2. Use only local data paths via `directories::ProjectDirs`
3. Do not introduce remote services or network calls without explicit user instruction
4. Keep version declarations in sync (`Cargo.toml` is the single source of truth for version)
5. SQLite database is local-only; no cloud sync

## Shipping Workflow

For SHIP/TCTBP activation, order, versioning, tagging, and approval rules, follow:

- `.github/TCTBP.json` (authoritative)
- `.github/TCTBP Agent.md` (behavioural guidance)

Tag convention: `vX.Y.Z`

SHIP cadence:

- SHIP is required after each completed implementation slice by default
- Docs-only/infrastructure-only slices are committed without version bump/tag

## Branch Naming

- `feature/<name>` – New features
- `fix/<name>` – Bug fixes
- `docs/<name>` – Documentation updates
- `infrastructure/<name>` – Tooling and workflow changes

## When Generating Code

- Prefer small, focused changes over broad rewrites
- Maintain clear boundaries between models, services, and UI
- Add tests alongside new non-trivial logic where practical
- Preserve local-first behaviour unless requirements change
- Keep logging and errors actionable for debugging
- Respect cross-platform compatibility — avoid platform-specific APIs without `#[cfg]` guards
- Run `cargo clippy` mentally — avoid patterns that trigger common lints
