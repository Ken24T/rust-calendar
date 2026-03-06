#![allow(dead_code)]

use std::collections::HashSet;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Local};
use rusqlite::Connection;

use crate::models::event::Event;
use crate::models::event_sync_map::EventSyncMap;
use crate::models::outbound_sync_operation::{
    OutboundSyncOperation, OUTBOUND_OPERATION_CREATE, OUTBOUND_OPERATION_DELETE,
};
use crate::models::sync_conflict::{
    SyncConflict, SYNC_CONFLICT_REASON_LOCAL_CREATE_PENDING,
    SYNC_CONFLICT_REASON_LOCAL_DELETE_PENDING, SYNC_CONFLICT_REASON_LOCAL_UPDATE_PENDING,
    SYNC_CONFLICT_RESOLUTION_REMOTE_WINS,
};
use crate::services::event::EventService;
use crate::services::google_account::GoogleAccountService;
use crate::services::icalendar::import::{self, ImportedIcsEvent};
use crate::services::outbound_sync::OutboundSyncService;
use crate::services::sync_conflict::SyncConflictService;

use super::fetcher::IcsFetcher;
use super::google_api::{
    GoogleCalendarApiClient, GoogleCalendarApiError, GoogleEventsSyncPayload,
    GoogleOutboundWriter, GoogleRemoteEvent,
};
use super::mapping::EventSyncMapService;
use super::sanitizer;
use super::{CalendarSourceService, SyncRunDiagnostics};
use crate::models::calendar_source::CalendarSource;
use thiserror::Error;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncRunResult {
    pub source_id: i64,
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub unchanged: usize,
    pub conflicts: usize,
    pub skipped_missing_uid: usize,
    pub skipped_duplicate_uid: usize,
    pub skipped_filtered: usize,
    pub error_count: usize,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncBatchResult {
    pub completed: Vec<SyncRunResult>,
    pub failed_sources: Vec<(i64, String)>,
}

pub struct CalendarSyncEngine<'a> {
    conn: &'a Connection,
    fetcher: IcsFetcher,
}

#[derive(Debug, Error)]
enum OutboundOperationError {
    #[error("Parent remote metadata for '{external_uid}' is missing remote_event_id")]
    MissingParentRemoteEventId { external_uid: String },
    #[error("Remote metadata for '{external_uid}' is missing remote_event_id")]
    MissingRemoteEventId { external_uid: String },
}

impl<'a> CalendarSyncEngine<'a> {
    pub fn new(conn: &'a Connection) -> Result<Self> {
        Ok(Self {
            conn,
            fetcher: IcsFetcher::new()?,
        })
    }

    pub fn sync_source(&self, source_id: i64) -> Result<SyncRunResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let started_at = chrono::Local::now();
        let timer = Instant::now();

        let result = if source.sync_capability
            == crate::models::calendar_source::SYNC_CAPABILITY_READ_WRITE
        {
            self.sync_source_from_google_api(source_id)
        } else {
            self.fetcher
                .fetch_ics(&source.ics_url)
                .and_then(|ics| self.sync_source_from_ics(source_id, &ics))
        };

