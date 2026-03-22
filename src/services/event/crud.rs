use super::shared::{deserialize_exceptions, serialize_exceptions, to_local_datetime};
use super::EventService;
use crate::models::event::Event;
use crate::models::event_sync_map::EventSyncMap;
use crate::models::outbound_sync_operation::{
    OUTBOUND_OPERATION_CREATE, OUTBOUND_OPERATION_UPDATE,
};
use crate::services::calendar_sync::mapping::EventSyncMapService;
use crate::services::outbound_sync::OutboundSyncService;
use anyhow::{anyhow, Context, Result};
use chrono::{Local, TimeZone, Utc};
use rusqlite::{self, params};
use serde_json::json;

impl<'a> EventService<'a> {
    /// Create a user-initiated local event and enqueue outbound sync when mapped to a writable source.
    pub fn create_local(&self, event: Event) -> Result<Event> {
        let created = self.create(event)?;
        if let Some(event_id) = created.id {
            let payload = Self::build_outbound_payload(&created, event_id).to_string();
            let outbound = OutboundSyncService::new(self.conn);
            let _ = outbound.enqueue_upsert_for_local_event(
                event_id,
                OUTBOUND_OPERATION_CREATE,
                Some(&payload),
            )?;
        }

        Ok(created)
    }

    /// Update a user-initiated local event and enqueue outbound sync when mapped to a writable source.
    pub fn update_local(&self, event: &Event) -> Result<()> {
        self.update(event)?;

        if let Some(event_id) = event.id {
            let payload = Self::build_outbound_payload(event, event_id).to_string();
            let outbound = OutboundSyncService::new(self.conn);
            let _ = outbound.enqueue_upsert_for_local_event(
                event_id,
                OUTBOUND_OPERATION_UPDATE,
                Some(&payload),
            )?;
        }

        Ok(())
    }

    /// Delete a user-initiated local event and enqueue outbound deletion for writable mapped sources.
    pub fn delete_local(&self, id: i64) -> Result<()> {
        let outbound = OutboundSyncService::new(self.conn);
        let mapping = outbound.writable_identity_for_local_event(id)?;

        self.delete(id)?;

        if let Some((source_id, external_uid)) = mapping {
            let payload = json!({
                "event_id": id,
                "external_uid": external_uid,
                "deleted_at": Local::now().to_rfc3339(),
            })
            .to_string();
            outbound.enqueue_delete_for_identity(source_id, &external_uid, Some(&payload))?;
        }

        Ok(())
    }

    /// Create a new event in the database.
    pub fn create(&self, mut event: Event) -> Result<Event> {
        event.validate().map_err(|e| anyhow!(e))?;

        let now = Local::now().to_rfc3339();
        let exceptions_json = serialize_exceptions(event.recurrence_exceptions.as_ref());

        self.conn
            .execute(
                "INSERT INTO events (
                    title, description, location, start_datetime, end_datetime,
                    is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    event.title,
                    event.description,
                    event.location,
                    event.start.to_rfc3339(),
                    event.end.to_rfc3339(),
                    event.all_day as i32,
                    event.category,
                    event.color,
                    event.recurrence_rule,
                    exceptions_json,
                    &now,
                    &now,
                ],
            )
            .context("Failed to insert event")?;

        let id = self.conn.last_insert_rowid();
        event.id = Some(id);
        event.created_at = Some(Local::now());
        event.updated_at = Some(Local::now());

