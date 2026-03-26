# Rust Calendar — TCTBP Cheatsheet

Short operator reference for the Rust Calendar workflows.

Use this file for the quick view.
Use [TCTBP Agent.md](TCTBP%20Agent.md) for the full workflow rules and guard rails.

## Core Rule

- No code is ever lost while syncing local and remote state.
- Do not use destructive shortcuts as part of normal workflow execution.
- If a workflow hits divergence, ambiguity, failed verification, or stale release state, it should stop rather than guess.

## Repo Gates

- Format check: `cargo fmt -- --check`
- Test: `cargo test`
- Lint: `cargo clippy -- -D warnings`
- Normal build gate: `cargo build`
- Release build: `cargo build --release`

## Version And Tags

- Version source: `Cargo.toml` field `package.version`
- Tag format: `vX.Y.Z`

## Triggers

### `ship` / `ship please` / `shipping` / `prepare release`

Formal source release workflow.

- preflights repo state
- runs verification gates
- assesses docs impact
- bumps version when required
- commits, tags, and pushes

### `publish` / `publish please`

Safely publish the current clean branch to `origin` without release semantics.

- no version bump
- no tag creation
- no handover metadata update

### `checkpoint` / `checkpoint please`

Create a durable local-only checkpoint commit on the current branch without release or sync side effects.

- stops if `HEAD` is detached, the tree is clean, conflicts exist, or a merge/rebase/cherry-pick/revert is in progress
- stages current tracked and non-ignored untracked changes
- creates a clearly marked non-release local commit
- ends with a concise four-column table covering the previous HEAD, new checkpoint commit, resulting working-tree state, sync state, and explicit local-only outcome
- does not push, create a tag, or update handover metadata

### `handover` / `handover please`

Safely checkpoint and publish the current work branch, then refresh `tctbp/handover-state` so another machine can resume deterministically.

### `resume` / `resume please`

Safely restore the intended work branch at the start of a session by consulting handover metadata first.

### `deploy` / `deploy please`

Run an explicit install target.

Repo-specific target:

- `linux-user-local`
  - build: `cargo build --release`
  - install: `bash ./packaging/install.sh`
  - validate: confirm the installed binary and desktop file exist under `~/.local`

### `status` / `status please`

Read-only operator snapshot of branch state, sync status, tags, version source, and recommended next steps.

### `abort`

Inspect and recover from a partially completed SHIP, sync, or deploy workflow.

### `branch` / `branch <new-branch-name>`

Close out current work cleanly, optionally starting the next branch.

- `branch` closes out the current branch and leaves the repo on the updated `main`
- `branch <new-branch-name>` closes out the current branch and starts the next branch from the updated `main`
- asks for explicit confirmation before merging a non-default branch back into `main`
- requires the source branch to be published before the transition continues

## Docs Impact Reminder

Repo-specific docs commonly reviewed:

- `README.md`
- `docs/USER_GUIDE.md`
- `docs/FEATURES.md`
- `docs/UI_SYSTEM.md`
- `CHANGELOG.md`
- `packaging/install.sh`
- `packaging/rust-calendar.desktop`
- `.github/TCTBP Agent.md`
- `.github/TCTBP Cheatsheet.md`
- `.github/copilot-instructions.md`

## Quick Choice

- Need a release version or tag: use `ship`
- Need a durable local-only save before deciding whether to publish or hand over: use `checkpoint`
- Need to sync a clean branch without release or metadata side effects: use `publish`
- Need to stop on one machine and resume on another safely: use `handover`
- Need to restore the last handed-over branch before starting work: use `resume`
- Need the local Linux install updated: use `deploy`
- Need a quick repo state check: use `status`
- Need to recover from a partial workflow state: use `abort`
- Need to close out the current branch or start the next branch: use `branch` or `branch <new-branch-name>`