        match result {
            Ok(mut success) => {
                let finished_at = chrono::Local::now();
                success.started_at = Some(started_at.to_rfc3339());
                success.finished_at = Some(finished_at.to_rfc3339());
                success.duration_ms = timer.elapsed().as_millis();
                success.error_count = 0;

                let diagnostics = SyncRunDiagnostics {
                    source_id,
                    started_at: success.started_at.clone().unwrap_or_default(),
                    finished_at: success.finished_at.clone().unwrap_or_default(),
                    status: "success".to_string(),
                    duration_ms: i64::try_from(success.duration_ms).unwrap_or(i64::MAX),
                    created_count: success.created as i64,
                    updated_count: success.updated as i64,
                    deleted_count: success.deleted as i64,
                    unchanged_count: success.unchanged as i64,
                    skipped_count: (success.skipped_missing_uid
                        + success.skipped_duplicate_uid
                        + success.skipped_filtered) as i64,
                    error_count: 0,
                    error_message: None,
                };

                source_service.update_sync_status_with_diagnostics(
                    source_id,
                    Some("success"),
                    None,
                    Some(&diagnostics),
                )?;

                Ok(success)
            }
            Err(err) => {
                let finished_at = chrono::Local::now();
                let redacted_error =
                    sanitizer::sanitize_error_message(&err.to_string(), &source.ics_url);
                let sync_status = Self::sync_status_for_error(&err);

                let diagnostics = SyncRunDiagnostics {
                    source_id,
                    started_at: started_at.to_rfc3339(),
                    finished_at: finished_at.to_rfc3339(),
                    status: sync_status.to_string(),
                    duration_ms: i64::try_from(timer.elapsed().as_millis()).unwrap_or(i64::MAX),
                    created_count: 0,
                    updated_count: 0,
                    deleted_count: 0,
                    unchanged_count: 0,
                    skipped_count: 0,
                    error_count: 1,
                    error_message: Some(redacted_error.clone()),
                };

                let _ = source_service.update_sync_status_with_diagnostics(
                    source_id,
                    Some(sync_status),
                    Some(&redacted_error),
                    Some(&diagnostics),
                );
                Err(anyhow!(redacted_error))
            }
        }
    }

    pub fn sync_source_from_ics(&self, source_id: i64, ics_content: &str) -> Result<SyncRunResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let imported = import::from_str_with_metadata(ics_content)?;

        if imported.is_empty() && ics_content.contains("BEGIN:VEVENT") {
            return Err(anyhow!(
                "ICS payload contained VEVENT markers but no events were parsed; aborting sync to avoid accidental deletions"
            ));
        }

        let (filtered, skipped_filtered) = Self::filter_imported_by_window(&source, imported);
        let mut result = self.apply_imported(source_id, filtered)?;
        result.skipped_filtered = skipped_filtered;
        Ok(result)
    }

    pub fn preview_source(&self, source_id: i64) -> Result<SyncRunResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        if source.sync_capability == crate::models::calendar_source::SYNC_CAPABILITY_READ_WRITE {
            self.preview_source_from_google_api(source_id)
        } else {
            self.fetcher
                .fetch_ics(&source.ics_url)
                .and_then(|ics| self.preview_source_from_ics(source_id, &ics))
        }
    }

    pub fn preview_source_from_ics(
        &self,
        source_id: i64,
        ics_content: &str,
    ) -> Result<SyncRunResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let imported = import::from_str_with_metadata(ics_content)?;

        if imported.is_empty() && ics_content.contains("BEGIN:VEVENT") {
            return Err(anyhow!(
                "ICS payload contained VEVENT markers but no events were parsed; aborting preview"
            ));
        }

        let (filtered, skipped_filtered) = Self::filter_imported_by_window(&source, imported);
        let mut result = self.preview_imported(source_id, filtered)?;
        result.skipped_filtered = skipped_filtered;
        Ok(result)
    }

    pub fn sync_all_enabled_sources(&self) -> Result<SyncBatchResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let sources = source_service.list_all()?;

        let mut batch = SyncBatchResult::default();
        for source in sources.into_iter().filter(|source| source.enabled) {
            let Some(source_id) = source.id else {
                continue;
            };

            match self.sync_source(source_id) {
                Ok(result) => batch.completed.push(result),
                Err(err) => batch.failed_sources.push((source_id, err.to_string())),
            }
        }

        Ok(batch)
    }

    pub(crate) fn sync_source_from_google_payload(
        &self,
        source_id: i64,
        payload: GoogleEventsSyncPayload,
    ) -> Result<SyncRunResult> {
        let mut result = self.reconcile_google_payload(source_id, &payload, true)?;
        let source_service = CalendarSourceService::new(self.conn);
        source_service.set_api_sync_token(source_id, payload.next_sync_token.as_deref())?;
        result.source_id = source_id;
        Ok(result)
    }

    pub(crate) fn preview_source_from_google_payload(
        &self,
        source_id: i64,
        payload: GoogleEventsSyncPayload,
    ) -> Result<SyncRunResult> {
        self.reconcile_google_payload(source_id, &payload, false)
    }

    fn apply_imported(
        &self,
        source_id: i64,
        imported_events: Vec<ImportedIcsEvent>,
    ) -> Result<SyncRunResult> {
        let mut result = SyncRunResult {
            source_id,
            ..SyncRunResult::default()
        };

        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let map_service = EventSyncMapService::new(self.conn);
        let event_service = EventService::new(self.conn);

        let mut seen_uids: HashSet<String> = HashSet::new();

        for imported in imported_events {
            let Some(uid) = Self::effective_uid(&imported) else {
                result.skipped_missing_uid += 1;
                continue;
            };

            if !seen_uids.insert(uid.clone()) {
                result.skipped_duplicate_uid += 1;
                continue;
            }

            match map_service.get_by_source_and_uid(source_id, &uid)? {
                Some(existing_map) => {
                    if let Some(existing_event) = event_service.get(existing_map.local_event_id)? {
                        if Self::is_effectively_unchanged(&existing_event, &imported.event) {
                            result.unchanged += 1;
                        } else {
                            let mut updated_event = imported.event.clone();
                            updated_event.id = existing_event.id;
                            updated_event.created_at = existing_event.created_at;
                            event_service.update(&updated_event)?;
                            result.updated += 1;
                        }
                    } else {
                        let created_event = event_service
                            .create(imported.event.clone())
                            .context("Failed to create event for stale mapping")?;

                        map_service
                            .delete_by_source_and_uid(source_id, &uid)
                            .context("Failed to remove stale mapping")?;

                        map_service
                            .create(EventSyncMap {
                                id: None,
                                source_id,
                                external_uid: uid.clone(),
                                local_event_id: created_event.id.ok_or_else(|| {
                                    anyhow!("Created event missing ID for mapping")
                                })?,
                                external_last_modified: imported.raw_last_modified.clone(),
                                external_etag_hash: None,
                                last_seen_at: Some(chrono::Local::now().to_rfc3339()),
                                first_missing_at: None,
                                purge_after_at: None,
                            })
                            .context("Failed to create replacement mapping")?;

                        result.created += 1;
                        continue;
                    }

                    map_service.touch_last_seen(source_id, &uid)?;
                }
                None => {
                    let created_event = event_service
                        .create(imported.event.clone())
                        .context("Failed to create imported event")?;

                    map_service
                        .create(EventSyncMap {
                            id: None,
                            source_id,
                            external_uid: uid.clone(),
                            local_event_id: created_event
                                .id
                                .ok_or_else(|| anyhow!("Created event missing ID for mapping"))?,
                            external_last_modified: imported.raw_last_modified.clone(),
                            external_etag_hash: None,
                            last_seen_at: Some(chrono::Local::now().to_rfc3339()),
                            first_missing_at: None,
                            purge_after_at: None,
                        })
                        .context("Failed to create event mapping")?;

                    result.created += 1;
                }
            }
        }

        let existing_maps = map_service.list_by_source_id(source_id)?;
        let now = Local::now();
        let grace_minutes = (source.poll_interval_minutes.max(1) * 3).max(30);
        for mapping in existing_maps {
            if !seen_uids.contains(&mapping.external_uid) {
                let should_purge = mapping
                    .purge_after_at
                    .as_deref()
                    .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
                    .map(|dt| now >= dt.with_timezone(&Local))
                    .unwrap_or(false);

                if should_purge {
                    map_service
                        .delete_by_source_and_uid(source_id, &mapping.external_uid)
                        .context("Failed to delete reconciled mapping")?;

                    if event_service.get(mapping.local_event_id)?.is_some() {
                        event_service
                            .delete(mapping.local_event_id)
                            .context("Failed to delete reconciled local event")?;
                    }
                    result.deleted += 1;
                } else {
                    let first_missing_at = now.to_rfc3339();
                    let purge_after_at = (now + Duration::minutes(grace_minutes)).to_rfc3339();
                    map_service
                        .mark_missing(
                            source_id,
                            &mapping.external_uid,
                            &first_missing_at,
                            &purge_after_at,
                        )
                        .context("Failed to stage reconciled deletion")?;
                }
            }
        }

        Ok(result)
    }

    fn sync_source_from_google_api(&self, source_id: i64) -> Result<SyncRunResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let account_service = GoogleAccountService::new(self.conn)?;
        let access_token = account_service.valid_access_token()?;
        let client = GoogleCalendarApiClient::new(access_token)?;

        self.process_pending_outbound_operations(&source, &client)?;

        match client.fetch_events_incremental(&source) {
            Ok(payload) => self.sync_source_from_google_payload(source_id, payload),
            Err(err) if Self::should_reset_api_sync_token(&err) => {
                source_service.set_api_sync_token(source_id, None)?;
                let refreshed_source = source_service
                    .get_by_id(source_id)?
                    .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;
                let payload = client.fetch_events_incremental(&refreshed_source)?;
                self.sync_source_from_google_payload(source_id, payload)
            }
            Err(err) => Err(err),
        }
    }

    fn preview_source_from_google_api(&self, source_id: i64) -> Result<SyncRunResult> {
        let source_service = CalendarSourceService::new(self.conn);
        let source = source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let account_service = GoogleAccountService::new(self.conn)?;
        let access_token = account_service.valid_access_token()?;
        let client = GoogleCalendarApiClient::new(access_token)?;

        let payload = match client.fetch_events_incremental(&source) {
            Ok(payload) => payload,
            Err(err) if Self::should_reset_api_sync_token(&err) => {
                let mut preview_source = source.clone();
                preview_source.api_sync_token = None;
                client.fetch_events_incremental(&preview_source)?
            }
            Err(err) => return Err(err),
        };

        self.preview_source_from_google_payload(source_id, payload)
    }

    fn process_pending_outbound_operations<W: GoogleOutboundWriter>(
        &self,
        source: &CalendarSource,
        writer: &W,
    ) -> Result<()> {
        let source_id = source
            .id
            .ok_or_else(|| anyhow!("Calendar source ID is required to process outbound operations"))?;
        let outbound_service = OutboundSyncService::new(self.conn);
        let operations = outbound_service.list_runnable_for_source(source_id, 100)?;

        for operation in operations {
            let Some(operation_id) = operation.id else {
                continue;
            };

            outbound_service.mark_operation_processing(operation_id)?;

            let result = self.process_single_outbound_operation(source, writer, &operation);
            match result {
                Ok(()) => {
                    outbound_service.mark_operation_completed(operation_id)?;
                    CalendarSourceService::new(self.conn).mark_last_push_now(source_id)?;
                }
                Err(err) => {
                    if Self::is_terminal_outbound_error(&err) {
                        outbound_service.mark_operation_failed(operation_id, &err.to_string())?;
                    } else {
                        outbound_service.mark_operation_failed_with_retry(
                            operation_id,
                            operation.attempt_count + 1,
                            source.poll_interval_minutes,
                            &err.to_string(),
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    fn process_single_outbound_operation<W: GoogleOutboundWriter>(
        &self,
        source: &CalendarSource,
        writer: &W,
        operation: &OutboundSyncOperation,
    ) -> Result<()> {
        let source_id = source
            .id
            .ok_or_else(|| anyhow!("Calendar source ID is required to push outbound operations"))?;
        let external_uid = operation
            .external_uid
            .as_deref()
            .ok_or_else(|| anyhow!("Outbound operation is missing external_uid"))?;
        let map_service = EventSyncMapService::new(self.conn);

        match operation.operation_type.as_str() {
            OUTBOUND_OPERATION_CREATE => {
                let payload_json = operation
                    .payload_json
                    .as_deref()
                    .ok_or_else(|| anyhow!("Outbound create operation is missing payload_json"))?;
                let (parent_external_uid, _) = external_uid.split_once("::RID::").ok_or_else(|| {
                    anyhow!(
                        "Outbound create for '{}' is unsupported without detached instance identity",
                        external_uid
                    )
                })?;
                let parent_remote = map_service
                    .get_remote_metadata(source_id, parent_external_uid)?
                    .and_then(|metadata| metadata.remote_event_id)
                    .ok_or_else(|| {
                        OutboundOperationError::MissingParentRemoteEventId {
                            external_uid: parent_external_uid.to_string(),
                        }
                    })?;
                let remote = writer.patch_detached_instance(
                    source,
                    &parent_remote,
                    external_uid,
                    payload_json,
                )?;
                self.complete_outbound_upsert(source_id, external_uid, operation.local_event_id, &remote)
            }
            OUTBOUND_OPERATION_DELETE => {
                let remote_event_id = map_service
                    .get_remote_metadata(source_id, external_uid)?
                    .and_then(|metadata| metadata.remote_event_id);

                if let Some(remote_event_id) = remote_event_id {
                    writer.delete_event(source, &remote_event_id)?;
                }

                self.clear_remote_identity_tracking(&map_service, source_id, external_uid)?;
                Ok(())
            }
            _ => {
                let payload_json = operation
                    .payload_json
                    .as_deref()
                    .ok_or_else(|| anyhow!("Outbound update operation is missing payload_json"))?;
                let remote_event_id = map_service
                    .get_remote_metadata(source_id, external_uid)?
                    .and_then(|metadata| metadata.remote_event_id)
                    .ok_or_else(|| {
                        OutboundOperationError::MissingRemoteEventId {
                            external_uid: external_uid.to_string(),
                        }
                    })?;
                let remote = writer.update_event(source, &remote_event_id, payload_json)?;
                self.complete_outbound_upsert(source_id, external_uid, operation.local_event_id, &remote)
            }
        }
    }

    fn complete_outbound_upsert(
        &self,
        source_id: i64,
        external_uid: &str,
        local_event_id: Option<i64>,
        remote: &GoogleRemoteEvent,
    ) -> Result<()> {
        let map_service = EventSyncMapService::new(self.conn);
        let local_event_id = local_event_id.ok_or_else(|| {
            anyhow!(
                "Outbound operation for '{}' is missing local_event_id",
                external_uid
            )
        })?;
        self.update_remote_tracking(&map_service, source_id, external_uid, local_event_id, remote)
    }

    fn clear_remote_identity_tracking(
        &self,
        map_service: &EventSyncMapService<'_>,
        source_id: i64,
        external_uid: &str,
    ) -> Result<()> {
        map_service.delete_remote_metadata(source_id, external_uid)?;

        if map_service
            .get_by_source_and_uid(source_id, external_uid)?
            .is_some()
        {
            map_service.delete_by_source_and_uid(source_id, external_uid)?;
        }

        Ok(())
    }

    fn reconcile_google_payload(
        &self,
        source_id: i64,
        payload: &GoogleEventsSyncPayload,
        apply: bool,
    ) -> Result<SyncRunResult> {
        let map_service = EventSyncMapService::new(self.conn);
        let event_service = EventService::new(self.conn);
        let outbound_service = OutboundSyncService::new(self.conn);
        let conflict_service = SyncConflictService::new(self.conn);
        let mut result = SyncRunResult {
            source_id,
            ..SyncRunResult::default()
        };

        for remote in &payload.items {
            let external_uid = remote.external_uid.clone();
            let active_outbound =
                outbound_service.active_operation_for_identity(source_id, &external_uid)?;
            let existing_map = map_service.get_by_source_and_uid(source_id, &external_uid)?;

            if remote.is_cancelled() {
                if let Some(mapping) = existing_map {
                    if let Some(operation) = active_outbound.as_ref() {
                        if Self::remote_matches_local_intent(operation, None, remote) {
                            if apply {
                                if let Some(operation_id) = operation.id {
                                    outbound_service.mark_operation_completed(operation_id)?;
                                }
                                conflict_service.resolve_open_for_identity(
                                    source_id,
                                    &external_uid,
                                    SYNC_CONFLICT_RESOLUTION_REMOTE_WINS,
                                )?;
                            }
                        } else {
                            result.conflicts += 1;
                            if apply {
                                self.record_remote_wins_conflict(
                                    &conflict_service,
                                    &outbound_service,
                                    &external_uid,
                                    Some(mapping.local_event_id),
                                    operation,
                                    "delete",
                                )?;
                            }
                        }
                    }

                    if apply {
                        map_service.delete_by_source_and_uid(source_id, &external_uid)?;
                        map_service.delete_remote_metadata(source_id, &external_uid)?;
                        if event_service.get(mapping.local_event_id)?.is_some() {
                            event_service.delete(mapping.local_event_id)?;
                        }
                    }
                    result.deleted += 1;
                } else if let Some(operation) = active_outbound.as_ref() {
                    if Self::remote_matches_local_intent(operation, None, remote) {
                        if apply {
                            if let Some(operation_id) = operation.id {
                                outbound_service.mark_operation_completed(operation_id)?;
                            }
                            conflict_service.resolve_open_for_identity(
                                source_id,
                                &external_uid,
                                SYNC_CONFLICT_RESOLUTION_REMOTE_WINS,
                            )?;
                            map_service.delete_remote_metadata(source_id, &external_uid)?;
                        }
                        result.deleted += 1;
                    }
                }
                continue;
            }

            let incoming_event = remote.event.clone().ok_or_else(|| {
                anyhow!("Google event payload missing local event representation")
            })?;

            match existing_map {
                Some(mapping) => {
                    if let Some(existing_event) = event_service.get(mapping.local_event_id)? {
                        if let Some(operation) = active_outbound.as_ref() {
                            if Self::remote_matches_local_intent(
                                operation,
                                Some(&existing_event),
                                remote,
                            ) {
                                if apply {
                                    if let Some(operation_id) = operation.id {
                                        outbound_service.mark_operation_completed(operation_id)?;
                                    }
                                    conflict_service.resolve_open_for_identity(
                                        source_id,
                                        &external_uid,
                                        SYNC_CONFLICT_RESOLUTION_REMOTE_WINS,
                                    )?;
                                }
                            } else {
                                result.conflicts += 1;
                                if apply {
                                    self.record_remote_wins_conflict(
                                        &conflict_service,
                                        &outbound_service,
                                        &external_uid,
                                        existing_event.id.or(Some(mapping.local_event_id)),
                                        operation,
                                        "update",
                                    )?;
                                }
                            }
                        }

                        if Self::is_effectively_unchanged(&existing_event, &incoming_event) {
                            if apply {
                                self.update_remote_tracking(
                                    &map_service,
                                    source_id,
                                    &external_uid,
                                    existing_event.id.unwrap_or(mapping.local_event_id),
                                    remote,
                                )?;
                            }
                            result.unchanged += 1;
                        } else {
                            if apply {
                                let mut updated_event = incoming_event.clone();
                                updated_event.id = existing_event.id;
                                updated_event.created_at = existing_event.created_at;
                                event_service.update(&updated_event)?;
                                self.update_remote_tracking(
                                    &map_service,
                                    source_id,
                                    &external_uid,
                                    updated_event.id.unwrap_or(mapping.local_event_id),
                                    remote,
                                )?;
                            }
                            result.updated += 1;
                        }
                    } else {
                        if let Some(operation) = active_outbound.as_ref() {
                            result.conflicts += 1;
                            if apply {
                                self.record_remote_wins_conflict(
                                    &conflict_service,
                                    &outbound_service,
                                    &external_uid,
                                    Some(mapping.local_event_id),
                                    operation,
                                    "update",
                                )?;
                            }
                        }

                        if apply {
                            let created_event = event_service.create(incoming_event.clone())?;
                            self.update_remote_tracking(
                                &map_service,
                                source_id,
                                &external_uid,
                                created_event
                                    .id
                                    .ok_or_else(|| anyhow!("Created event missing ID"))?,
                                remote,
                            )?;
                        }
                        result.created += 1;
                    }
                }
                None => {
                    if apply {
                        let created_event = event_service.create(incoming_event.clone())?;
                        let local_event_id = created_event
                            .id
                            .ok_or_else(|| anyhow!("Created event missing ID for mapping"))?;
                        map_service.create(EventSyncMap {
                            id: None,
                            source_id,
                            external_uid: external_uid.clone(),
                            local_event_id,
                            external_last_modified: remote.updated_at.clone(),
                            external_etag_hash: remote.etag.clone(),
                            last_seen_at: Some(Local::now().to_rfc3339()),
                            first_missing_at: None,
                            purge_after_at: None,
                        })?;
                        self.update_remote_tracking(
                            &map_service,
                            source_id,
                            &external_uid,
                            local_event_id,
                            remote,
                        )?;
                    }
                    result.created += 1;
                }
            }
        }

        Ok(result)
    }

    fn preview_imported(
        &self,
        source_id: i64,
        imported_events: Vec<ImportedIcsEvent>,
    ) -> Result<SyncRunResult> {
        let mut result = SyncRunResult {
            source_id,
            ..SyncRunResult::default()
        };

        let source_service = CalendarSourceService::new(self.conn);
        source_service
            .get_by_id(source_id)?
            .ok_or_else(|| anyhow!("Calendar source with id {} not found", source_id))?;

        let map_service = EventSyncMapService::new(self.conn);
        let event_service = EventService::new(self.conn);

        let mut seen_uids: HashSet<String> = HashSet::new();

        for imported in imported_events {
            let Some(uid) = Self::effective_uid(&imported) else {
                result.skipped_missing_uid += 1;
                continue;
            };

            if !seen_uids.insert(uid.clone()) {
                result.skipped_duplicate_uid += 1;
                continue;
            }

            match map_service.get_by_source_and_uid(source_id, &uid)? {
                Some(existing_map) => {
                    if let Some(existing_event) = event_service.get(existing_map.local_event_id)? {
                        if Self::is_effectively_unchanged(&existing_event, &imported.event) {
                            result.unchanged += 1;
                        } else {
                            result.updated += 1;
                        }
                    } else {
                        // Mapping exists but points to missing event; apply path would recreate it.
                        result.created += 1;
                    }
                }
                None => {
                    result.created += 1;
                }
            }
        }

        let existing_maps = map_service.list_by_source_id(source_id)?;
        for mapping in existing_maps {
            if !seen_uids.contains(&mapping.external_uid) {
                result.deleted += 1;
            }
        }

        Ok(result)
    }

    fn is_effectively_unchanged(existing: &Event, incoming: &Event) -> bool {
        existing.title == incoming.title
            && existing.description == incoming.description
            && existing.location == incoming.location
            && existing.start == incoming.start
            && existing.end == incoming.end
            && existing.all_day == incoming.all_day
            && existing.category == incoming.category
            && existing.color == incoming.color
            && existing.recurrence_rule == incoming.recurrence_rule
            && existing.recurrence_exceptions == incoming.recurrence_exceptions
    }

    fn filter_imported_by_window(
        source: &crate::models::calendar_source::CalendarSource,
        imported_events: Vec<ImportedIcsEvent>,
    ) -> (Vec<ImportedIcsEvent>, usize) {
        let now = Local::now();
        let past_cutoff = now - Duration::days(source.sync_past_days.max(0));
        let future_cutoff = now + Duration::days(source.sync_future_days.max(1));

        let mut kept = Vec::with_capacity(imported_events.len());
        let mut skipped = 0usize;

        for imported in imported_events {
            let in_window =
                imported.event.end >= past_cutoff && imported.event.start <= future_cutoff;
            if in_window {
                kept.push(imported);
            } else {
                skipped += 1;
            }
        }

        (kept, skipped)
    }

    fn effective_uid(imported: &ImportedIcsEvent) -> Option<String> {
        let base_uid = imported
            .uid
            .as_deref()
            .map(str::trim)
            .filter(|uid| !uid.is_empty())?
            .to_string();

        match imported
            .recurrence_id
            .as_deref()
            .map(str::trim)
            .filter(|rid| !rid.is_empty())
        {
            Some(rid) => Some(format!("{}::RID::{}", base_uid, rid)),
            None => Some(base_uid),
        }
    }

    fn update_remote_tracking(
        &self,
        map_service: &EventSyncMapService<'_>,
        source_id: i64,
        external_uid: &str,
        local_event_id: i64,
        remote: &GoogleRemoteEvent,
    ) -> Result<()> {
        map_service.update_mapping_state(
            source_id,
            external_uid,
            local_event_id,
            remote.updated_at.as_deref(),
            remote.etag.as_deref(),
        )?;

        map_service.upsert_remote_metadata(&super::mapping::RemoteEventMetadata {
            source_id,
            external_uid: external_uid.to_string(),
            remote_event_id: Some(remote.remote_event_id.clone()),
            remote_etag: remote.etag.clone(),
            remote_payload_hash: Some(remote.payload_hash.clone()),
        })?;

        Ok(())
    }

    fn remote_matches_local_intent(
        operation: &OutboundSyncOperation,
        existing_event: Option<&Event>,
        remote: &GoogleRemoteEvent,
    ) -> bool {
        match operation.operation_type.as_str() {
            OUTBOUND_OPERATION_DELETE => remote.is_cancelled(),
            _ => {
                let Some(local_event) = existing_event else {
                    return false;
                };

                if remote.is_cancelled() {
                    return false;
                }

                let Some(incoming_event) = remote.event.as_ref() else {
                    return false;
                };

                Self::is_effectively_unchanged(local_event, incoming_event)
            }
        }
    }

    fn record_remote_wins_conflict(
        &self,
        conflict_service: &SyncConflictService<'_>,
        outbound_service: &OutboundSyncService<'_>,
        external_uid: &str,
        local_event_id: Option<i64>,
        operation: &OutboundSyncOperation,
        remote_change_type: &str,
    ) -> Result<()> {
        if let Some(operation_id) = operation.id {
            outbound_service.mark_operation_failed(
                operation_id,
                "Conflict detected during Google sync: remote change was applied and local change was kept for manual review",
            )?;
        }

        conflict_service.upsert_open(&SyncConflict {
            id: None,
            source_id: operation.source_id,
            local_event_id,
            external_uid: external_uid.to_string(),
            outbound_operation_id: operation.id,
            local_operation_type: Some(operation.operation_type.clone()),
            remote_change_type: remote_change_type.to_string(),
            reason: Self::conflict_reason_for_operation(&operation.operation_type).to_string(),
            resolution: Some(SYNC_CONFLICT_RESOLUTION_REMOTE_WINS.to_string()),
            status: crate::models::sync_conflict::SYNC_CONFLICT_STATUS_OPEN.to_string(),
            created_at: None,
            resolved_at: None,
            updated_at: None,
        })?;

        Ok(())
    }

    fn conflict_reason_for_operation(operation_type: &str) -> &'static str {
        match operation_type {
            crate::models::outbound_sync_operation::OUTBOUND_OPERATION_CREATE => {
                SYNC_CONFLICT_REASON_LOCAL_CREATE_PENDING
            }
            OUTBOUND_OPERATION_DELETE => SYNC_CONFLICT_REASON_LOCAL_DELETE_PENDING,
            _ => SYNC_CONFLICT_REASON_LOCAL_UPDATE_PENDING,
        }
    }

    fn should_reset_api_sync_token(err: &anyhow::Error) -> bool {
        err.downcast_ref::<GoogleCalendarApiError>()
            .is_some_and(GoogleCalendarApiError::is_sync_token_expired)
    }

    fn sync_status_for_error(err: &anyhow::Error) -> &'static str {
        if err
            .downcast_ref::<GoogleCalendarApiError>()
            .and_then(GoogleCalendarApiError::retry_after_minutes)
            .is_some()
        {
            return "backoff";
        }

        "failed"
    }

    fn is_terminal_outbound_error(err: &anyhow::Error) -> bool {
        err.downcast_ref::<OutboundOperationError>().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::{CalendarSyncEngine, OutboundOperationError};
    use anyhow::anyhow;
    use crate::models::calendar_source::SYNC_CAPABILITY_READ_WRITE;
    use crate::models::outbound_sync_operation::{
        OUTBOUND_OPERATION_CREATE, OUTBOUND_OPERATION_DELETE, OUTBOUND_OPERATION_UPDATE,
        OUTBOUND_STATUS_FAILED,
    };
    use crate::models::sync_conflict::{
        SYNC_CONFLICT_RESOLUTION_REMOTE_WINS, SYNC_CONFLICT_STATUS_OPEN,
    };
    use crate::services::calendar_sync::mapping::EventSyncMapService;
    use crate::services::database::Database;
    use crate::services::event::EventService;
    use crate::services::calendar_sync::google_api::GoogleCalendarApiError;
    use chrono::{Duration, Local, TimeZone, Utc};
    use rusqlite::{params, Connection};
    use crate::models::calendar_source::CalendarSource;
    use crate::services::calendar_sync::google_api::{GoogleOutboundWriter, GoogleRemoteEvent};

    fn create_source(conn: &Connection, name: &str, enabled: bool) -> i64 {
        conn.execute(
            "INSERT INTO calendar_sources (name, source_type, ics_url, enabled, poll_interval_minutes)
             VALUES (?1, ?2, ?3, ?4, 15)",
            params![
                name,
                "google_ics",
                "https://calendar.google.com/calendar/ical/test%40gmail.com/private-token/basic.ics",
                enabled as i32,
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn create_rw_source(conn: &Connection, name: &str) -> i64 {
        conn.execute(
            "INSERT INTO calendar_sources (
                name, source_type, ics_url, enabled, poll_interval_minutes, sync_capability
             ) VALUES (?1, 'google_ics', ?2, 1, 15, ?3)",
            params![
                name,
                "https://calendar.google.com/calendar/ical/test%40gmail.com/private-token/basic.ics",
                SYNC_CAPABILITY_READ_WRITE,
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn test_should_reset_api_sync_token_only_for_expired_sync_token_errors() {
        let expired = anyhow!(GoogleCalendarApiError::SyncTokenExpired);
        let rate_limited = anyhow!(GoogleCalendarApiError::RetryAfter {
            status_code: 429,
            retry_after_minutes: 30,
        });

        assert!(CalendarSyncEngine::should_reset_api_sync_token(&expired));
        assert!(!CalendarSyncEngine::should_reset_api_sync_token(&rate_limited));
    }

    #[test]
    fn test_sync_status_for_error_marks_retry_after_errors_as_backoff() {
        let rate_limited = anyhow!(GoogleCalendarApiError::RetryAfter {
            status_code: 503,
            retry_after_minutes: 15,
        });
        let generic = anyhow!("plain failure");

        assert_eq!(CalendarSyncEngine::sync_status_for_error(&rate_limited), "backoff");
        assert_eq!(CalendarSyncEngine::sync_status_for_error(&generic), "failed");
    }

    #[test]
    fn test_is_terminal_outbound_error_detects_broken_remote_metadata() {
        let broken = anyhow!(OutboundOperationError::MissingRemoteEventId {
            external_uid: "uid-broken".to_string(),
        });
        let transient = anyhow!("temporary outage");

        assert!(CalendarSyncEngine::is_terminal_outbound_error(&broken));
        assert!(!CalendarSyncEngine::is_terminal_outbound_error(&transient));
    }

    fn set_source_windows(conn: &Connection, source_id: i64, past_days: i64, future_days: i64) {
        conn.execute(
            "UPDATE calendar_sources SET sync_past_days = ?1, sync_future_days = ?2 WHERE id = ?3",
            params![past_days, future_days, source_id],
        )
        .unwrap();
    }

    #[derive(Default)]
    struct FakeGoogleOutboundWriter {
        updated_ids: std::sync::Mutex<Vec<String>>,
        deleted_ids: std::sync::Mutex<Vec<String>>,
        patched_instances: std::sync::Mutex<Vec<(String, String)>>,
    }

    impl GoogleOutboundWriter for FakeGoogleOutboundWriter {
        fn update_event(
            &self,
            _source: &CalendarSource,
            remote_event_id: &str,
            _payload_json: &str,
        ) -> anyhow::Result<GoogleRemoteEvent> {
            self.updated_ids
                .lock()
                .unwrap()
                .push(remote_event_id.to_string());
            Ok(GoogleRemoteEvent {
                remote_event_id: remote_event_id.to_string(),
                external_uid: "uid-api-series".to_string(),
                etag: Some("\"etag-updated\"".to_string()),
                updated_at: Some("2026-03-06T01:00:00Z".to_string()),
                payload_hash: "hash-updated".to_string(),
                status: Some("confirmed".to_string()),
                event: Some(
                    crate::models::event::Event::builder()
                        .title("Series")
                        .start(Local::now())
                        .end(Local::now() + Duration::hours(1))
                        .build()
                        .unwrap(),
                ),
            })
        }

        fn delete_event(
            &self,
            _source: &CalendarSource,
            remote_event_id: &str,
        ) -> anyhow::Result<()> {
            self.deleted_ids
                .lock()
                .unwrap()
                .push(remote_event_id.to_string());
            Ok(())
        }

        fn patch_detached_instance(
            &self,
            _source: &CalendarSource,
            parent_remote_event_id: &str,
            detached_external_uid: &str,
            _payload_json: &str,
        ) -> anyhow::Result<GoogleRemoteEvent> {
            self.patched_instances.lock().unwrap().push((
                parent_remote_event_id.to_string(),
                detached_external_uid.to_string(),
            ));
            Ok(GoogleRemoteEvent {
                remote_event_id: "remote-instance-1".to_string(),
                external_uid: detached_external_uid.to_string(),
                etag: Some("\"etag-instance\"".to_string()),
                updated_at: Some("2026-03-06T02:00:00Z".to_string()),
                payload_hash: "hash-instance".to_string(),
                status: Some("confirmed".to_string()),
                event: Some(
                    crate::models::event::Event::builder()
                        .title("Detached")
                        .start(Local::now())
                        .end(Local::now() + Duration::hours(1))
                        .build()
                        .unwrap(),
                ),
            })
        }
    }

    #[test]
    fn test_sync_source_from_ics_creates_and_updates_events() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let ics_first = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-100
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Original Title
    LAST-MODIFIED:20260227T000000Z
    END:VEVENT
    END:VCALENDAR"#;

        let result_first = engine.sync_source_from_ics(source_id, ics_first).unwrap();
        assert_eq!(result_first.created, 1);
        assert_eq!(result_first.updated, 0);

        let ics_second = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-100
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Updated Title
    LAST-MODIFIED:20260227T010000Z
    END:VEVENT
    END:VCALENDAR"#;

        let result_second = engine.sync_source_from_ics(source_id, ics_second).unwrap();
        assert_eq!(result_second.created, 0);
        assert_eq!(result_second.updated, 1);

        let event_service = EventService::new(conn);
        let all_events = event_service.list_all().unwrap();
        assert_eq!(all_events.len(), 1);
        assert_eq!(all_events[0].title, "Updated Title");
    }

    #[test]
    fn test_sync_source_from_ics_reconciles_deleted_events() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let ics_initial = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-a
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Event A
    END:VEVENT
    BEGIN:VEVENT
    UID:uid-b
    DTSTART:20260227T110000
    DTEND:20260227T120000
    SUMMARY:Event B
    END:VEVENT
    END:VCALENDAR"#;

        let initial = engine.sync_source_from_ics(source_id, ics_initial).unwrap();
        assert_eq!(initial.created, 2);

        let ics_next = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-a
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Event A
    END:VEVENT
    END:VCALENDAR"#;

        let next = engine.sync_source_from_ics(source_id, ics_next).unwrap();
        assert_eq!(next.deleted, 0);

        // First missing run should stage deletion, not purge immediately.
        let staged_maps = EventSyncMapService::new(conn)
            .list_by_source_id(source_id)
            .unwrap();
        let staged = staged_maps
            .iter()
            .find(|m| m.external_uid == "uid-b")
            .expect("uid-b mapping should be staged for deletion");
        assert!(staged.first_missing_at.is_some());
        assert!(staged.purge_after_at.is_some());

        // Force grace window expiry and run again to purge.
        conn.execute(
            "UPDATE event_sync_map SET purge_after_at = ?1 WHERE source_id = ?2 AND external_uid = ?3",
            params!["2000-01-01T00:00:00+00:00", source_id, "uid-b"],
        )
        .unwrap();

        let purge = engine.sync_source_from_ics(source_id, ics_next).unwrap();
        assert_eq!(purge.deleted, 1);

        let event_service = EventService::new(conn);
        let all_events = event_service.list_all().unwrap();
        assert_eq!(all_events.len(), 1);
        assert_eq!(all_events[0].title, "Event A");

        let map_service = EventSyncMapService::new(conn);
        let maps = map_service.list_by_source_id(source_id).unwrap();
        assert_eq!(maps.len(), 1);
        assert_eq!(maps[0].external_uid, "uid-a");
    }

    #[test]
    fn test_sync_source_from_ics_skips_missing_uid() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let ics = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:No UID Event
    END:VEVENT
    END:VCALENDAR"#;

        let result = engine.sync_source_from_ics(source_id, ics).unwrap();
        assert_eq!(result.skipped_missing_uid, 1);
        assert_eq!(result.created, 0);

        let event_service = EventService::new(conn);
        assert_eq!(event_service.list_all().unwrap().len(), 0);
    }

    #[test]
    fn test_sync_source_from_ics_aborts_when_vevent_present_but_nothing_parsed() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let good_ics = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-safe
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Existing Event
    END:VEVENT
    END:VCALENDAR"#;

        let initial = engine.sync_source_from_ics(source_id, good_ics).unwrap();
        assert_eq!(initial.created, 1);

        let suspicious_ics = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-safe
    DTSTART:20260227T090000
    DTEND:20260227T100000
    END:VEVENT
    END:VCALENDAR"#;

        let result = engine.sync_source_from_ics(source_id, suspicious_ics);
        assert!(result.is_err());

        let event_service = EventService::new(conn);
        let all_events = event_service.list_all().unwrap();
        assert_eq!(all_events.len(), 1);
        assert_eq!(all_events[0].title, "Existing Event");
    }

    #[test]
    fn test_sync_source_from_ics_counts_unchanged() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let ics = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-200
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Stable Event
    END:VEVENT
    END:VCALENDAR"#;

        let first = engine.sync_source_from_ics(source_id, ics).unwrap();
        assert_eq!(first.created, 1);

        let second = engine.sync_source_from_ics(source_id, ics).unwrap();
        assert_eq!(second.unchanged, 1);
        assert_eq!(second.updated, 0);
    }

    #[test]
    fn test_preview_source_from_ics_reports_changes_without_writing() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let ics = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:preview-1
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Preview Event
    END:VEVENT
    END:VCALENDAR"#;

        let preview = engine.preview_source_from_ics(source_id, ics).unwrap();
        assert_eq!(preview.created, 1);
        assert_eq!(preview.updated, 0);
        assert_eq!(preview.deleted, 0);

        // Preview must not persist events or mappings.
        let event_service = EventService::new(conn);
        assert_eq!(event_service.list_all().unwrap().len(), 0);

        let map_service = EventSyncMapService::new(conn);
        assert_eq!(map_service.list_by_source_id(source_id).unwrap().len(), 0);
    }

    #[test]
    fn test_preview_source_from_ics_reports_delete_candidates() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let initial = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-a
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Event A
    END:VEVENT
    BEGIN:VEVENT
    UID:uid-b
    DTSTART:20260227T110000
    DTEND:20260227T120000
    SUMMARY:Event B
    END:VEVENT
    END:VCALENDAR"#;

        let _ = engine.sync_source_from_ics(source_id, initial).unwrap();

        let next = r#"BEGIN:VCALENDAR
    VERSION:2.0
    BEGIN:VEVENT
    UID:uid-a
    DTSTART:20260227T090000
    DTEND:20260227T100000
    SUMMARY:Event A
    END:VEVENT
    END:VCALENDAR"#;

        let preview = engine.preview_source_from_ics(source_id, next).unwrap();
        assert_eq!(preview.deleted, 1);

        // Preview must not apply deletions.
        let event_service = EventService::new(conn);
        assert_eq!(event_service.list_all().unwrap().len(), 2);
    }

    #[test]
    fn test_sync_source_from_ics_respects_source_date_window() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);
        set_source_windows(conn, source_id, 0, 1);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let now = Local::now();
        let in_window_start = now + Duration::hours(6);
        let in_window_end = in_window_start + Duration::hours(1);
        let out_window_start = now + Duration::days(5);
        let out_window_end = out_window_start + Duration::hours(1);

        let ics = format!(
            "BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nUID:keep-uid\nDTSTART:{}\nDTEND:{}\nSUMMARY:Keep Event\nEND:VEVENT\nBEGIN:VEVENT\nUID:skip-uid\nDTSTART:{}\nDTEND:{}\nSUMMARY:Skip Event\nEND:VEVENT\nEND:VCALENDAR",
            in_window_start.format("%Y%m%dT%H%M%S"),
            in_window_end.format("%Y%m%dT%H%M%S"),
            out_window_start.format("%Y%m%dT%H%M%S"),
            out_window_end.format("%Y%m%dT%H%M%S"),
        );

        let result = engine.sync_source_from_ics(source_id, &ics).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.skipped_filtered, 1);

        let event_service = EventService::new(conn);
        let events = event_service.list_all().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Keep Event");
    }

    #[test]
    fn test_sync_source_from_ics_keeps_modified_instances_with_same_uid() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_source(conn, "Work", true);

        let engine = CalendarSyncEngine::new(conn).unwrap();

        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:series-uid
