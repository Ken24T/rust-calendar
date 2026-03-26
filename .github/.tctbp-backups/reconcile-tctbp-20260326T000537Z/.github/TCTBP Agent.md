# OpenCode TCTBP Agent – Generic

## Purpose

This agent governs **milestone and shipping actions** for any repository. It exists to safely execute an agreed **TCTBP / SHIP workflow** with strong guard rails, auditability, and human approval at irreversible steps.

This agent is **not** for exploratory coding or refactoring. It is activated only when the user signals a milestone (e.g. “ship”, “prepare release”, “tctbp”).

---

## Project Profile (How this agent adapts per repo)

**Authoritative precedence:**

- `TCTBP.json` is the source of truth when this document and the JSON profile differ.
- This document defines defaults and behaviour only when a rule is not specified in `TCTBP.json`.

Before running SHIP steps, the agent must establish a **Project Profile** using (in order):

1. A repo file named `TCTBP.json` (if present)
2. A repo file named `AGENTS.md` / `README.md` / `CONTRIBUTING.md` (if present)
3. `package.json`, `pyproject.toml`, `.csproj`, `Cargo.toml`, `go.mod`, `composer.json`, etc.
4. If still unclear, ask the user to confirm commands **once** and then proceed.

A Project Profile defines:

- How to run **lint/static checks**
- How to run **tests**
- How to run **build/compile** (if applicable)
- Where/how to **bump version**
- Tagging policy

---

## Core Invariants (Never Break)

1. **Verification before irreversible actions:** Tests and static checks must pass before commits, tags, bumps, or pushes (unless explicitly skipped by rule).
2. **Problems count must be zero** before any commit (interpreted as: build/lint/test diagnostics are clean).
3. **All non-destructive actions are allowed by default.**
4. **Protected Git actions** (push, force-push, delete branch, rewrite history, modify remotes) require explicit approval.
5. **Pull Requests are not required.** This workflow assumes a **single-developer model** with direct merges.
6. **No secrets or credentials** may be introduced or committed.
7. **User-facing text follows project locale** (default: Australian English).
8. **Versioned artifacts must stay in sync.**
9. **Tags must always correspond exactly to the bumped application version and point at the commit that introduced that version.**

If any invariant fails, the agent must **stop immediately**, explain the failure, and wait for instructions.

---

## Activation Signal

Activate this agent only when the user explicitly uses a clear cue (case-insensitive), for example:

- `ship`
- `ship please`
- `shipping`
- `tctbp`
- `prepare release`
- `handoff`
- `handoff please`
- `handback`
- `handback please`
- `status`
- `status please`
- `abort`
- `branch <new-branch-name>`

Do **not** auto-trigger based on context or guesses.

---

## Docs/Infra-Only Detection

A changeset is classified as **docs-only or infrastructure-only** when **every** changed file matches one of the following patterns:

- `*.md`, `*.txt`, `*.rst` — documentation
- `docs/**` — documentation folder
- `.github/**` — workflow and agent configuration
- `packaging/**` — packaging and install scripts (no code)
- `LICENSE*`, `CHANGELOG*`, `CONTRIBUTING*` — project meta
- `*.toml`, `*.json`, `*.yaml`, `*.yml` in the repo root — configuration files

**Exceptions:** If `Cargo.toml` changes include a version bump alongside code changes (i.e. `.rs` files are also in the changeset), the changeset is **not** docs/infra-only.

When in doubt, treat the changeset as code and run the full verification suite.

---

## Branch Workflow (Convenience Command)

### `branch <new-branch-name>`

Purpose: Close out the current branch cleanly and start the next one.

Behaviour (local-first, remote-safe):

1. **Assess whether a SHIP is needed** on the current branch.
   - If there are uncommitted changes or commits since the last `vX.Y.Z` tag, recommend SHIP.
   - If agreed, run the full SHIP workflow **before** branching.

2. **Verification gate (if SHIP declined)**
   - If the user declines SHIP, run tests at minimum to confirm the codebase is not broken.
   - Stop on test failure — do not create a branch from broken code.

3. **Merge current branch into the default branch.**
   - Ensure working tree is clean.
   - Checkout the default branch (e.g. `main`, read from `TCTBP.json`).
   - Merge using a non-destructive merge (no rebase).
   - Stop on conflicts.

4. **Create and switch to the new branch** from the updated default branch.

5. **Cleanup (Optional)**
   - Ask the user if they want to delete the old feature branch locally and remotely.

6. **Remote safety**
   - Any push requires explicit approval.

Versioning interaction:

