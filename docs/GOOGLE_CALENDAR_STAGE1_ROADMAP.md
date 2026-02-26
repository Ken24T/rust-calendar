# Google Calendar Stage 1 Roadmap (Read-Only Sync)

## Goal

Enable reliable **read-only** synchronisation from one or more Google Calendars into Rust Calendar using private ICS feeds.

Stage 1 must support:

- Adding multiple Google calendar ICS sources
- Scheduled pull sync and manual sync
- Create/update/delete reflection from Google into local data
- Read-only behaviour for synced events in local UI

## Non-Goals (Stage 1)

- Writing changes back to Google Calendar
- OAuth2 / Google Calendar API integration
- Two-way conflict resolution
- Offline edit queue for remote calendars

## Architecture Overview

Stage 1 introduces a source-aware sync pipeline built on existing iCalendar parsing:

1. Fetch ICS feed(s) over HTTPS
2. Parse events with stable external identifiers
3. Upsert into local database with source mapping
4. Reconcile deletions by source
5. Surface synced events as read-only in UI

## Data Model Changes

### New tables

1. `calendar_sources`
   - `id` (PK)
   - `name` (user-friendly source name)
   - `source_type` (`google_ics`)
   - `ics_url` (encrypted or protected-at-rest if feasible)
   - `enabled` (bool)
   - `poll_interval_minutes`
   - `last_sync_at`
   - `last_sync_status`
   - `last_error`
   - `created_at`, `updated_at`

2. `event_sync_map`
   - `id` (PK)
   - `source_id` (FK to `calendar_sources`)
   - `external_uid` (ICS `UID`)
   - `local_event_id` (FK to `events`)
   - `external_last_modified`
   - `external_etag_hash` (optional checksum/fingerprint)
   - `last_seen_at`
   - unique index on (`source_id`, `external_uid`)

### Event-level metadata (option A: table-only, preferred)

Keep `events` mostly unchanged and use `event_sync_map` for ownership/read-only checks.

## Service Layer Plan

### 1) Source Repository + Service

Add `services/calendar_sync/` with:

- Source CRUD (add/edit/remove/enable/disable)
- Validation for Google ICS URL format
- Poll scheduling metadata updates

### 2) ICS Fetcher

- Add HTTP client dependency (`reqwest` with rustls)
- Timeout, retries with backoff, and size guardrails
- Conditional fetch optional for later optimisation

### 3) Import Mapper (UID-aware)

Extend ICS import path to capture:

- `UID` (required for deterministic upsert)
- `LAST-MODIFIED` when present
- Better UTC/timezone handling for `Z` and TZID forms

### 4) Upsert + Reconciliation

Per source sync run:

- Parse all incoming events
- Upsert by (`source_id`, `external_uid`)
  - New UID -> create local event + map
  - Existing UID -> update local event fields
- Mark all seen UIDs with `last_seen_at = now`
- Deletion reconciliation:
  - previously mapped rows not seen in this run -> delete local event + map row (or soft-delete)

## UI/UX Plan (Minimal Stage 1)

### Settings dialog additions

- New “Google Calendar Sync (Read-Only)” section:
  - Add source (name + private ICS URL)
  - Enable/disable source
  - Poll interval selector (e.g. 5/15/30/60 min)
  - Manual “Sync now” button
  - Last sync status + timestamp + error text

### Calendar UI behaviour

- Synced events visually marked (small source badge/icon)
- Disable edit/delete actions for synced events
- Tooltip: “Read-only event from Google Calendar source”

## Reliability & Safety

- Never log full ICS secret URLs
- Store secrets carefully (minimum: redact in logs; better: encrypted at rest where practical)
- Fail one source independently without aborting all syncs
- Idempotent sync runs
- Guardrails for malformed ICS and oversized payloads

## Testing Strategy

1. Unit tests
   - Source validation, schedule decisions, UID upsert logic, deletion reconciliation
2. Integration tests (SQLite)
   - First sync import, repeated sync no duplicates, update propagation, delete propagation
3. Parser tests
   - UID parsing, LAST-MODIFIED parsing, UTC/TZ cases, recurring events baseline
4. Manual QA
   - Two or more Google calendars, create/update/delete in Google, verify local reflection

## Milestones

### M1: Foundations

- Add schema + migrations for `calendar_sources` and `event_sync_map`
- Add source repository/service and settings model wiring

### M2: Sync Engine

- Implement HTTP ICS fetcher
- Extend parser import model for UID + metadata
- Implement source-scoped upsert + deletion reconciliation

### M3: UI & Controls

- Settings UI for source management and manual sync
- Sync status/error surfaces
- Read-only enforcement in event interactions

### M4: Hardening

- Retry/backoff, robust error handling, logging redaction
- Tests and edge-case fixes
- Documentation and user guidance

## Acceptance Criteria (Stage 1 Done)

- User can configure at least two Google ICS sources
- Manual sync imports both without duplicates
- Scheduled sync updates changed events within configured interval
- Deleted Google events are removed locally on subsequent sync
- Synced events are read-only in local UI
- `cargo test` and `cargo clippy --all-targets --all-features -- -D warnings` pass

## Stage 2 Preview (Later)

After Stage 1 is stable, evaluate:

- OAuth2 + Google Calendar API
- Two-way sync semantics and conflict policy
- Per-event local override strategy
