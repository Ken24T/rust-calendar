use super::shared::{deserialize_exceptions, serialize_exceptions, to_local_datetime};
use super::EventService;
use crate::models::event::Event;
use crate::models::outbound_sync_operation::{
    OUTBOUND_OPERATION_CREATE, OUTBOUND_OPERATION_UPDATE,
};
use crate::services::outbound_sync::OutboundSyncService;
use anyhow::{anyhow, Context, Result};
use chrono::{Local, TimeZone};
use rusqlite::{self, params};
use serde_json::json;

impl<'a> EventService<'a> {
    /// Create a user-initiated local event and enqueue outbound sync when mapped to a writable source.
    pub fn create_local(&self, event: Event) -> Result<Event> {
        let created = self.create(event)?;
        if let Some(event_id) = created.id {
            let payload = json!({
                "event_id": event_id,
                "title": created.title,
                "start": created.start.to_rfc3339(),
                "end": created.end.to_rfc3339(),
                "all_day": created.all_day,
                "updated_at": Local::now().to_rfc3339(),
            })
            .to_string();
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
            let payload = json!({
                "event_id": event_id,
                "title": event.title,
                "start": event.start.to_rfc3339(),
                "end": event.end.to_rfc3339(),
                "all_day": event.all_day,
                "updated_at": Local::now().to_rfc3339(),
            })
            .to_string();
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
    pub fn delete_occurrence(
        &self,
        id: i64,
        occurrence_date: chrono::DateTime<Local>,
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
        let mut exceptions = event.recurrence_exceptions.unwrap_or_default();

        // Normalize to midnight for all-day events, or keep exact time
        let exception_date = if event.all_day {
            occurrence_date
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .and_then(|dt| Local.from_local_datetime(&dt).single())
                .unwrap_or(occurrence_date)
        } else {
            occurrence_date
        };

        // Add if not already present
        if !exceptions.contains(&exception_date) {
            exceptions.push(exception_date);
            event.recurrence_exceptions = Some(exceptions);
            self.update(&event)?;
        }

        Ok(())
    }
}
