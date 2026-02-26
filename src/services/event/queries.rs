use super::shared::{deserialize_exceptions, to_local_datetime};
use super::EventService;
use crate::models::event::Event;
use anyhow::Result;
use chrono::{DateTime, Local};
use rusqlite::{self, Row};

impl<'a> EventService<'a> {
    /// List every event ordered by start date.
    #[allow(dead_code)]
    pub fn list_all(&self) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, location, start_datetime, end_datetime,
                    is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                    created_at, updated_at
             FROM events
             ORDER BY start_datetime ASC",
        )?;

        let events = stmt
            .query_map([], map_event_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(events)
    }

    /// Search events by title, description, location, or category.
    pub fn search(&self, query: &str) -> Result<Vec<Event>> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }
        
        let search_pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, location, start_datetime, end_datetime,
                    is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                    created_at, updated_at
             FROM events
             WHERE LOWER(title) LIKE ?1
                OR LOWER(COALESCE(description, '')) LIKE ?1
                OR LOWER(COALESCE(location, '')) LIKE ?1
                OR LOWER(COALESCE(category, '')) LIKE ?1
             ORDER BY start_datetime ASC",
        )?;

        let events = stmt
            .query_map([&search_pattern], map_event_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(events)
    }

    /// Find events, expanding the window slightly when recurrence is present.
    pub fn find_by_date_range(
        &self,
        start: DateTime<Local>,
        end: DateTime<Local>,
    ) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, location, start_datetime, end_datetime,
                    is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                    created_at, updated_at
             FROM events
             WHERE (start_datetime <= ? AND end_datetime >= ?)
                OR (recurrence_rule IS NOT NULL AND recurrence_rule != '' AND recurrence_rule != 'None' AND start_datetime <= ?)
             ORDER BY start_datetime ASC",
        )?;

        let events = stmt
            .query_map(
                [end.to_rfc3339(), start.to_rfc3339(), end.to_rfc3339()],
                map_event_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(events)
    }
}

fn map_event_row(row: &Row<'_>) -> Result<Event, rusqlite::Error> {
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
}