DTSTART:20260310T090000
DTEND:20260310T100000
SUMMARY:Series Master
END:VEVENT
BEGIN:VEVENT
UID:series-uid
RECURRENCE-ID:20260311T090000
DTSTART:20260311T120000
DTEND:20260311T130000
SUMMARY:Moved Instance
END:VEVENT
END:VCALENDAR"#;

        let result = engine.sync_source_from_ics(source_id, ics).unwrap();
        assert_eq!(result.created, 2);
        assert_eq!(result.skipped_duplicate_uid, 0);

        let event_service = EventService::new(conn);
        let events = event_service.list_all().unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_sync_source_from_google_payload_creates_event_and_sets_sync_token() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let engine = CalendarSyncEngine::new(conn).unwrap();

        let payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-1",
                        "etag": "\"etag-1\"",
                        "status": "confirmed",
                        "summary": "Inbound API Event",
                        "iCalUID": "uid-api-1",
                        "updated": "2026-03-06T00:00:00Z",
                        "start": { "dateTime": "2026-03-10T09:00:00Z" },
                        "end": { "dateTime": "2026-03-10T10:00:00Z" }
                    }
                ],
                "nextSyncToken": "sync-token-1"
            }"#,
            )
            .unwrap();

        let result = engine
            .sync_source_from_google_payload(source_id, payload)
            .unwrap();
        assert_eq!(result.created, 1);

        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        assert_eq!(source.api_sync_token.as_deref(), Some("sync-token-1"));

        let events = EventService::new(conn).list_all().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Inbound API Event");
    }

    #[test]
    fn test_sync_source_from_google_payload_deletes_cancelled_event() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let engine = CalendarSyncEngine::new(conn).unwrap();

        let initial_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-2",
                        "etag": "\"etag-2\"",
                        "status": "confirmed",
                        "summary": "Delete Me",
                        "iCalUID": "uid-api-2",
                        "updated": "2026-03-06T00:00:00Z",
                        "start": { "dateTime": "2026-03-10T09:00:00Z" },
                        "end": { "dateTime": "2026-03-10T10:00:00Z" }
                    }
                ],
                "nextSyncToken": "sync-token-2"
            }"#,
            )
            .unwrap();
        engine
            .sync_source_from_google_payload(source_id, initial_payload)
            .unwrap();

        let delete_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-2",
                        "status": "cancelled",
                        "iCalUID": "uid-api-2"
                    }
                ],
                "nextSyncToken": "sync-token-3"
            }"#,
            )
            .unwrap();

        let result = engine
            .sync_source_from_google_payload(source_id, delete_payload)
            .unwrap();
        assert_eq!(result.deleted, 1);
        assert!(EventService::new(conn).list_all().unwrap().is_empty());
        assert!(EventSyncMapService::new(conn)
            .list_by_source_id(source_id)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_sync_source_from_google_payload_records_remote_wins_conflict() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let engine = CalendarSyncEngine::new(conn).unwrap();

        let initial_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-3",
                        "etag": "\"etag-3\"",
                        "status": "confirmed",
                        "summary": "Remote Baseline",
                        "iCalUID": "uid-api-3",
                        "updated": "2026-03-06T00:00:00Z",
                        "start": { "dateTime": "2026-03-10T09:00:00Z" },
                        "end": { "dateTime": "2026-03-10T10:00:00Z" }
                    }
                ],
                "nextSyncToken": "sync-token-4"
            }"#,
            )
            .unwrap();
        engine
            .sync_source_from_google_payload(source_id, initial_payload)
            .unwrap();

        let existing = EventService::new(conn).list_all().unwrap().remove(0);
        let mut local_edit = existing.clone();
        local_edit.title = "Local Edit".to_string();
        EventService::new(conn).update_local(&local_edit).unwrap();

        let operation = crate::services::outbound_sync::OutboundSyncService::new(conn)
            .active_operation_for_identity(source_id, "uid-api-3")
            .unwrap()
            .unwrap();
        assert_eq!(operation.operation_type, OUTBOUND_OPERATION_UPDATE);

        let remote_update_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-3",
                        "etag": "\"etag-4\"",
                        "status": "confirmed",
                        "summary": "Remote Edit",
                        "iCalUID": "uid-api-3",
                        "updated": "2026-03-07T00:00:00Z",
                        "start": { "dateTime": "2026-03-10T09:00:00Z" },
                        "end": { "dateTime": "2026-03-10T10:00:00Z" }
                    }
                ],
                "nextSyncToken": "sync-token-5"
            }"#,
            )
            .unwrap();

        let result = engine
            .sync_source_from_google_payload(source_id, remote_update_payload)
            .unwrap();

        assert_eq!(result.updated, 1);
        assert_eq!(result.conflicts, 1);

        let updated = EventService::new(conn).list_all().unwrap().remove(0);
        assert_eq!(updated.title, "Remote Edit");

        let queue_status: String = conn
            .query_row(
                "SELECT status FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, "uid-api-3"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(queue_status, OUTBOUND_STATUS_FAILED);

        let conflict: (String, String, String) = conn
            .query_row(
                "SELECT status, resolution, reason
                 FROM sync_conflicts
                 WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, "uid-api-3"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(conflict.0, SYNC_CONFLICT_STATUS_OPEN);
        assert_eq!(conflict.1, SYNC_CONFLICT_RESOLUTION_REMOTE_WINS);
        assert_eq!(
            conflict.2,
            crate::models::sync_conflict::SYNC_CONFLICT_REASON_LOCAL_UPDATE_PENDING
        );
    }

    #[test]
    fn test_sync_source_from_google_payload_preserves_recurrence_exceptions() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let engine = CalendarSyncEngine::new(conn).unwrap();

        let payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-series",
                        "etag": "\"etag-series\"",
                        "status": "confirmed",
                        "summary": "Series With EXDATE",
                        "iCalUID": "uid-api-exdate",
                        "updated": "2026-03-06T00:00:00Z",
                        "recurrence": [
                            "RRULE:FREQ=WEEKLY;BYDAY=TU",
                            "EXDATE:20260317T090000Z"
                        ],
                        "start": { "dateTime": "2026-03-10T09:00:00Z" },
                        "end": { "dateTime": "2026-03-10T10:00:00Z" }
                    }
                ],
                "nextSyncToken": "sync-token-exdate"
            }"#,
            )
            .unwrap();

        let result = engine
            .sync_source_from_google_payload(source_id, payload)
            .unwrap();
        assert_eq!(result.created, 1);

        let events = EventService::new(conn).list_all().unwrap();
        assert_eq!(events.len(), 1);
        let exceptions = events[0].recurrence_exceptions.as_ref().unwrap();
        assert_eq!(exceptions.len(), 1);
        assert_eq!(
            exceptions[0].with_timezone(&chrono::Utc).to_rfc3339(),
            "2026-03-17T09:00:00+00:00"
        );
    }

    #[test]
    fn test_sync_source_from_google_payload_round_trips_timed_series_recurrence_exceptions_after_outbound_update() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let engine = CalendarSyncEngine::new(conn).unwrap();

        let initial_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-series-exdate-timed",
                        "etag": "\"etag-series-timed-1\"",
                        "status": "confirmed",
                        "summary": "Timed Series",
                        "iCalUID": "uid-series-exdate-timed",
                        "updated": "2026-03-06T00:00:00Z",
                        "recurrence": [
                            "RRULE:FREQ=WEEKLY;BYDAY=TU",
                            "EXDATE:20260317T090000Z"
                        ],
                        "start": { "dateTime": "2026-03-10T09:00:00Z" },
                        "end": { "dateTime": "2026-03-10T10:00:00Z" }
                    }
                ],
                "nextSyncToken": "sync-token-series-timed-1"
            }"#,
            )
            .unwrap();
        engine
            .sync_source_from_google_payload(source_id, initial_payload)
            .unwrap();

        let existing = EventService::new(conn).list_all().unwrap().remove(0);
        let mut local_edit = existing.clone();
        local_edit.recurrence_exceptions = Some(vec![
            chrono::Utc
                .with_ymd_and_hms(2026, 3, 17, 9, 0, 0)
                .unwrap()
                .with_timezone(&Local),
            chrono::Utc
                .with_ymd_and_hms(2026, 3, 24, 9, 0, 0)
                .unwrap()
                .with_timezone(&Local),
        ]);
        EventService::new(conn).update_local(&local_edit).unwrap();

        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        let writer = FakeGoogleOutboundWriter::default();
        engine
            .process_pending_outbound_operations(&source, &writer)
            .unwrap();

        let round_trip_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-series-exdate-timed",
                        "etag": "\"etag-series-timed-2\"",
                        "status": "confirmed",
                        "summary": "Timed Series",
                        "iCalUID": "uid-series-exdate-timed",
                        "updated": "2026-03-06T03:00:00Z",
                        "recurrence": [
                            "RRULE:FREQ=WEEKLY;BYDAY=TU",
                            "EXDATE:20260317T090000Z,20260324T090000Z"
                        ],
                        "start": { "dateTime": "2026-03-10T09:00:00Z" },
                        "end": { "dateTime": "2026-03-10T10:00:00Z" }
                    }
                ],
                "nextSyncToken": "sync-token-series-timed-2"
            }"#,
            )
            .unwrap();

        let result = engine
            .sync_source_from_google_payload(source_id, round_trip_payload)
            .unwrap();

        assert_eq!(result.unchanged, 1);
        assert_eq!(result.conflicts, 0);

        let refreshed = EventService::new(conn).get(existing.id.unwrap()).unwrap().unwrap();
        let exceptions = refreshed.recurrence_exceptions.unwrap();
        assert_eq!(exceptions.len(), 2);
        assert_eq!(
            exceptions[0].with_timezone(&chrono::Utc).to_rfc3339(),
            "2026-03-17T09:00:00+00:00"
        );
        assert_eq!(
            exceptions[1].with_timezone(&chrono::Utc).to_rfc3339(),
            "2026-03-24T09:00:00+00:00"
        );
    }

    #[test]
    fn test_sync_source_from_google_payload_round_trips_all_day_series_recurrence_exceptions_after_outbound_update() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let engine = CalendarSyncEngine::new(conn).unwrap();

        let initial_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-series-exdate-allday",
                        "etag": "\"etag-series-allday-1\"",
                        "status": "confirmed",
                        "summary": "All Day Series",
                        "iCalUID": "uid-series-exdate-allday",
                        "updated": "2026-03-06T00:00:00Z",
                        "recurrence": [
                            "RRULE:FREQ=WEEKLY;BYDAY=WE",
                            "EXDATE;VALUE=DATE:20260311"
                        ],
                        "start": { "date": "2026-03-04" },
                        "end": { "date": "2026-03-05" }
                    }
                ],
                "nextSyncToken": "sync-token-series-allday-1"
            }"#,
            )
            .unwrap();
        engine
            .sync_source_from_google_payload(source_id, initial_payload)
            .unwrap();

        let existing = EventService::new(conn).list_all().unwrap().remove(0);
        let mut local_edit = existing.clone();
        local_edit.recurrence_exceptions = Some(vec![
            Local.with_ymd_and_hms(2026, 3, 11, 0, 0, 0).unwrap(),
            Local.with_ymd_and_hms(2026, 3, 18, 0, 0, 0).unwrap(),
        ]);
        EventService::new(conn).update_local(&local_edit).unwrap();

        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        let writer = FakeGoogleOutboundWriter::default();
        engine
            .process_pending_outbound_operations(&source, &writer)
            .unwrap();

        let round_trip_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-series-exdate-allday",
                        "etag": "\"etag-series-allday-2\"",
                        "status": "confirmed",
                        "summary": "All Day Series",
                        "iCalUID": "uid-series-exdate-allday",
                        "updated": "2026-03-06T03:00:00Z",
                        "recurrence": [
                            "RRULE:FREQ=WEEKLY;BYDAY=WE",
                            "EXDATE;VALUE=DATE:20260311,20260318"
                        ],
                        "start": { "date": "2026-03-04" },
                        "end": { "date": "2026-03-05" }
                    }
                ],
                "nextSyncToken": "sync-token-series-allday-2"
            }"#,
            )
            .unwrap();

        let result = engine
            .sync_source_from_google_payload(source_id, round_trip_payload)
            .unwrap();

        assert_eq!(result.unchanged, 1);
        assert_eq!(result.conflicts, 0);

        let refreshed = EventService::new(conn).get(existing.id.unwrap()).unwrap().unwrap();
        let exceptions = refreshed.recurrence_exceptions.unwrap();
        assert_eq!(exceptions.len(), 2);
        assert_eq!(exceptions[0].date_naive().to_string(), "2026-03-11");
        assert_eq!(exceptions[1].date_naive().to_string(), "2026-03-18");
    }

    #[test]
    fn test_process_pending_outbound_operations_updates_series_and_marks_push_time() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let engine = CalendarSyncEngine::new(conn).unwrap();

        let initial_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-series-1",
                        "etag": "\"etag-1\"",
                        "status": "confirmed",
                        "summary": "Series",
                        "iCalUID": "uid-api-series",
                        "updated": "2026-03-06T00:00:00Z",
                        "start": { "dateTime": "2026-03-10T09:00:00Z" },
                        "end": { "dateTime": "2026-03-10T10:00:00Z" }
                    }
                ],
                "nextSyncToken": "sync-token-series"
            }"#,
            )
            .unwrap();
        engine
            .sync_source_from_google_payload(source_id, initial_payload)
            .unwrap();

        let existing = EventService::new(conn).list_all().unwrap().remove(0);
        let mut local_edit = existing.clone();
        local_edit.title = "Series Local Edit".to_string();
        EventService::new(conn).update_local(&local_edit).unwrap();

        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        let writer = FakeGoogleOutboundWriter::default();
        engine
            .process_pending_outbound_operations(&source, &writer)
            .unwrap();

        let updated_ids = writer.updated_ids.lock().unwrap().clone();
        assert_eq!(updated_ids, vec!["remote-series-1".to_string()]);

        let queue_status: String = conn
            .query_row(
                "SELECT status FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, "uid-api-series"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(queue_status, crate::models::outbound_sync_operation::OUTBOUND_STATUS_COMPLETED);

        let last_push_at: Option<String> = conn
            .query_row(
                "SELECT last_push_at FROM calendar_sources WHERE id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(last_push_at.is_some());
    }

    #[test]
    fn test_process_pending_outbound_operations_patches_detached_instance_create() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        let engine = CalendarSyncEngine::new(conn).unwrap();
        let event_service = EventService::new(conn);

        let start = Local.with_ymd_and_hms(2026, 3, 10, 9, 0, 0).unwrap();
        let end = start + Duration::hours(1);
        let mut series = crate::models::event::Event::new("Series", start, end).unwrap();
        series.recurrence_rule = Some("FREQ=WEEKLY;BYDAY=TU".to_string());
        let series = event_service.create(series).unwrap();
        let series_id = series.id.unwrap();

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id) VALUES (?1, ?2, ?3)",
            params![source_id, "uid-parent", series_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_remote_metadata (source_id, external_uid, remote_event_id, remote_etag, remote_payload_hash, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![source_id, "uid-parent", "remote-parent-1", "\"etag-parent\"", "hash-parent", "2026-03-06T00:00:00Z"],
        )
        .unwrap();

        let detached_start = Local.with_ymd_and_hms(2026, 4, 7, 9, 0, 0).unwrap();
        let detached_end = detached_start + Duration::hours(2);
        let detached = event_service
            .create(
                crate::models::event::Event::builder()
                    .title("Detached")
                    .start(detached_start)
                    .end(detached_end)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let detached_id = detached.id.unwrap();
        let detached_uid = format!(
            "uid-parent::RID::{}",
            detached_start.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")
        );

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id) VALUES (?1, ?2, ?3)",
            params![source_id, detached_uid, detached_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, local_event_id, external_uid, operation_type, payload_json, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
            params![
                source_id,
                detached_id,
                detached_uid,
                OUTBOUND_OPERATION_CREATE,
                serde_json::json!({
                    "title": "Detached",
                    "start": detached_start.to_rfc3339(),
                    "end": detached_end.to_rfc3339(),
                    "all_day": false,
                    "recurrence_rule": null,
                    "recurrence_exceptions": null
                })
                .to_string(),
            ],
        )
        .unwrap();

        let writer = FakeGoogleOutboundWriter::default();
        engine
            .process_pending_outbound_operations(&source, &writer)
            .unwrap();

        let patched = writer.patched_instances.lock().unwrap().clone();
        assert_eq!(patched.len(), 1);
        assert_eq!(patched[0].0, "remote-parent-1");
        assert!(patched[0].1.contains("::RID::"));

        let remote_event_id: Option<String> = conn
            .query_row(
                "SELECT remote_event_id FROM event_remote_metadata WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, patched[0].1.clone()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remote_event_id.as_deref(), Some("remote-instance-1"));
    }

    #[test]
    fn test_sync_source_from_google_payload_completes_matching_detached_create_without_conflict() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let engine = CalendarSyncEngine::new(conn).unwrap();
        let event_service = EventService::new(conn);

        let detached_start = Local.with_ymd_and_hms(2026, 4, 28, 9, 0, 0).unwrap();
        let detached_end = detached_start + Duration::hours(2);
        let detached = event_service
            .create(
                crate::models::event::Event::builder()
                    .title("Detached Race")
                    .start(detached_start)
                    .end(detached_end)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let detached_id = detached.id.unwrap();
        let detached_uid = format!(
            "uid-parent::RID::{}",
            detached_start.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")
        );

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id) VALUES (?1, ?2, ?3)",
            params![source_id, detached_uid, detached_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, local_event_id, external_uid, operation_type, payload_json, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
            params![
                source_id,
                detached_id,
                detached_uid,
                OUTBOUND_OPERATION_CREATE,
                serde_json::json!({
                    "title": "Detached Race",
                    "start": detached_start.to_rfc3339(),
                    "end": detached_end.to_rfc3339(),
                    "all_day": false,
                    "recurrence_rule": null,
                    "recurrence_exceptions": null
                })
                .to_string(),
            ],
        )
        .unwrap();

        let payload = super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
            &format!(
                r#"{{
                "items": [
                    {{
                        "id": "remote-instance-race-1",
                        "etag": "\"etag-race\"",
                        "status": "confirmed",
                        "summary": "Detached Race",
                        "iCalUID": "uid-parent",
                        "updated": "2026-03-06T04:00:00Z",
                        "originalStartTime": {{ "dateTime": "{}" }},
                        "start": {{ "dateTime": "{}" }},
                        "end": {{ "dateTime": "{}" }}
                    }}
                ],
                "nextSyncToken": "sync-token-race"
            }}"#,
                detached_start.with_timezone(&Utc).to_rfc3339(),
                detached_start.with_timezone(&Utc).to_rfc3339(),
                detached_end.with_timezone(&Utc).to_rfc3339(),
            ),
        )
        .unwrap();

        let result = engine
            .sync_source_from_google_payload(source_id, payload)
            .unwrap();

        assert_eq!(result.unchanged, 1);
        assert_eq!(result.conflicts, 0);

        let queue_status: String = conn
            .query_row(
                "SELECT status FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(queue_status, crate::models::outbound_sync_operation::OUTBOUND_STATUS_COMPLETED);

        let remote_event_id: Option<String> = conn
            .query_row(
                "SELECT remote_event_id FROM event_remote_metadata WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remote_event_id.as_deref(), Some("remote-instance-race-1"));
    }

    #[test]
    fn test_process_pending_outbound_operations_retries_due_failed_operation() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let engine = CalendarSyncEngine::new(conn).unwrap();

        let initial_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-series-2",
                        "etag": "\"etag-1\"",
                        "status": "confirmed",
                        "summary": "Series",
                        "iCalUID": "uid-api-retry",
                        "updated": "2026-03-06T00:00:00Z",
                        "start": { "dateTime": "2026-03-10T09:00:00Z" },
                        "end": { "dateTime": "2026-03-10T10:00:00Z" }
                    }
                ],
                "nextSyncToken": "sync-token-retry"
            }"#,
            )
            .unwrap();
        engine
            .sync_source_from_google_payload(source_id, initial_payload)
            .unwrap();

        let existing = EventService::new(conn).list_all().unwrap().remove(0);
        let mut local_edit = existing.clone();
        local_edit.title = "Retry Me".to_string();
        EventService::new(conn).update_local(&local_edit).unwrap();

        conn.execute(
            "UPDATE outbound_sync_operations
             SET status = 'failed', next_retry_at = '2000-01-01T00:00:00+00:00', attempt_count = 1
             WHERE source_id = ?1 AND external_uid = ?2",
            params![source_id, "uid-api-retry"],
        )
        .unwrap();

        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        let writer = FakeGoogleOutboundWriter::default();
        engine
            .process_pending_outbound_operations(&source, &writer)
            .unwrap();

        let updated_ids = writer.updated_ids.lock().unwrap().clone();
        assert_eq!(updated_ids, vec!["remote-series-2".to_string()]);

        let queue_status: String = conn
            .query_row(
                "SELECT status FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, "uid-api-retry"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(queue_status, crate::models::outbound_sync_operation::OUTBOUND_STATUS_COMPLETED);
    }

    #[test]
    fn test_process_pending_outbound_operations_updates_detached_instance_using_remote_metadata() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        let engine = CalendarSyncEngine::new(conn).unwrap();
        let event_service = EventService::new(conn);

        let detached_start = Local.with_ymd_and_hms(2026, 4, 7, 9, 0, 0).unwrap();
        let detached_end = detached_start + Duration::hours(2);
        let detached = event_service
            .create(
                crate::models::event::Event::builder()
                    .title("Detached")
                    .start(detached_start)
                    .end(detached_end)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let detached_id = detached.id.unwrap();
        let detached_uid = format!(
            "uid-parent::RID::{}",
            detached_start.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")
        );

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id) VALUES (?1, ?2, ?3)",
            params![source_id, detached_uid, detached_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_remote_metadata (source_id, external_uid, remote_event_id, remote_etag, remote_payload_hash, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![source_id, detached_uid, "remote-instance-update-1", "\"etag-before\"", "hash-before", "2026-03-06T00:00:00Z"],
        )
        .unwrap();

        let mut updated_detached = detached.clone();
        updated_detached.title = "Detached Updated".to_string();
        event_service.update_local(&updated_detached).unwrap();

        let writer = FakeGoogleOutboundWriter::default();
        engine
            .process_pending_outbound_operations(&source, &writer)
            .unwrap();

        let updated_ids = writer.updated_ids.lock().unwrap().clone();
        assert_eq!(updated_ids, vec!["remote-instance-update-1".to_string()]);

        let queue_status: String = conn
            .query_row(
                "SELECT status FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(queue_status, crate::models::outbound_sync_operation::OUTBOUND_STATUS_COMPLETED);
    }

    #[test]
    fn test_process_pending_outbound_operations_marks_missing_remote_metadata_as_terminal_failed() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        let engine = CalendarSyncEngine::new(conn).unwrap();

        let initial_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-series-broken-1",
                        "etag": "\"etag-broken-1\"",
                        "status": "confirmed",
                        "summary": "Series",
                        "iCalUID": "uid-api-broken",
                        "updated": "2026-03-06T00:00:00Z",
                        "start": { "dateTime": "2026-03-10T09:00:00Z" },
                        "end": { "dateTime": "2026-03-10T10:00:00Z" }
                    }
                ],
                "nextSyncToken": "sync-token-broken"
            }"#,
            )
            .unwrap();
        engine
            .sync_source_from_google_payload(source_id, initial_payload)
            .unwrap();

        conn.execute(
            "UPDATE event_remote_metadata SET remote_event_id = NULL WHERE source_id = ?1 AND external_uid = ?2",
            params![source_id, "uid-api-broken"],
        )
        .unwrap();

        let existing = EventService::new(conn).list_all().unwrap().remove(0);
        let mut local_edit = existing.clone();
        local_edit.title = "Broken Metadata Edit".to_string();
        EventService::new(conn).update_local(&local_edit).unwrap();

        let writer = FakeGoogleOutboundWriter::default();
        engine
            .process_pending_outbound_operations(&source, &writer)
            .unwrap();

        let (status, next_retry_at, last_error): (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT status, next_retry_at, last_error FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, "uid-api-broken"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(status, crate::models::outbound_sync_operation::OUTBOUND_STATUS_FAILED);
        assert!(next_retry_at.is_none());
        assert!(last_error
            .as_deref()
            .is_some_and(|error| error.contains("missing remote_event_id")));

        let updated_ids = writer.updated_ids.lock().unwrap().clone();
        assert!(updated_ids.is_empty());
    }

    #[test]
    fn test_process_pending_outbound_operations_completes_delete_when_remote_metadata_is_missing() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        let engine = CalendarSyncEngine::new(conn).unwrap();

        let initial_payload =
            super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
                r#"{
                "items": [
                    {
                        "id": "remote-series-delete-1",
                        "etag": "\"etag-delete-1\"",
                        "status": "confirmed",
                        "summary": "Series",
                        "iCalUID": "uid-api-delete-broken",
                        "updated": "2026-03-06T00:00:00Z",
                        "start": { "dateTime": "2026-03-10T09:00:00Z" },
                        "end": { "dateTime": "2026-03-10T10:00:00Z" }
                    }
                ],
                "nextSyncToken": "sync-token-delete-broken"
            }"#,
            )
            .unwrap();
        engine
            .sync_source_from_google_payload(source_id, initial_payload)
            .unwrap();

        conn.execute(
            "UPDATE event_remote_metadata SET remote_event_id = NULL WHERE source_id = ?1 AND external_uid = ?2",
            params![source_id, "uid-api-delete-broken"],
        )
        .unwrap();

        let existing = EventService::new(conn).list_all().unwrap().remove(0);
        EventService::new(conn).delete_local(existing.id.unwrap()).unwrap();

        let writer = FakeGoogleOutboundWriter::default();
        engine
            .process_pending_outbound_operations(&source, &writer)
            .unwrap();

        let status: String = conn
            .query_row(
                "SELECT status FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, "uid-api-delete-broken"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, crate::models::outbound_sync_operation::OUTBOUND_STATUS_COMPLETED);

        let metadata_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_remote_metadata WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, "uid-api-delete-broken"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(metadata_exists, 0);

        let mapping_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_sync_map WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, "uid-api-delete-broken"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(mapping_exists, 0);

        let deleted_ids = writer.deleted_ids.lock().unwrap().clone();
        assert!(deleted_ids.is_empty());
    }

    #[test]
    fn test_process_pending_outbound_operations_deletes_detached_instance_and_clears_metadata() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        let engine = CalendarSyncEngine::new(conn).unwrap();
        let event_service = EventService::new(conn);

        let detached_start = Local.with_ymd_and_hms(2026, 4, 14, 9, 0, 0).unwrap();
        let detached_end = detached_start + Duration::hours(2);
        let detached = event_service
            .create(
                crate::models::event::Event::builder()
                    .title("Detached Delete")
                    .start(detached_start)
                    .end(detached_end)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let detached_id = detached.id.unwrap();
        let detached_uid = format!(
            "uid-parent::RID::{}",
            detached_start.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")
        );

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id) VALUES (?1, ?2, ?3)",
            params![source_id, detached_uid, detached_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_remote_metadata (source_id, external_uid, remote_event_id, remote_etag, remote_payload_hash, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![source_id, detached_uid, "remote-instance-delete-1", "\"etag-delete\"", "hash-delete", "2026-03-06T00:00:00Z"],
        )
        .unwrap();

        event_service.delete_local(detached_id).unwrap();

        let operation_type: String = conn
            .query_row(
                "SELECT operation_type FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(operation_type, OUTBOUND_OPERATION_DELETE);

        let writer = FakeGoogleOutboundWriter::default();
        engine
            .process_pending_outbound_operations(&source, &writer)
            .unwrap();

        let deleted_ids = writer.deleted_ids.lock().unwrap().clone();
        assert_eq!(deleted_ids, vec!["remote-instance-delete-1".to_string()]);

        let remaining_mapping: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_sync_map WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining_mapping, 0);

        let remaining_metadata: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_remote_metadata WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining_metadata, 0);
    }

    #[test]
    fn test_sync_source_from_google_payload_round_trips_detached_instance_after_outbound_update() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        let engine = CalendarSyncEngine::new(conn).unwrap();
        let event_service = EventService::new(conn);

        let detached_start = Local.with_ymd_and_hms(2026, 4, 21, 11, 0, 0).unwrap();
        let detached_end = detached_start + Duration::hours(2);
        let detached = event_service
            .create(
                crate::models::event::Event::builder()
                    .title("Detached Original")
                    .start(detached_start)
                    .end(detached_end)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let detached_id = detached.id.unwrap();
        let detached_uid = format!(
            "uid-parent::RID::{}",
            detached_start.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")
        );

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id) VALUES (?1, ?2, ?3)",
            params![source_id, detached_uid, detached_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_remote_metadata (source_id, external_uid, remote_event_id, remote_etag, remote_payload_hash, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![source_id, detached_uid, "remote-instance-roundtrip-1", "\"etag-roundtrip\"", "hash-roundtrip", "2026-03-06T00:00:00Z"],
        )
        .unwrap();

        let mut updated_detached = detached.clone();
        updated_detached.title = "Detached Roundtrip".to_string();
        event_service.update_local(&updated_detached).unwrap();

        let writer = FakeGoogleOutboundWriter::default();
        engine
            .process_pending_outbound_operations(&source, &writer)
            .unwrap();

        let inbound_payload = super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
            &format!(
                r#"{{
                "items": [
                    {{
                        "id": "remote-instance-roundtrip-1",
                        "etag": "\"etag-roundtrip-2\"",
                        "status": "confirmed",
                        "summary": "Detached Roundtrip",
                        "iCalUID": "uid-parent",
                        "updated": "2026-03-06T03:00:00Z",
                        "originalStartTime": {{ "dateTime": "{}" }},
                        "start": {{ "dateTime": "{}" }},
                        "end": {{ "dateTime": "{}" }}
                    }}
                ],
                "nextSyncToken": "sync-token-roundtrip"
            }}"#,
                detached_start.with_timezone(&Utc).to_rfc3339(),
                detached_start.with_timezone(&Utc).to_rfc3339(),
                detached_end.with_timezone(&Utc).to_rfc3339(),
            ),
        )
        .unwrap();

        let result = engine
            .sync_source_from_google_payload(source_id, inbound_payload)
            .unwrap();

        assert_eq!(result.unchanged, 1);
        assert_eq!(result.conflicts, 0);

        let refreshed = event_service.get(detached_id).unwrap().unwrap();
        assert_eq!(refreshed.title, "Detached Roundtrip");
    }

    #[test]
    fn test_sync_source_from_google_payload_round_trips_detached_instance_after_outbound_create() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let source = crate::services::calendar_sync::CalendarSourceService::new(conn)
            .get_by_id(source_id)
            .unwrap()
            .unwrap();
        let engine = CalendarSyncEngine::new(conn).unwrap();
        let event_service = EventService::new(conn);

        let series_start = Local.with_ymd_and_hms(2026, 3, 10, 9, 0, 0).unwrap();
        let series_end = series_start + Duration::hours(1);
        let mut series = crate::models::event::Event::new("Series", series_start, series_end).unwrap();
        series.recurrence_rule = Some("FREQ=WEEKLY;BYDAY=TU".to_string());
        let series = event_service.create(series).unwrap();
        let series_id = series.id.unwrap();

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id) VALUES (?1, ?2, ?3)",
            params![source_id, "uid-parent", series_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_remote_metadata (source_id, external_uid, remote_event_id, remote_etag, remote_payload_hash, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![source_id, "uid-parent", "remote-parent-1", "\"etag-parent\"", "hash-parent", "2026-03-06T00:00:00Z"],
        )
        .unwrap();

        let detached_start = Local.with_ymd_and_hms(2026, 5, 5, 9, 0, 0).unwrap();
        let detached_end = detached_start + Duration::hours(2);
        let detached = event_service
            .create(
                crate::models::event::Event::builder()
                    .title("Detached Created")
                    .start(detached_start)
                    .end(detached_end)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let detached_id = detached.id.unwrap();
        let detached_uid = format!(
            "uid-parent::RID::{}",
            detached_start.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")
        );

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id) VALUES (?1, ?2, ?3)",
            params![source_id, detached_uid, detached_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO outbound_sync_operations (source_id, local_event_id, external_uid, operation_type, payload_json, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
            params![
                source_id,
                detached_id,
                detached_uid,
                OUTBOUND_OPERATION_CREATE,
                serde_json::json!({
                    "title": "Detached Created",
                    "start": detached_start.to_rfc3339(),
                    "end": detached_end.to_rfc3339(),
                    "all_day": false,
                    "recurrence_rule": null,
                    "recurrence_exceptions": null
                })
                .to_string(),
            ],
        )
        .unwrap();

        let writer = FakeGoogleOutboundWriter::default();
        engine
            .process_pending_outbound_operations(&source, &writer)
            .unwrap();

        let payload = super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
            &format!(
                r#"{{
                "items": [
                    {{
                        "id": "remote-instance-1",
                        "etag": "\"etag-instance-2\"",
                        "status": "confirmed",
                        "summary": "Detached Created",
                        "iCalUID": "uid-parent",
                        "updated": "2026-03-06T05:00:00Z",
                        "originalStartTime": {{ "dateTime": "{}" }},
                        "start": {{ "dateTime": "{}" }},
                        "end": {{ "dateTime": "{}" }}
                    }}
                ],
                "nextSyncToken": "sync-token-create-roundtrip"
            }}"#,
                detached_start.with_timezone(&Utc).to_rfc3339(),
                detached_start.with_timezone(&Utc).to_rfc3339(),
                detached_end.with_timezone(&Utc).to_rfc3339(),
            ),
        )
        .unwrap();

        let result = engine
            .sync_source_from_google_payload(source_id, payload)
            .unwrap();

        assert_eq!(result.unchanged, 1);
        assert_eq!(result.conflicts, 0);

        let refreshed = event_service.get(detached_id).unwrap().unwrap();
        assert_eq!(refreshed.title, "Detached Created");

        let mapping_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_sync_map WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(mapping_count, 1);
    }

    #[test]
    fn test_sync_source_from_google_payload_deletes_cancelled_detached_instance() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let engine = CalendarSyncEngine::new(conn).unwrap();
        let event_service = EventService::new(conn);

        let detached_start = Local.with_ymd_and_hms(2026, 5, 12, 9, 0, 0).unwrap();
        let detached_end = detached_start + Duration::hours(2);
        let detached = event_service
            .create(
                crate::models::event::Event::builder()
                    .title("Detached Cancel")
                    .start(detached_start)
                    .end(detached_end)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let detached_id = detached.id.unwrap();
        let detached_uid = format!(
            "uid-parent::RID::{}",
            detached_start.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")
        );

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id) VALUES (?1, ?2, ?3)",
            params![source_id, detached_uid, detached_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_remote_metadata (source_id, external_uid, remote_event_id, remote_etag, remote_payload_hash, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![source_id, detached_uid, "remote-instance-cancel-1", "\"etag-cancel\"", "hash-cancel", "2026-03-06T00:00:00Z"],
        )
        .unwrap();

        let payload = super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
            &format!(
                r#"{{
                "items": [
                    {{
                        "id": "remote-instance-cancel-1",
                        "status": "cancelled",
                        "iCalUID": "uid-parent",
                        "originalStartTime": {{ "dateTime": "{}" }}
                    }}
                ],
                "nextSyncToken": "sync-token-detached-cancel"
            }}"#,
                detached_start.with_timezone(&Utc).to_rfc3339(),
            ),
        )
        .unwrap();

        let result = engine
            .sync_source_from_google_payload(source_id, payload)
            .unwrap();

        assert_eq!(result.deleted, 1);
        assert!(event_service.get(detached_id).unwrap().is_none());

        let mapping_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_sync_map WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(mapping_count, 0);

        let metadata_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_remote_metadata WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(metadata_count, 0);
    }

    #[test]
    fn test_sync_source_from_google_payload_completes_matching_detached_delete_without_conflict() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn, "API Source");
        let engine = CalendarSyncEngine::new(conn).unwrap();
        let event_service = EventService::new(conn);

        let detached_start = Local.with_ymd_and_hms(2026, 5, 19, 9, 0, 0).unwrap();
        let detached_end = detached_start + Duration::hours(2);
        let detached = event_service
            .create(
                crate::models::event::Event::builder()
                    .title("Detached Delete Converges")
                    .start(detached_start)
                    .end(detached_end)
                    .build()
                    .unwrap(),
            )
            .unwrap();
        let detached_id = detached.id.unwrap();
        let detached_uid = format!(
            "uid-parent::RID::{}",
            detached_start.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")
        );

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id) VALUES (?1, ?2, ?3)",
            params![source_id, detached_uid, detached_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_remote_metadata (source_id, external_uid, remote_event_id, remote_etag, remote_payload_hash, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![source_id, detached_uid, "remote-instance-delete-race-1", "\"etag-delete-race\"", "hash-delete-race", "2026-03-06T00:00:00Z"],
        )
        .unwrap();

        event_service.delete_local(detached_id).unwrap();

        let payload = super::super::google_api::GoogleCalendarApiClient::parse_events_response_body(
            &format!(
                r#"{{
                "items": [
                    {{
                        "id": "remote-instance-delete-race-1",
                        "status": "cancelled",
                        "iCalUID": "uid-parent",
                        "originalStartTime": {{ "dateTime": "{}" }}
                    }}
                ],
                "nextSyncToken": "sync-token-detached-delete-race"
            }}"#,
                detached_start.with_timezone(&Utc).to_rfc3339(),
            ),
        )
        .unwrap();

        let result = engine
            .sync_source_from_google_payload(source_id, payload)
            .unwrap();

        assert_eq!(result.deleted, 1);
        assert_eq!(result.conflicts, 0);

        let queue_status: String = conn
            .query_row(
                "SELECT status FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(queue_status, crate::models::outbound_sync_operation::OUTBOUND_STATUS_COMPLETED);

        let mapping_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_sync_map WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(mapping_count, 0);
    }
}