        Ok(event)
    }

    /// Retrieve an event by ID.
    pub fn get(&self, id: i64) -> Result<Option<Event>> {
        let result = self.conn.query_row(
            "SELECT id, title, description, location, start_datetime, end_datetime,
                    is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                    created_at, updated_at
             FROM events WHERE id = ?",
            [id],
            |row| {
                let recurrence_exceptions = deserialize_exceptions(row.get(10)?)?;

                Ok(Event {
                    id: Some(row.get(0)?),
                    title: row.get(1)?,
                    description: row.get(2)?,
                    location: row.get(3)?,
                    start: to_local_datetime(row.get::<_, String>(4)?)?,
                    end: to_local_datetime(row.get::<_, String>(5)?)?,
                    all_day: row.get::<_, i32>(6)? != 0,
                    category: row.get(7)?,
                    color: row.get(8)?,
                    recurrence_rule: row.get(9)?,
                    recurrence_exceptions,
                    created_at: Some(to_local_datetime(row.get::<_, String>(11)?)?),
                    updated_at: Some(to_local_datetime(row.get::<_, String>(12)?)?),
                })
            },
        );

        match result {
            Ok(event) => Ok(Some(event)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update an existing event.
    pub fn update(&self, event: &Event) -> Result<()> {
        let id = event
            .id
            .ok_or_else(|| anyhow!("Event ID is required for update"))?;
        event.validate().map_err(|e| anyhow!(e))?;

        let exceptions_json = serialize_exceptions(event.recurrence_exceptions.as_ref());
        let rows_affected = self
            .conn
            .execute(
                "UPDATE events SET
                    title = ?, description = ?, location = ?, start_datetime = ?, end_datetime = ?,
                    is_all_day = ?, category = ?, color = ?, recurrence_rule = ?,
                    recurrence_exceptions = ?, updated_at = ?
                 WHERE id = ?",
                params![
                    event.title,
                    event.description,
                    event.location,
                    event.start.to_rfc3339(),
                    event.end.to_rfc3339(),
                    event.all_day as i32,
                    event.category,
                    event.color,
                    event.recurrence_rule,
                    exceptions_json,
                    Local::now().to_rfc3339(),
                    id,
                ],
            )
            .context("Failed to update event")?;

        if rows_affected == 0 {
            return Err(anyhow!("Event with id {} not found", id));
        }

        Ok(())
    }

    /// Delete an event by ID.
    pub fn delete(&self, id: i64) -> Result<()> {
        let rows_affected = self
            .conn
            .execute("DELETE FROM events WHERE id = ?", [id])
            .context("Failed to delete event")?;

        if rows_affected == 0 {
            return Err(anyhow!("Event with id {} not found", id));
        }

        Ok(())
    }

    /// Add an exception date to a recurring event (deletes single occurrence).
    #[allow(dead_code)]
    pub fn delete_occurrence(
        &self,
        id: i64,
        occurrence_date: chrono::DateTime<Local>,
    ) -> Result<()> {
        self.delete_occurrence_inner(id, occurrence_date, false)
    }

    /// Add an exception date to a recurring event and enqueue an outbound update when writable.
    pub fn delete_occurrence_local(
        &self,
        id: i64,
        occurrence_date: chrono::DateTime<Local>,
    ) -> Result<()> {
        self.delete_occurrence_inner(id, occurrence_date, true)
    }

    pub fn detach_occurrence_local(
        &self,
        id: i64,
        occurrence_date: chrono::DateTime<Local>,
        detached_event: Event,
    ) -> Result<Event> {
        let parent_event = self
            .get(id)?
            .ok_or_else(|| anyhow!("Event with id {} not found", id))?;

        if parent_event.recurrence_rule.is_none() {
            return Err(anyhow!(
                "Event is not recurring, edit the event directly instead"
            ));
        }

        let detached_occurrence_date =
            Self::normalize_occurrence_date(&parent_event, occurrence_date);
        let outbound = OutboundSyncService::new(self.conn);
        let writable_identity = outbound.writable_identity_for_local_event(id)?;
        let detached_event = Self::prepare_detached_occurrence_event(detached_event);
        let created = self.create(detached_event)?;
        let created_id = created
            .id
            .ok_or_else(|| anyhow!("Detached occurrence did not receive an event id"))?;

        if let Some((source_id, parent_external_uid)) = writable_identity.as_ref() {
            let detached_external_uid = Self::detached_occurrence_external_uid(
                parent_external_uid,
                &parent_event,
                detached_occurrence_date,
            );

            EventSyncMapService::new(self.conn).create(EventSyncMap {
                id: None,
                source_id: *source_id,
                external_uid: detached_external_uid.clone(),
                local_event_id: created_id,
                external_last_modified: None,
                external_etag_hash: None,
                last_seen_at: None,
                first_missing_at: None,
                purge_after_at: None,
            })?;

            let payload = Self::build_outbound_payload(&created, created_id).to_string();
            outbound.enqueue_upsert_for_identity(
                *source_id,
                created_id,
                &detached_external_uid,
                OUTBOUND_OPERATION_CREATE,
                Some(&payload),
            )?;
        }

        self.delete_occurrence_inner(id, detached_occurrence_date, writable_identity.is_some())?;

        Ok(created)
    }

    fn delete_occurrence_inner(
        &self,
        id: i64,
        occurrence_date: chrono::DateTime<Local>,
        queue_outbound: bool,
    ) -> Result<()> {
        // Get the event
        let mut event = self
            .get(id)?
            .ok_or_else(|| anyhow!("Event with id {} not found", id))?;

        // Ensure it's a recurring event
        if event.recurrence_rule.is_none() {
            return Err(anyhow!("Event is not recurring, use delete() instead"));
        }

        // Add the occurrence date to exceptions
        let mut exceptions = event.recurrence_exceptions.clone().unwrap_or_default();

        // Normalize to midnight for all-day events, or keep exact time
        let exception_date = Self::normalize_occurrence_date(&event, occurrence_date);

        // Add if not already present
        if !exceptions.contains(&exception_date) {
            exceptions.push(exception_date);
            event.recurrence_exceptions = Some(exceptions);
            if queue_outbound {
                self.update_local(&event)?;
            } else {
                self.update(&event)?;
            }
        }

        Ok(())
    }

    fn build_outbound_payload(event: &Event, event_id: i64) -> serde_json::Value {
        let recurrence_exceptions = event.recurrence_exceptions.as_ref().map(|dates| {
            dates
                .iter()
                .map(|dt| dt.to_rfc3339())
                .collect::<Vec<String>>()
        });

        json!({
            "event_id": event_id,
            "title": event.title,
            "description": event.description,
            "location": event.location,
            "start": event.start.to_rfc3339(),
            "end": event.end.to_rfc3339(),
            "all_day": event.all_day,
            "category": event.category,
            "color": event.color,
            "recurrence_rule": event.recurrence_rule,
            "recurrence_exceptions": recurrence_exceptions,
            "updated_at": Local::now().to_rfc3339(),
        })
    }

    fn prepare_detached_occurrence_event(mut detached_event: Event) -> Event {
        detached_event.id = None;
        detached_event.recurrence_rule = None;
        detached_event.recurrence_exceptions = None;
        detached_event.created_at = None;
        detached_event.updated_at = None;
        detached_event
    }

    fn normalize_occurrence_date(
        event: &Event,
        occurrence_date: chrono::DateTime<Local>,
    ) -> chrono::DateTime<Local> {
        if event.all_day {
            occurrence_date
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .and_then(|dt| Local.from_local_datetime(&dt).single())
                .unwrap_or(occurrence_date)
        } else {
            occurrence_date
        }
    }

    fn detached_occurrence_external_uid(
        parent_external_uid: &str,
        parent_event: &Event,
        occurrence_date: chrono::DateTime<Local>,
    ) -> String {
        let recurrence_token = if parent_event.all_day {
            occurrence_date.format("%Y%m%d").to_string()
        } else {
            occurrence_date
                .with_timezone(&Utc)
                .format("%Y%m%dT%H%M%SZ")
                .to_string()
        };

        format!("{}::RID::{}", parent_external_uid, recurrence_token)
    }
}

#[cfg(test)]
mod tests {
    use super::EventService;
    use crate::models::calendar_source::SYNC_CAPABILITY_READ_WRITE;
    use crate::models::event::Event;
    use crate::services::database::Database;
    use chrono::{Duration, Local, TimeZone};
    use rusqlite::params;

    fn create_rw_source(conn: &rusqlite::Connection) -> i64 {
        conn.execute(
            "INSERT INTO calendar_sources (name, source_type, ics_url, enabled, poll_interval_minutes, sync_capability)
             VALUES (?1, 'google_ics', ?2, 1, 15, ?3)",
            params![
                "RW Source",
                "https://calendar.google.com/calendar/ical/test%40gmail.com/private-token/basic.ics",
                SYNC_CAPABILITY_READ_WRITE,
            ],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn recurring_event() -> Event {
        let start = Local
            .with_ymd_and_hms(2026, 3, 10, 9, 0, 0)
            .single()
            .unwrap();
        let end = start + Duration::hours(1);
        let mut event = Event::builder()
            .title("Recurring")
            .start(start)
            .end(end)
            .recurrence_rule("FREQ=WEEKLY;BYDAY=TU")
            .build()
            .unwrap();
        event.recurrence_exceptions = Some(vec![start + Duration::weeks(1)]);
        event
    }

    #[test]
    fn test_update_local_payload_includes_recurrence_fields() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn);
        let service = EventService::new(conn);

        let created = service.create(recurring_event()).unwrap();
        let event_id = created.id.unwrap();

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id)
             VALUES (?1, ?2, ?3)",
            params![source_id, "uid-recur-1", event_id],
        )
        .unwrap();

        let mut updated = created.clone();
        updated.recurrence_exceptions = Some(vec![
            Local
                .with_ymd_and_hms(2026, 3, 17, 9, 0, 0)
                .single()
                .unwrap(),
            Local
                .with_ymd_and_hms(2026, 3, 24, 9, 0, 0)
                .single()
                .unwrap(),
        ]);
        service.update_local(&updated).unwrap();

        let payload: String = conn
            .query_row(
                "SELECT payload_json FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, "uid-recur-1"],
                |row| row.get(0),
            )
            .unwrap();

        assert!(payload.contains("FREQ=WEEKLY;BYDAY=TU"));
        assert!(payload.contains("recurrence_exceptions"));
        assert!(payload.contains("2026-03-17T09:00:00"));
        assert!(payload.contains("2026-03-24T09:00:00"));
    }

    #[test]
    fn test_delete_occurrence_local_updates_exceptions_and_queue() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn);
        let service = EventService::new(conn);

        let created = service.create(recurring_event()).unwrap();
        let event_id = created.id.unwrap();

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id)
             VALUES (?1, ?2, ?3)",
            params![source_id, "uid-recur-2", event_id],
        )
        .unwrap();

        let deleted_occurrence = Local
            .with_ymd_and_hms(2026, 3, 31, 9, 0, 0)
            .single()
            .unwrap();
        service
            .delete_occurrence_local(event_id, deleted_occurrence)
            .unwrap();

        let refreshed = service.get(event_id).unwrap().unwrap();
        let exceptions = refreshed.recurrence_exceptions.unwrap();
        assert!(exceptions.contains(&deleted_occurrence));

        let payload: String = conn
            .query_row(
                "SELECT payload_json FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid = ?2",
                params![source_id, "uid-recur-2"],
                |row| row.get(0),
            )
            .unwrap();

        assert!(payload.contains("2026-03-31T09:00:00"));
    }

    #[test]
    fn test_detach_occurrence_local_creates_standalone_event_and_queues_series_and_instance() {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        let conn = db.connection();
        let source_id = create_rw_source(conn);
        let service = EventService::new(conn);

        let created = service.create(recurring_event()).unwrap();
        let event_id = created.id.unwrap();

        conn.execute(
            "INSERT INTO event_sync_map (source_id, external_uid, local_event_id)
             VALUES (?1, ?2, ?3)",
            params![source_id, "uid-recur-3", event_id],
        )
        .unwrap();

        let detached_start = Local
            .with_ymd_and_hms(2026, 4, 7, 9, 0, 0)
            .single()
            .unwrap();
        let detached_end = detached_start + Duration::hours(2);
        let detached_event = Event::builder()
            .title("Detached occurrence")
            .start(detached_start)
            .end(detached_end)
            .description("Edited just once")
            .build()
            .unwrap();

        let saved = service
            .detach_occurrence_local(event_id, detached_start, detached_event)
            .unwrap();
        let detached_id = saved.id.unwrap();

        let parent = service.get(event_id).unwrap().unwrap();
        assert!(parent
            .recurrence_exceptions
            .unwrap_or_default()
            .contains(&detached_start));

        let detached = service.get(detached_id).unwrap().unwrap();
        assert_eq!(detached.title, "Detached occurrence");
        assert!(detached.recurrence_rule.is_none());

        let detached_uid: String = conn
            .query_row(
                "SELECT external_uid FROM event_sync_map WHERE local_event_id = ?1",
                params![detached_id],
                |row| row.get(0),
            )
            .unwrap();
        let expected_uid = format!(
            "uid-recur-3::RID::{}",
            detached_start
                .with_timezone(&chrono::Utc)
                .format("%Y%m%dT%H%M%SZ")
        );
        assert_eq!(detached_uid, expected_uid);

        let queued_ops: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM outbound_sync_operations WHERE source_id = ?1 AND external_uid IN (?2, ?3)",
                params![source_id, "uid-recur-3", detached_uid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(queued_ops, 2);
    }
}