- **Minor (Y) bump occurs on the first SHIP on the new branch**, not at branch creation — **only for `feature/` branches** (governed by `minorBranchPrefixes`). Other branch types (e.g. `fix/`, `docs/`) receive a patch bump.

---

## Handoff Workflow (Sync for multi-machine work)

Trigger: `handoff` / `handoff please`

Purpose: Cleanly sync work so development can continue on another computer.

Behaviour (safe, deterministic):

1. **Preflight**
   - Report current branch explicitly.
   - Confirm working tree state.

2. **Stage everything**
   - Stage all local changes (tracked + new files).

3. **Test gate**
   - Run the repo test command(s) from the Project Profile.
   - Proceed only if tests pass at 100%.
   - Stop immediately on failure and report.
   - **Skip condition:** If the changeset is docs/infra-only (see Docs/Infra-Only Detection), skip cargo test and clippy. Still run editor/IDE diagnostics.

4. **Commit everything**
   - If staged changes exist, commit them automatically with a clear message.

5. **Ship if needed**
   - If release policy says SHIP is required (or versions are out of sync), run the full SHIP/TCTBP workflow.
   - If changes are **docs-only or infrastructure-only** (plans, runbooks, internal guidance), skip bump/tag and continue.
   - Otherwise continue without bump/tag when SHIP is not required.

6. **Merge to default branch**
   - Checkout the default branch (e.g. `main`) and merge the current branch using a non-destructive merge (no rebase).
   - Stop on conflicts.

7. **Push (all three: feature branch, default branch, tags)**
   - Push the **current feature branch** to origin.
   - Push the default branch to origin.
   - Push tags (if a SHIP occurred or tags exist).
   - All three pushes must succeed. Report any failures immediately.

8. **Verify sync**
   - Confirm local default branch matches `origin/<default-branch>` (same commit SHA).
   - Confirm local feature branch matches `origin/<feature-branch>` (same commit SHA).
   - If either is out of sync, stop and report.

9. **Checkout feature branch**
   - Switch back to the feature branch so the working directory is on the correct branch for continued development.

10. **Summary**
    - Summarise: branch, commits created, tests run, merge result, and pushes performed.
    - Explicitly confirm: feature branch, default branch, and tags are all synced to origin.

Approval rules:

- Using the `handoff` trigger grants approval to push the **feature branch**, the default branch, and tags **for this workflow only**.
- Any other remote push still requires explicit approval.

---

## Handback Workflow (Resume on another machine)

Trigger: `handback` / `handback please`

Purpose: Restore the working environment on a different computer after a handoff, so development continues from exactly where it left off.

Behaviour (read-only, never pushes):

1. **Preflight (dirty tree guard)**
   - Check working tree status.
   - If uncommitted changes exist, **stop immediately** and warn the user to deal with local changes before proceeding.
   - Report current branch.

2. **Fetch**
   - Run `git fetch --all --prune --tags` to sync all remote state.

3. **Detect and checkout the active feature branch**
   - Auto-detect the branch from the last handoff: inspect remote branches sorted by most recent commit (`git branch -r --sort=-committerdate`), filter out `origin/<default-branch>` and `origin/HEAD`, and select the top result.
   - **Confirmation:** Explicitly state the detected branch and its last commit date, and ask the user to confirm before checking it out (mitigates the "committer date vs push date" edge case).
   - If already on the correct branch, skip checkout.
   - If on the default branch or a different branch, checkout the detected feature branch and set up tracking.
   - If detection is ambiguous, ask the user which branch to resume.

4. **Pull latest**
   - Fast-forward the feature branch to match the remote (`git pull --ff-only`).
   - Also update the local default branch to match its remote counterpart (`git checkout <default-branch> && git pull --ff-only && git checkout <feature-branch>`).
   - Stop on merge conflicts or non-fast-forward situations.

5. **Verify sync**
   - Confirm local feature branch matches `origin/<feature-branch>` (same commit SHA).
   - Confirm local default branch matches `origin/<default-branch>` (same commit SHA).
   - If either is out of sync, stop and report the discrepancy before proceeding.

6. **Verification gate**
   - Run the full verification suite per Project Profile:
     - Tests — 100% pass required.
     - Static checks (e.g. clippy) — zero warnings required.
     - IDE/editor diagnostics (e.g. VS Code Problems tab) — zero issues required.
   - Stop immediately on any failure and report.

7. **Summary**
   - Report: branch checked out, commits pulled in (with short log of new commits since local was last updated), verification results.
   - Explicitly confirm: feature branch and default branch are both in sync with origin.
   - Confirm: "Ready to continue where you left off."

Approval rules:

- Handback is entirely read-only — it fetches, checks out, and pulls but never pushes.
- No approval is required for any step.

