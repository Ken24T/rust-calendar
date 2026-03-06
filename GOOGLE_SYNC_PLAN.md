# Google Sync Plan

This document defines a two-stage implementation plan for Google Calendar sync in Rust Calendar:

- Stage 1: Enhanced read-only (RO) sync via ICS feeds
- Stage 2: Full CRUD sync via Google Calendar API

The plan is designed for incremental delivery, strict quality gates, and clear branch boundaries.

## Goals

1. Deliver a reliable, transparent, and safe RO sync experience first.
2. Add full create/update/delete behavior only after RO behavior is stable.
3. Keep local-first and cross-platform principles intact.
4. Preserve user trust with clear read-only/write indicators and conflict handling.

## Non-Goals

- No Stage 2 scope is started before Stage 1 acceptance criteria are met.
- No destructive behavior without explicit reconciliation safeguards.
- No hidden sync writes; all writes must be attributable and auditable.

## Architecture Principles

- Source-aware sync identity is mandatory (`source_id`, external ID mapping).
- Sync must be idempotent and resilient to malformed payloads.
- One failing source must not block other sources.
- Secrets must be redacted in logs and protected at rest where practical.
- UI must communicate ownership and mutability state clearly.

## Stage 1: Enhanced Read-Only Sync (ICS)

Stage 1 builds on the existing RO foundation and focuses on reliability, observability, and operator control.

### Stage 1 Functional Scope

- Multi-source Google ICS sync (manual and scheduled)
- Deterministic upsert and safe deletion reconciliation
- Read-only enforcement for synced events in UI
- Sync status visibility and diagnostics
- Guardrails for malformed or transient feed errors

### Stage 1 Implementation Slices

#### S1.1 Sync Health and Diagnostics

- Add per-source status panel: last success, last failure, next poll, last duration
- Show counters per run: created, updated, deleted, unchanged, skipped, errors
- Persist sync run summaries for troubleshooting

Exit criteria:
- User can identify source health without reading logs
- Latest sync run has actionable summary and error text

#### S1.2 Preview / Dry-Run Sync

- Add optional manual "Preview Sync" action
- Compute and display pending changes before apply
- Allow explicit confirmation before write phase

Exit criteria:
- Preview shows deterministic change set
- Apply result matches preview in normal conditions

#### S1.3 Selective Sync Controls

- Per-source date window controls (example: past 90 days, future 365 days)
- Option flags for cancelled events and source filtering behavior
- Validation and defaults to avoid accidental over-import

Exit criteria:
- User can limit imported horizon by source
- Re-sync respects source limits consistently

#### S1.4 Reconciliation Safety Window

- Introduce delayed deletion policy for not-seen entries
- Mark missing entries first; hard-delete only after grace threshold
- Add safeguards for suspicious zero-parse runs

Exit criteria:
- Temporary feed issues do not cause mass local deletions
- Deletions are explainable and traceable

#### S1.5 Recurrence and Instance Hardening (RO)

- Improve handling of recurring events with exceptions
- Preserve stable mapping for modified instances where possible
- Harden timezone and UTC/TZID edge cases

Exit criteria:
- Recurrence update behavior is stable across repeated sync runs
- Regression suite covers key recurrence and timezone edge cases

#### S1.6 Read-Only UX Improvements

- Stronger source badges and read-only indicators in event UI
- Explicit blocked-action messaging for edit/delete attempts
- Better source provenance in tooltips/detail views

Exit criteria:
- Users can immediately distinguish local vs synced events
- Blocked write actions are clear and non-confusing

#### S1.7 Security and Logging Hardening

- Redact URLs/secrets in all logs and errors
- Evaluate protected-at-rest storage for ICS secrets
- Add tests for redaction behavior

Exit criteria:
- No secret-bearing URLs in logs
- Error messages remain useful without leaking credentials

### Stage 1 Acceptance Criteria

- Two or more sources sync reliably with no duplicates
- Manual and scheduled sync both behave correctly
- Create/update/delete reflection from feed is stable and safe
- Read-only enforcement is complete in UI paths
- Sync diagnostics are visible and actionable
- `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo build` pass

## Stage 2: Full CRUD Sync (Google Calendar API)

Stage 2 introduces bidirectional synchronization using OAuth and Google Calendar API.

### Stage 2 Functional Scope

- OAuth2 sign-in and token lifecycle management
- Local create/update/delete propagation to Google
- Remote create/update/delete reflection to local
- Conflict detection and deterministic resolution policy
- Recurring series and exception-safe mutation behavior

### Stage 2 Implementation Slices

#### S2.1 OAuth and Account Linking

- Add account connection flow for Google Calendar API
- Store refresh/access tokens securely
- Support disconnect/reconnect and token revocation recovery

Exit criteria:
- User can connect and reconnect account reliably
- Token expiry and refresh behavior is robust

#### S2.2 Remote Identity and Sync Metadata

- Extend mapping model with remote event IDs, etags, sync tokens, hashes
- Track last-synced snapshots for conflict detection
- Support per-source capability flags (RO vs RW)

Exit criteria:
- Every synced event has stable remote identity metadata
- Incremental sync can resume efficiently

#### S2.3 Outbound Local -> Google Writes

- Push local creates/updates/deletes to Google API
- Retry with backoff for transient failures
- Surface pending/failed write states in UI

