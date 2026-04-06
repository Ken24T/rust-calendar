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

For SHIP, publish, handover, resume, deploy, status, abort, and branch transition rules, use:

- `.github/TCTBP.json` as the authoritative machine-readable profile
- `.github/TCTBP Agent.md` for workflow guard rails and interpretation
- `.github/TCTBP Cheatsheet.md` for quick operator guidance
- `.github/agents/TCTBP.agent.md` as the runtime trigger-routing entry point

For this repo, treat `cargo build` as the normal verification build and reserve `cargo build --release` for explicit installation or deployment work.

## Repo-Specific Rules

- Use idiomatic Rust: `Result<T, E>`, `Option<T>`, pattern matching, iterators
- Prefer `anyhow::Result` for application-level errors, `thiserror` for library-level error types
- Use `log` macros (`log::info!`, `log::warn!`, `log::error!`) for runtime diagnostics
- Keep files under approximately 300 lines; split by responsibility
- Prefer `&Path` over `&PathBuf` in function signatures
- Use `.clamp()` instead of `.min().max()` chains
- Use `#[derive(...)]` where appropriate (Default, Clone, Debug, Serialize, Deserialize)
- Keep `Cargo.toml` as the single source of truth for version.
- Preserve the `vX.Y.Z` tag convention unless explicitly changed.
- Keep language in Australian English for user-facing text
- Review `README.md`, `docs/USER_GUIDE.md`, and `docs/FEATURES.md` when behaviour changes.
- Review `packaging/install.sh` and `packaging/rust-calendar.desktop` when install behaviour changes.

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

## TCTBP Runtime Files

This repository also carries the TCTBP workflow runtime.

Authoritative workflow files for milestone, sync, and branch actions are:

- `.github/agents/TCTBP.agent.md`
- `.github/TCTBP.json`
- `.github/TCTBP Agent.md`
- `.github/TCTBP Cheatsheet.md`
- `.github/copilot-instructions.md`

If the optional hook layer is enabled, keep these aligned as well:

- `.github/hooks/tctbp-safety.json`
- `scripts/tctbp-pretool-hook.js`

When these files change, keep them aligned. Preserve the Rust project commands, `Cargo.toml` version source, documentation paths, and cross-platform assumptions while merging forward generic TCTBP improvements.