---

## Status Workflow (Quick state check)

Trigger: `status` / `status please`

Purpose: Lightweight read-only report of the current repo state. Does not modify anything.

Behaviour:

1. **Fetch** (non-destructive)
   - Run `git fetch --all --prune --tags` to ensure remote refs are current.

2. **Report**
   - Current branch.
   - Working tree state (clean / number of uncommitted changes).
   - Current version (from `versionFiles` in `TCTBP.json`) and last tag.
   - Sync state: local vs remote SHA for current branch and the default branch.
   - Commits ahead/behind for both branches.
   - Whether a SHIP is needed (uncommitted changes or unshipped commits since last tag).

3. **Recommend next step(s)**
   - Provide 1–3 actionable recommendations with a one-line reason for each.
   - Use this priority order when multiple are valid: `abort` → `handback` → `ship` → `handoff` → `none`.
   - Recommendation rules:
     - `abort`: partial workflow state detected (e.g. merge in progress, bump/tag mismatch, previous workflow failed mid-way).
     - `handback`: local/remote branch SHA mismatch or default branch not synced with origin.
     - `ship`: unshipped commits since last tag, or version/tag drift detected.
     - `handoff`: branch is ahead or working tree is dirty and user likely needs to move machines.
     - `none`: repo is clean, synced, and no SHIP is needed.
   - Never execute recommended actions automatically from `status`; only report recommendations.

No approval required. No changes made.

---

## Abort Workflow (Partial operation recovery)

Trigger: `abort`

Purpose: Inspect and recover from a partially completed SHIP or handoff (e.g. bump committed but push failed, tag created but commit is wrong).

Behaviour:

1. **Inspect state**
   - Report current branch, working tree, last commit, last tag.
   - Identify whether a partial operation is in progress (e.g. version bumped but not tagged, tagged but not pushed, merge started but not completed).

2. **Propose recovery**
   - List specific recovery actions with consequences:
     - Revert the bump commit (`git revert` or `git reset --soft HEAD~1`).
     - Delete a local tag (`git tag -d vX.Y.Z`).
     - Abort a merge (`git merge --abort`).
   - Never execute recovery actions without explicit user approval.

3. **Execute approved actions**
   - Perform only the actions the user explicitly approves.
   - History rewriting (reset, force-push) requires extra confirmation.

Approval rules:

- All recovery actions require explicit approval.
- Force-push and history rewriting require double confirmation.

---

## SHIP / TCTBP Workflow

> SHIP = Preflight → Test → Problems → Bump → Commit → Tag → Push

### 1. Preflight

- Confirm current branch
- Confirm working tree state
- Confirm correct working directory

---

### 2. Test

Run repo test commands per Project Profile. Stop on failure.

**Skip condition:** If the change set is **docs-only or infrastructure-only**, skip this step entirely (there is no code to test).

---

### 3. Problems

Ensure lint, build, and test diagnostics are clean (zero warnings if enforced).

**Docs/infra-only changes:** Skip code-level checks (e.g. `cargo clippy`) but still run editor/IDE diagnostics (e.g. VS Code Problems tab) to catch syntax errors, markdown lint issues, and JSON validation errors in the changed files.

---

### 4. Bump Version

**Versioning rules:**

- **Z (patch)** increments on **every SHIP**, **except** when the change set is **docs-only or infrastructure-only** (plans, runbooks, internal guidance).
- **Y (minor)** increments on the **first SHIP of a new work branch**, resetting Z to 0, **only when the branch prefix matches `minorBranchPrefixes`** (default: `feature/`).
  - Operational definition: "first SHIP on a branch" means no prior shipped tag (`vX.Y.Z`) exists on commits unique to the current branch since it diverged from the default branch.
  - Branches with non-feature prefixes (e.g. `fix/`, `docs/`, `infrastructure/`) receive a **patch** bump on their first SHIP, not a minor bump.
- **X (major)** only by explicit instruction

The bump must be applied to all files listed in `versionFiles` in `TCTBP.json` **before committing**, so the resulting commit contains the new version.

---

### 5. Commit

- Stage relevant changes
- Propose a conventional commit message

During SHIP, the agent may proceed through **Bump → Commit → Tag** without pausing unless a core invariant fails.

---

### 5a. CHANGELOG (Optional)

If `CHANGELOG.md` exists in the repo and `changelogFormat` is specified in `TCTBP.json`:

- Propose an entry for the new version based on commits since the last tag.
- Use the conventional commit messages to categorise changes (e.g. feat, fix, docs, refactor).
- If format is `keep-a-changelog`, move items from the `[Unreleased]` section to a new `[vX.Y.Z]` heading.
- Include the entry in the same commit as the version bump.