Exit criteria:
- Local edits are propagated reliably
- Failed writes are visible and retryable

#### S2.4 Inbound Incremental Google -> Local Sync

- Pull remote changes incrementally using sync tokens
- Reconcile changes with mapping and etag checks
- Keep idempotent behavior under repeated sync runs

Exit criteria:
- Incremental sync avoids full re-fetch in normal operation
- Repeated runs do not duplicate or drift event state

#### S2.5 Conflict Policy and Resolution UX

- Implement default conflict policy (recommended initial default: remote wins)
- Track conflict reasons and resolution outcome
- Add conflict review surface for manual override when needed

Exit criteria:
- Conflicts are deterministic and auditable
- User can inspect and resolve non-trivial conflicts

#### S2.6 Recurrence + Exceptions CRUD Parity

- Handle edits to single instance vs whole series
- Preserve exception integrity between local and remote models
- Harden DST/timezone behavior for recurring series

Exit criteria:
- Single-instance and series edits round-trip correctly
- Recurrence exception behavior is tested and stable

#### S2.7 Operational Hardening

- Quota/rate-limit handling and adaptive backoff
- Health metrics and sync run audit history improvements
- Migration and recovery playbook for broken mappings

Exit criteria:
- System remains stable under API quotas and partial outages
- Recovery from mapping/token issues is documented and practical

### Stage 2 Acceptance Criteria

- Full local CRUD propagates to Google reliably
- Remote CRUD reflects to local reliably
- Conflict policy is deterministic and user-visible
- Recurrence CRUD behavior is stable for series and instances
- OAuth/token lifecycle is production-safe
- Test, lint, and build gates pass with zero warnings

## Data and Schema Considerations

- Keep source metadata and mapping table as single source of sync truth.
- Add sync run history table for audit/debug support.
- Add staged deletion fields (first_missing_at, purge_after_at) for RO safety.
- Add capability and state fields for RW rollout (enabled_rw, last_push_at, pending_ops).

## Quality and Safety Gates

Before shipping any slice:

1. Unit + integration tests for new logic and edge cases
2. Regression tests for recurrence/timezone and deletion reconciliation
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `cargo test`
5. `cargo build`
6. Manual smoke checks for settings + sync status + event mutability UX

## Suggested Branch Strategy

Use separate branches exactly as agreed:

1. Stage 1 branch: `feature/google-sync-stage1-enhanced-ro`
2. Stage 2 branch (after Stage 1 is complete): `feature/google-sync-stage2-full-crud`

Optional slice-level short-lived branches can be used under each stage branch if needed.

## Shipping Cadence (Mandatory)

- Ship after every successful slice within a stage.
- Stage 1 examples: ship after S1.1, then ship after S1.2, and so on.
- Stage 2 examples: ship after S2.1, then ship after S2.2, and so on.
- Each slice ship must pass quality gates before merge/tag decisions.

Branch enforcement:

- All S1 work must remain on `feature/google-sync-stage1-enhanced-ro`.
- All S2 work must remain on `feature/google-sync-stage2-full-crud`.
- Stage 2 branch starts only after Stage 1 completion review and approval.

## Suggested Delivery Order

1. Complete Stage 1 slices S1.1 to S1.7
2. Validate Stage 1 acceptance criteria and ship
3. Create Stage 2 branch
4. Complete Stage 2 slices S2.1 to S2.7
5. Validate Stage 2 acceptance criteria and ship

## Risks and Mitigations

- Risk: accidental deletions from transient feed issues
  - Mitigation: staged deletion with grace window and suspicious-run guards
- Risk: confusing mutability state for users
  - Mitigation: explicit badges, lock states, and blocked-action messaging
- Risk: bidirectional conflict complexity
  - Mitigation: deterministic default policy plus conflict audit trail
- Risk: API quota and token churn
  - Mitigation: incremental sync, backoff, and robust token lifecycle handling

## Stage Status

### Stage 1 Completion Audit (2026-03-06)

- Status: complete
- Branch: `feature/google-sync-stage1-enhanced-ro`
- Latest Stage 1 release tag: `v2.4.10`

Slices shipped:

- [x] S1.1 Sync Health and Diagnostics (`v2.4.4`)
- [x] S1.2 Preview / Dry-Run Sync (`v2.4.5`)
- [x] S1.3 Selective Sync Controls (`v2.4.6`)
- [x] S1.4 Reconciliation Safety Window (`v2.4.7`)
- [x] S1.5 Recurrence and Instance Hardening (RO) (`v2.4.8`)
- [x] S1.6 Read-Only UX Improvements (`v2.4.9`)
- [x] S1.7 Security and Logging Hardening (`v2.4.10`)

Stage 1 acceptance criteria checkpoint:

- [x] Multi-source RO sync reliability and safe reconciliation
- [x] Manual and scheduled sync support
- [x] UI read-only enforcement and provenance clarity
- [x] Diagnostics/summary visibility for operators
- [x] Quality gates (`clippy`, `test`, `build`) for each shipped slice

### Stage 2 Activation

- Approved to start Stage 2 branch after Stage 1 completion review.

## Decision Log (Initial)

- Stage 1 remains strictly RO with safety and UX hardening.
- Stage 2 introduces full CRUD only through Google API (not ICS).
- Branching is stage-separated for clean review and release control.

