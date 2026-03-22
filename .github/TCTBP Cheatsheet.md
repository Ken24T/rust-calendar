# TCTBP Developer Cheatsheet

Short operator reference for the Rust Calendar TCTBP workflows.

Use this file for the quick view.
Use [TCTBP Agent.md](TCTBP%20Agent.md) for the full workflow rules and guard rails.

## Core Rule

- No code is ever lost while syncing local and remote state.
- Do not use destructive shortcuts as part of normal workflow execution.
- If a workflow hits divergence, ambiguity, or a failed invariant, it should stop rather than guess.

## Repo Gates

- Format check: `cargo fmt -- --check`
- Test: `cargo test`
- Lint: `cargo clippy`
- Normal build gate: `cargo build`
- Runtime or deployment build: `cargo build --release`

## Triggers

### `ship` / `ship please` / `shipping` / `tctbp` / `prepare release`

Purpose:
Formal source release workflow.

Attempts to:

- preflight the repo state
- show a concise origin-vs-local snapshot table before mutating anything
- run verification gates
- confirm zero problems
- assess docs impact
- bump version when required
- commit the release changes
- create the version tag
- push the current branch

Use when:

- you want a formal shipped version
- version and tag state needs to be updated
- the repo should be published as a release milestone

Notes:

- Starts with a four-column table: `Origin`, `Local`, `Status`, `Action(s)`
- Table should explicitly include rows such as tag state and commits ahead/behind
- Uses the normal build gate by default, not the release build
- Patch bump happens on every ship unless the changes are docs-only or infrastructure-only
- Release build is reserved for installation or deployment scenarios
- Stops if the branch is dirty, missing an upstream, behind origin, or diverged from origin
- Stops if `HEAD` is detached

### `handover` / `handover please`

Purpose:
Safely reconcile the working branch with `origin` so you can stop on one machine and resume on another from the latest safely recoverable shared state.

Scope:

- syncs the active work branch, a dedicated handover metadata branch, and relevant tags only
- does not reconcile every branch in the repository
- does not merge into `main` as part of normal machine-to-machine sync

Handover metadata:

- metadata branch: `tctbp/handover-state`
- metadata file: `.github/TCTBP_STATE.json`
- stores the last successfully handed-over work branch and commit
- is consulted before branch-recency inference on another machine
- is never treated as a work-branch candidate itself

Attempts to:

- preserve dirty work on the active branch when needed
- create a durable checkpoint for dirty unpublished work before verification can strand it on one machine
- fetch and inspect remote state
- prefer the handover metadata branch over an arbitrary clean non-default branch when metadata is newer and valid
- fall back to branch detection only when metadata is missing, stale, or invalid
- ask for confirmation before switching if branch choice is ambiguous
- check out the target branch when safe
- fast-forward when remote is ahead and local is clean
- publish the branch when local is ahead
- stop on divergence or ambiguity
- push relevant tags when appropriate
- confirm that you are positioned back on the resumed work branch with local and remote in sync

Notes:

- Ends with a concise four-column table: `Origin`, `Local`, `Status`, `Action(s)`
- Keep the handover table to five rows focused on branch sync, latest tag, metadata branch publication, metadata consistency, and final baseline state
- Use a short completion line after the table to confirm the handed-over branch and commit
- Update the metadata branch using a secondary worktree or another equally non-destructive method
- Stops if `HEAD` is detached

Use when:

- you are finishing work on one machine
- you are resuming work on another machine
- you want one trusted sync command before stopping or starting work

Never does:

- auto-rebase
- hard reset
- destructive checkout
- force-push

### `deploy` / `deploy please`

Purpose:
Build a runtime-ready artefact and install it into the local desktop environment.

Attempts to:

- preflight the repo state and deploy target
- require a clean tree and synced branch
- optionally run `ship` first if repo policy requires it
- run verification gates
- assess docs impact for packaging, runtime, or install-path changes
- run the release build path
- execute the repo install script
- run post-deploy validation checks
- summarise the deployed result

Use when:

- the local installed Rust Calendar runtime should be updated
- a release artefact is required for install verification

Repo-specific deploy target:

1. `linux-user-local`
   - runs `./packaging/install.sh`
   - validates `~/.local/bin/rust-calendar`
   - validates `~/.local/share/applications/rust-calendar.desktop`
   - validates `~/.local/share/icons/hicolor/256x256/apps/rust-calendar.png`

