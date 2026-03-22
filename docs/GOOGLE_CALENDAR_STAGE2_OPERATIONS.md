# Google Calendar Stage 2 Operations

This guide covers the operational behaviour of writable Google Calendar sync in
Stage 2, with emphasis on backoff states, sync history, broken mappings, and
safe recovery steps.

For first-time writable OAuth setup, see
[GOOGLE_OAUTH_SETUP.md](GOOGLE_OAUTH_SETUP.md).

## What Stage 2 Adds

Stage 2 enables writable Google Calendar sync through the Google Calendar API.
In practical terms, this means:

- local edits on writable synced events can be pushed outbound
- remote Google changes can be pulled back inbound
- recurrence exceptions and detached instances are tracked explicitly
- failures are recorded in sync history and the outbound queue

## Health Signals In Settings

The calendar sync settings section now shows:

- the latest source status
- recent sync runs with status, duration, counters, and any recorded error
- outbound queue counts for writable sources
- the latest outbound failure when a queue item is stuck

Use these signals before taking recovery actions. They distinguish temporary API
pressure from durable sync-identity problems.

## Backoff And Quota Behaviour

When Google requests throttling through `Retry-After`, the source status is
recorded as `backoff` instead of generic `failed`.

Expected operator behaviour:

1. Leave the source enabled.
2. Wait for the next scheduled retry window.
3. Check recent sync runs to confirm the source returns to `success`.

Do not use manual recovery for a normal `backoff` condition. It is a transient
quota or availability state, not a broken sync identity.

## Sync Token Recovery

If Google invalidates an incremental sync token, the sync engine automatically
falls back to a full incremental refresh path for that source. This is handled
internally and should not require manual operator action unless repeated failures
continue after the token reset.

If repeated failures persist after token recovery:

1. Check the most recent sync run errors in settings.
2. Confirm the Google account is still connected and authorised.
3. Re-run sync after the transient failure clears.

## Broken Mapping Symptoms

A broken mapping usually appears as an outbound failure mentioning
`missing remote_event_id`.

This means the local event is still present, but the stored remote identity for
that synced event is no longer usable.

Typical symptoms:

- `Retry Failed Pushes` reports that nothing retryable was reset
- the latest outbound error references `missing remote_event_id`
- a writable synced event stops accepting successful outbound updates

## Recovery Actions

### Retry Failed Pushes

Use `Retry Failed Pushes` only for transient failures such as temporary Google
errors or retryable backoff conditions.

It intentionally skips broken-mapping failures so the queue does not keep
reviving work that cannot succeed.

### Disconnect Broken Mapping

Use `Disconnect Broken Mapping` when the latest outbound failure is a broken
mapping (`missing remote_event_id`).

This action will:

- clear stale remote metadata
- remove the sync mapping for that event identity
- mark the failed outbound queue item complete
- leave the local event intact

This action will not:

- recreate the missing Google event identity
- re-link the event automatically
- push the existing local-only event back to Google

After disconnecting, treat the event as local-only unless you deliberately
recreate or reconcile it manually.

## Delete-Specific Recovery

Broken delete operations are handled more automatically than broken update
operations.

If a delete is already local-only and its remote identity is gone, the engine
clears stale sync tracking and completes the delete path without requiring a
manual recovery step.

## Recommended Operator Workflow

When a writable source shows trouble:

1. Check `Last status` and the recent sync run history.
2. If the status is `backoff`, wait for the scheduled retry.
3. If the latest outbound error is transient, use `Retry Failed Pushes`.
4. If the latest outbound error says `missing remote_event_id`, use
   `Disconnect Broken Mapping` only if you want to preserve the local event and
   stop treating it as remotely linked.
5. Verify the event state manually in Google Calendar if the remote copy still
   matters.

## Completion Note

With the runtime hardening shipped through `v2.4.32` and this operations
playbook in place, Stage 2.7 operational hardening is considered complete.