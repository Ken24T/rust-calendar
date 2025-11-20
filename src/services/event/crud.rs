use super::shared::{deserialize_exceptions, serialize_exceptions, to_local_datetime};
use super::EventService;
use crate::models::event::Event;
use anyhow::{anyhow, Context, Result};
use chrono::{Local, TimeZone};
use rusqlite::{self, params};

impl<'a> EventService<'a> {
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
                .map(|dt| Local.from_local_datetime(&dt).single())
                .flatten()
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