Current deploy policy:

- `requireCleanTree: true`
- `requireSyncedBranch: true`
- `requireShipFirst: false`
- `migrationCommand: null`

Deploy guard rails:

- detached `HEAD` should stop deploy
- destructive replacement without rollback expectations should stop deploy

### `status` / `status please`

Purpose:
Read-only operator snapshot of branch state, sync status, last tag, and recommended next steps.

Use when:

- you want to know whether `handover`, `ship`, or `abort` is needed before doing anything else

Notes:

- This is the trigger that should show the fuller four-column table: `Origin`, `Local`, `Status`, `Action(s)`
- Table should explicitly include branch state, default-branch state, tag state, ahead/behind counts, working tree, and whether `ship` or `handover` is recommended
- If metadata points another machine at the wrong published branch, call that out as a resume-target mismatch

### `abort`

Purpose:
Inspect and recover from a partially completed SHIP, sync, or deploy workflow.

Use when:

- a prior workflow stopped part-way through
- version, tag, merge, or push state looks inconsistent
- branch publication and handover metadata disagree
- a version bump, changelog update, or tag exists without the rest of the release state
- `main` and a newly created branch are only partially published after a branch workflow

Recovery expectations:

- inspect concrete partial states before proposing action
- preserve unpublished work before cleanup when needed
- never rewrite history or force-push without explicit extra confirmation

### `branch <new-branch-name>`

Purpose:
Close out current work cleanly and start the next branch.

Attempts to:

- assess whether the current branch should be shipped first
- stop if `HEAD` is detached
- stop if the requested new branch name is invalid or already exists locally or remotely
- stop instead of switching if the current branch is dirty and SHIP is declined
- stop instead of guessing if the source branch or local `main` is diverged
- stop if the source branch is ahead, behind, or otherwise not yet synced to its upstream
- merge the current branch into local `main` when the current branch is not already `main`
- skip the merge step when you already start on `main`
- create and switch to the new branch from updated local `main`

Safety expectation:

- never discards local work to complete the transition
- never uses stash, reset, rebase, force-push, or destructive checkout as part of the branch workflow
- requires the source branch to be published before branch closeout continues
- only offers old-branch deletion after the merge and new-branch creation have both succeeded

## Handover Promise

When `handover` succeeds:

- the active work branch has been safely reconciled with `origin`
- the handover metadata branch points at that branch and handed-over commit
- relevant tags have been pushed when needed
- if you started on another machine from a clean state, you are back on the detected and confirmed work branch
- if the workflow could not do that safely, it stops instead of guessing
- no implicit merge to `main` was performed as part of that sync

## Docs Impact Reminder

Review docs when the change touches:

- user-visible features
- UI or interaction
- config or settings
- packaging or metadata
- roadmap or status

Repo-specific docs commonly reviewed:

- [README.md](../README.md)
- [docs/USER_GUIDE.md](../docs/USER_GUIDE.md)
- [docs/FEATURES.md](../docs/FEATURES.md)
- [docs/UI_SYSTEM.md](../docs/UI_SYSTEM.md)
- [docs/FUTURE_ENHANCEMENTS.md](../docs/FUTURE_ENHANCEMENTS.md)
- [packaging/install.sh](../packaging/install.sh)
- [packaging/rust-calendar.desktop](../packaging/rust-calendar.desktop)

## Deployment Notes

- `cargo build` is the normal verification build
- `cargo build --release` is for installation or deployment work
- Deployment should validate the installed result, not just copy files
- Use `./packaging/install.sh` instead of ad hoc copy commands

## Approval Model

- `ship` may create local commit and tag state as part of the workflow
- `handover` grants approval to push the target branch and relevant tags for that workflow only
- `deploy` grants approval to run the repo-defined deployment commands for that workflow only
- Any other remote push still requires explicit approval unless already covered by the active workflow

## Quick Choice

- Need a release version or tag: use `ship`
- Need to stop on one machine and resume on another safely: use `handover`
- Need the local runtime installed or refreshed: use `deploy`
- Need a quick repo state check: use `status`
- Need to recover from partial workflow state: use `abort`
- Need to start the next branch: use `branch <new-branch-name>`
