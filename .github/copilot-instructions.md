# Rust Calendar – Copilot Instructions

## Project Overview

Rust Calendar is a cross-platform desktop calendar application built with Rust and egui.

- GUI: egui/eframe
- Database: SQLite via rusqlite
- Platforms: Linux and Windows from a single codebase
- Persistence: local-first via the `directories` crate

Primary goal: a polished, performant calendar app that runs natively on Linux and Windows from a single codebase.

## Repository Structure

| Folder | Purpose |
|--------|---------|
| `src/main.rs` | Binary entry point |
| `src/lib.rs` | Library root |
| `src/models/` | Domain entities |
| `src/services/` | Business logic |
| `src/ui_egui/` | egui UI layer |
| `tests/` | Integration and property tests |
| `benches/` | Criterion benchmarks |
| `assets/` | Theme definitions, icons, and other app assets |
| `packaging/` | Linux desktop integration and local install script |
| `docs/` | Architecture, feature, and planning documentation |
| `.github/` | Copilot guidance and TCTBP runtime/workflow files |

## TCTBP Runtime Surface

The Rust Calendar TCTBP runtime and workflow surface lives in:

- `.github/agents/TCTBP.agent.md`
- `.github/TCTBP.json`
- `.github/TCTBP Agent.md`
- `.github/TCTBP Cheatsheet.md`
- `.github/copilot-instructions.md`
- `.github/prompts/Install TCTBP Agent Infrastructure Into Another Repository.prompt.md`
- optional hook layer: `.github/hooks/tctbp-safety.json` and `scripts/tctbp-pretool-hook.js`

Keep these files aligned when the workflow or runtime entry points change.

## Development Commands

```bash
cargo build
cargo build --release
cargo run
cargo test
cargo clippy -- -D warnings
cargo fmt
cargo fmt -- --check
cargo bench
```

## Cross-Platform Guidelines

- Use `#[cfg(target_os = "...")]` or `cfg!()` for platform-specific code.
- Use `Path` and `PathBuf` plus the `directories` crate for filesystem paths.
- Keep Linux desktop integration in `packaging/`.
- Avoid unguarded platform-specific APIs.

## Quality Gates

1. `cargo test`
2. `cargo clippy -- -D warnings`
3. VS Code Problems tab zero issues
4. `cargo build`

## TCTBP Workflow

For SHIP, checkpoint, publish, handover, resume, deploy, status, abort, and branch transition rules, use:

- `.github/TCTBP.json` as the authoritative machine-readable profile
- `.github/TCTBP Agent.md` for workflow guard rails and interpretation
- `.github/TCTBP Cheatsheet.md` for quick operator guidance
- `.github/agents/TCTBP.agent.md` as the runtime trigger-routing entry point

For this repo, treat `cargo build` as the normal verification build and reserve `cargo build --release` for explicit installation or deployment work.

## Repo-Specific Rules

- Keep `Cargo.toml` as the single source of truth for version.
- Preserve the `vX.Y.Z` tag convention unless explicitly changed.
- Use Australian English in user-facing text and comments.
- Review `README.md`, `docs/USER_GUIDE.md`, and `docs/FEATURES.md` when behaviour changes.
- Review `packaging/install.sh` and `packaging/rust-calendar.desktop` when install behaviour changes.