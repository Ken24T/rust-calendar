# Home Linux App – Copilot Instructions

## Project Overview

This repository is a local-first Linux desktop app project built with Python and GTK4.

Current app:

- TaskPad (notes/tasks desktop app)
- GUI layer in GTK
- Local persistence (JSON now, SQLite planned)

Primary goal: learn native Linux desktop development with pragmatic, incremental delivery.

## Repository Structure

| Folder | Purpose |
|--------|---------|
| `/src/taskpad` | Application code (app lifecycle, UI, models, repository, storage) |
| `/data` | Desktop integration metadata (`.desktop`, metainfo) |
| `/tests` | Unit/integration tests (create as project grows) |
| `/.github` | Automation, Copilot guidance, and SHIP/TCTBP workflow files |

## Development Commands

```bash
# Environment setup
python3 -m venv .venv
source .venv/bin/activate
pip install -e .

# Run app
taskpad

# Baseline validation
python3 -m compileall src

# Tests (when present)
pytest
```

## Environment and Dependencies

- Target runtime: Python 3.10+
- Linux desktop stack: GTK4 via PyGObject (`python3-gi`, `gir1.2-gtk-4.0`)
- No hard-coded machine-specific paths; use XDG paths or user-home-safe defaults

## Architecture and Code Patterns

### Application Layers

- `app.py`: Gtk.Application lifecycle and startup wiring
- `window.py`: UI composition, actions, and user interaction flow
- `models.py`: domain entities and shared value logic
- `repository.py`: business-level data operations and ordering rules
- `storage_*.py`: persistence adapters (JSON now; SQLite adapter in phase 2)

### Coding Style

- Prefer clear, typed Python (`from __future__ import annotations`, dataclasses, explicit types)
- Keep files under approximately 300 lines; split by responsibility
- Keep UI logic and persistence logic separate
- Handle empty, loading-like, and error states explicitly where relevant
- Keep language in Australian English for user-facing text

## Testing

- Baseline verification gate: run `python3 -m compileall src` and ensure diagnostics are zero before SHIP
- Unit tests: use `pytest`, with test files under `tests/` named `test_*.py`
- Prioritise tests for non-UI logic first (`models`, `repository`, persistence adapters)
- Keep GTK UI tests lightweight (smoke-level), unless a UI bug requires deeper coverage

## Security and Safety Rules

1. Never commit secrets, tokens, or credentials
2. Use only local data paths and configuration suitable for desktop apps
3. Do not introduce remote services without explicit user instruction
4. Keep version declarations in sync across relevant project files

## Shipping Workflow

For SHIP/TCTBP activation, order, versioning, tagging, and approval rules, follow:

- `.github/TCTBP.json` (authoritative)
- `.github/TCTBP Agent.md` (behavioural guidance)

Tag convention for SHIP is `vX.Y.Z`.

SHIP cadence for this project:

- SHIP is required after each completed implementation slice by default.
- Docs-only/infrastructure-only slices are committed without version bump/tag.

## Branch Naming

- `feature/<name>` – New features
- `fix/<name>` – Bug fixes
- `docs/<name>` – Documentation updates
- `infrastructure/<name>` – Tooling and workflow changes

## When Generating Code

- Prefer small, focused changes over broad rewrites
- Maintain clear boundaries between UI, domain, and storage
- Add tests alongside new non-trivial logic where practical
- Preserve local-first behaviour unless requirements change
- Keep logging and errors actionable for debugging
