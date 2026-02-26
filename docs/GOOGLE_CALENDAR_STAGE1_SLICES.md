# Google Calendar Stage 1 (RO) - Implementation Slices

This document breaks Stage 1 into small, shippable slices. Each slice has a clear purpose, implementation scope, and exit criteria.

## Slice S1 - Source Registry Foundation

### What this slice does

Introduces persistent source configuration for one or more Google ICS feeds, without syncing events yet.

### Scope

- Add new DB table `calendar_sources`
- Add model(s) and repository/service methods for source CRUD
- Add URL validation (Google ICS URL shape)
- Add basic sync metadata fields (`last_sync_at`, `last_sync_status`, `last_error`)

### Key outputs

- DB migration + schema updates
- Service API for create/list/update/enable-disable/delete source
- Unit tests for validation and CRUD

### Exit criteria

- Can persist multiple enabled/disabled sources
- Validation rejects invalid URL inputs
- `cargo test` and strict clippy pass

---

## Slice S2 - External Identity Mapping

### What this slice does

Creates stable source-to-event identity mapping so imports become deterministic upserts instead of duplicate inserts.

### Scope

- Add new DB table `event_sync_map`
- Unique key on (`source_id`, `external_uid`)
- Service/repository methods to resolve map rows and update `last_seen_at`

### Key outputs

- Migration + schema + mapping repository
- Integration tests for uniqueness and lookup behaviour

### Exit criteria

- Mapping row can be created and resolved reliably by source + UID
- Duplicate UID for same source cannot create extra map rows

---

## Slice S3 - ICS Fetch + UID-Aware Parse

### What this slice does

Adds remote ICS download and parser enrichment so each incoming event includes stable external identity metadata.

### Scope

- Add HTTP fetcher (timeouts, retries, response size guard)
- Extend import flow to read `UID` and `LAST-MODIFIED`
- Improve datetime parsing for UTC `Z` and common TZID forms

### Key outputs

- Fetch service under `services/calendar_sync/`
- Parser test coverage for UID and timezone cases
- Redacted logging (no secret URL leakage)

### Exit criteria

- ICS payload fetch succeeds for valid URLs and fails safely for bad responses
- Parsed event envelope includes source UID metadata

---

## Slice S4 - Sync Engine (Upsert + Delete Reconciliation)

### What this slice does

Implements the actual read-only sync behaviour: create/update/delete reflection from Google feed into local events.

### Scope

- Per-source sync run orchestration
- Upsert by (`source_id`, `external_uid`)
- Maintain `last_seen_at`
- Reconcile deletions for rows not seen in latest run
- Update source sync status/error metadata

### Key outputs

- `sync_source(source_id)` and `sync_all_enabled_sources()` entry points
- Integration tests for create/update/delete reflection

### Exit criteria

- First sync imports events
- Second sync does not duplicate
- Feed updates propagate to local events
- Feed deletions remove mapped local events

---

## Slice S5 - Settings UI + Manual Sync

### What this slice does

Provides operator controls in the app to configure sources and run sync on demand.

### Scope

- Settings UI section for Google Calendar RO sources
- Add/edit/remove/enable-disable source controls
- Poll interval control
- Manual “Sync now” action per source and/or all sources
- Last sync status, timestamp, and error display

### Key outputs

- UI wiring to source + sync services
- Validation and user feedback for bad URLs and sync failures

### Exit criteria

- User can configure at least two sources in UI
- User can trigger manual sync and see status feedback

---

## Slice S6 - Read-Only Event Enforcement + Visual Marking

### What this slice does

Ensures synced events are visibly identified and protected from local edit/delete operations.

### Scope

- Determine “synced/read-only” from `event_sync_map`
- Disable or block edit/delete actions for synced events
- Add subtle source marker/badge and tooltip in event rendering

### Key outputs

- Guard clauses in event edit/delete paths
- UI marker for synced events
- Tests covering blocked edit/delete behaviour

### Exit criteria

- Synced events cannot be edited/deleted from local UI
- Tooltip/indicator clearly communicates read-only source

---

## Slice S7 - Scheduler + Hardening

### What this slice does

Adds scheduled polling and production guardrails for reliable background operation.

### Scope

- Timer-driven poll loop using configured intervals
- Per-source failure isolation (one bad source does not stop others)
- Retry/backoff policy and robust error handling
- Logging redaction and diagnostics

### Key outputs

- Background sync scheduler
- Telemetry/log messages for success/failure counts
- End-to-end tests for scheduling and partial failures

### Exit criteria

- Scheduled sync updates sources without manual action
- Failures are isolated and observable
- No secret ICS URLs appear in logs

---

## Recommended Delivery Sequence

1. S1 Source Registry Foundation
2. S2 External Identity Mapping
3. S3 ICS Fetch + UID-Aware Parse
4. S4 Sync Engine
5. S5 Settings UI + Manual Sync
6. S6 Read-Only Enforcement + Marking
7. S7 Scheduler + Hardening

## Stage 1 Completion Definition

Stage 1 is complete when S1-S7 are shipped and validated together:

- Multi-source Google ICS configuration works
- Manual and scheduled sync both work
- Create/update/delete reflection from Google is reliable
- Synced events are read-only locally
- Test + clippy gates pass with zero warnings