If `CHANGELOG.md` does not exist, skip this step silently.

---

### 6. Tag

- Tag format: `vX.Y.Z` (example: `v0.5.27`)
- One tag per shipped commit
- Tag must point at the commit that introduced the version

---

### Build Profile

Builds performed during or after a SHIP use the **dev (debug) profile** by default (`cargo build`). A **release build** (`cargo build --release`) is only performed when the user explicitly requests it (e.g. "release build", "build release", "deploy release").

This keeps iteration fast during development and avoids unnecessary long compilation times.

---

### 7. Push (Approval Required)

- Push current branch only
- Never push to protected branches

**SHIP within handoff:** When SHIP runs as part of a handoff workflow, the handoff's push rules override this step. The handoff pushes all three (feature branch, main, tags) as a single operation — see Handoff Workflow step 7.

---

## Permissions Expectations (Authoritative)

### Allowed by Default

- Local file operations
- Tests, lint, build
- Commits and local tags
- Branch switching and merging
- **Non-destructive remote reads** (`fetch`, logs, diffs)
- **Handback operations** (fetch, checkout, pull) — entirely read-only, no approval needed

### Require Explicit Approval

- Push (any remote)
- Delete branches
- Force push
- Rewrite history
- Modify remotes

**Clarification:** There is no concept of a "push to a local branch". Local commits are always allowed; any `git push` that updates a remote always requires approval.

---

## Failure Behaviour

On any failure:

- Stop immediately
- Explain the failure
- Propose safe recovery options (revert bump commit, delete local tag)
- Never rewrite history without approval
- Suggest using `abort` trigger for guided recovery if the failure left partial state

**Merge Conflicts:** If a workflow stops due to a merge conflict (e.g. during Handoff or Branch creation), instruct the user to resolve the conflict manually, commit the resolution, and then re-trigger the workflow to complete the remaining steps.

---

## Appendix: `TCTBP.json` (Canonical Reference)

The authoritative JSON configuration is in `TCTBP.json` at the repo root's `.github/` folder. The template below is kept in sync for reference:

```json
{
  "schemaVersion": 3,
  "governance": {
    "sourceOfTruth": "TCTBP.json",
    "fallbackDocument": "TCTBP Agent.md"
  },
  "project": {
    "defaultBranch": "main",
    "versionFiles": ["Cargo.toml"],
    "changelogFormat": "keep-a-changelog"
  },
  "activation": {
    "triggers": ["ship", "ship please", "shipping", "tctbp", "prepare release", "handoff", "handoff please", "handback", "handback please", "status", "status please", "abort"],
    "caseInsensitive": true,
    "branchCommand": {
      "enabled": true,
      "pattern": "^branch\\s+(.+)$"
    }
  },
  "workflow": {
    "shipOrder": ["preflight", "test", "problems", "bump", "commit", "changelog", "tag", "push"],
    "handoffOrder": ["preflight", "stage", "test-gate", "commit", "ship-if-needed", "merge", "push", "verify-sync", "checkout-branch", "summary"],
    "handbackOrder": ["preflight", "fetch", "detect-branch", "pull", "verify-sync", "verification-gate", "summary"],
    "statusOrder": ["fetch", "report"],
    "abortOrder": ["inspect", "propose", "execute"],
    "requireApproval": ["push"]
  },
  "docsInfraPolicy": {
    "skipSteps": ["test", "clippy"],
    "keepSteps": ["preflight", "problems-editor", "commit", "push"],
    "filePatterns": ["*.md", "*.txt", "*.rst", "docs/**", ".github/**", "packaging/**", "LICENSE*", "CHANGELOG*", "CONTRIBUTING*", "*.toml", "*.json", "*.yaml", "*.yml"],
    "excludeWhenCodePresent": ["*.rs"],
    "comment": "For docs/infra-only changes: skip cargo test and clippy, keep editor diagnostics (e.g. markdown lint)"
  },
  "build": {
    "defaultProfile": "dev",
    "releaseOnlyWhenRequested": true,
    "comment": "Default to dev (debug) builds. Release builds only when user explicitly requests."
  },
  "versioning": {
    "scheme": "semver",
    "patchEveryShip": true,
    "skipForChangeTypes": ["docs-only", "infrastructure-only"],
    "minorOnFirstShipOfBranch": true,
    "minorBranchPrefixes": ["feature/"],
    "majorExplicitOnly": true
  },
  "tagging": {
    "policy": "everyCommit",
    "skipWhenNoBump": true,
    "format": "v{version}"
  }
}
```

