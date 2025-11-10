// Event service module
// CRUD operations for calendar events with database integration

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Local};
use rusqlite::Connection;
use crate::models::event::Event;

/// Service for managing calendar events
pub struct EventService<'a> {
    conn: &'a Connection,
}

impl<'a> EventService<'a> {
    /// Create a new EventService with a database connection
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    
    /// Create a new event in the database
    /// 
    /// # Arguments
    /// * `event` - The event to create (id will be ignored and auto-generated)
    /// 
    /// # Returns
    /// Returns the created event with its database-assigned ID
    pub fn create(&self, mut event: Event) -> Result<Event> {
        // Validate before inserting
        event.validate().map_err(|e| anyhow::anyhow!(e))?;
        
        let now = Local::now().to_rfc3339();
        
        // Serialize recurrence exceptions if present
        let exceptions_json = event.recurrence_exceptions.as_ref()
            .map(|excs| {
                let dates: Vec<String> = excs.iter()
                    .map(|dt| dt.to_rfc3339())
                    .collect();
                serde_json::to_string(&dates).unwrap_or_default()
            });
        
        self.conn.execute(
            "INSERT INTO events (
                title, description, location, start_datetime, end_datetime,
                is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
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
        ).context("Failed to insert event")?;
        
        let id = self.conn.last_insert_rowid();
        event.id = Some(id);
        event.created_at = Some(Local::now());
        event.updated_at = Some(Local::now());
        
        Ok(event)
    }
    
    /// Retrieve an event by ID
    pub fn get(&self, id: i64) -> Result<Option<Event>> {
        let result = self.conn.query_row(
            "SELECT id, title, description, location, start_datetime, end_datetime,
                    is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                    created_at, updated_at
             FROM events WHERE id = ?",
            [id],
            |row| {
                let exceptions_json: Option<String> = row.get(10)?;
                let recurrence_exceptions = exceptions_json.and_then(|json| {
                    serde_json::from_str::<Vec<String>>(&json).ok()
                        .map(|dates| dates.into_iter()
                            .filter_map(|s| DateTime::parse_from_rfc3339(&s).ok()
                                .map(|dt| dt.with_timezone(&Local)))
                            .collect())
                });
                
                Ok(Event {
                    id: Some(row.get(0)?),
                    title: row.get(1)?,
                    description: row.get(2)?,
                    location: row.get(3)?,
                    start: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Local),
                    end: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Local),
                    all_day: row.get::<_, i32>(6)? != 0,
                    category: row.get(7)?,
                    color: row.get(8)?,
                    recurrence_rule: row.get(9)?,
                    recurrence_exceptions,
                    created_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Local)),
                    updated_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Local)),
                })
            },
        );
        
        match result {
            Ok(event) => Ok(Some(event)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    /// Update an existing event
    pub fn update(&self, event: &Event) -> Result<()> {
        let id = event.id.ok_or_else(|| anyhow::anyhow!("Event ID is required for update"))?;
        
        // Validate before updating
        event.validate().map_err(|e| anyhow::anyhow!(e))?;
        
        // Serialize recurrence exceptions if present
        let exceptions_json = event.recurrence_exceptions.as_ref()
            .map(|excs| {
                let dates: Vec<String> = excs.iter()
                    .map(|dt| dt.to_rfc3339())
                    .collect();
                serde_json::to_string(&dates).unwrap_or_default()
            });
        
        let rows_affected = self.conn.execute(
            "UPDATE events SET
                title = ?, description = ?, location = ?, start_datetime = ?, end_datetime = ?,
                is_all_day = ?, category = ?, color = ?, recurrence_rule = ?,
                recurrence_exceptions = ?, updated_at = ?
             WHERE id = ?",
            rusqlite::params![
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
        ).context("Failed to update event")?;
        
        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Event with id {} not found", id));
        }
        
        Ok(())
    }
    
    /// Delete an event by ID
    pub fn delete(&self, id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM events WHERE id = ?",
            [id],
        ).context("Failed to delete event")?;
        
        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Event with id {} not found", id));
        }
        
        Ok(())
    }
    
    /// List all events
    pub fn list_all(&self) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, location, start_datetime, end_datetime,
                    is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                    created_at, updated_at
             FROM events
             ORDER BY start_datetime ASC"
        )?;
        
        let events = stmt.query_map([], |row| {
            let exceptions_json: Option<String> = row.get(10)?;
            let recurrence_exceptions = exceptions_json.and_then(|json| {
                serde_json::from_str::<Vec<String>>(&json).ok()
                    .map(|dates| dates.into_iter()
                        .filter_map(|s| DateTime::parse_from_rfc3339(&s).ok()
                            .map(|dt| dt.with_timezone(&Local)))
                        .collect())
            });
            
            Ok(Event {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                description: row.get(2)?,
                location: row.get(3)?,
                start: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local),
                end: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local),
                all_day: row.get::<_, i32>(6)? != 0,
                category: row.get(7)?,
                color: row.get(8)?,
                recurrence_rule: row.get(9)?,
                recurrence_exceptions,
                created_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local)),
                updated_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local)),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(events)
    }
    
    /// Find events within a date range
    pub fn find_by_date_range(&self, start: DateTime<Local>, end: DateTime<Local>) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, location, start_datetime, end_datetime,
                    is_all_day, category, color, recurrence_rule, recurrence_exceptions,
                    created_at, updated_at
             FROM events
             WHERE (start_datetime <= ? AND end_datetime >= ?)
                OR (recurrence_rule IS NOT NULL AND recurrence_rule != '' AND recurrence_rule != 'None' AND start_datetime <= ?)
             ORDER BY start_datetime ASC"
        )?;
        
        let events = stmt.query_map([end.to_rfc3339(), start.to_rfc3339(), end.to_rfc3339()], |row| {
            let exceptions_json: Option<String> = row.get(10)?;
            let recurrence_exceptions = exceptions_json.and_then(|json| {
                serde_json::from_str::<Vec<String>>(&json).ok()
                    .map(|dates| dates.into_iter()
                        .filter_map(|s| DateTime::parse_from_rfc3339(&s).ok()
                            .map(|dt| dt.with_timezone(&Local)))
                        .collect())
            });
            
            Ok(Event {
                id: Some(row.get(0)?),
                title: row.get(1)?,
                description: row.get(2)?,
                location: row.get(3)?,
                start: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local),
                end: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local),
                all_day: row.get::<_, i32>(6)? != 0,
                category: row.get(7)?,
                color: row.get(8)?,
                recurrence_rule: row.get(9)?,
                recurrence_exceptions,
                created_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local)),
                updated_at: Some(DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Local)),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(events)
    }
    
    /// Expand recurring events into individual occurrences within the date range
    /// Non-recurring events are returned as-is
    pub fn expand_recurring_events(&self, start: DateTime<Local>, end: DateTime<Local>) -> Result<Vec<Event>> {
        let base_events = self.find_by_date_range(start, end)?;
        let mut expanded_events = Vec::new();
        
        for event in base_events {
            if let Some(ref rrule) = event.recurrence_rule {
                if rrule != "None" && !rrule.is_empty() {
                    // Parse the RRULE and generate occurrences
                    let occurrences = self.generate_occurrences(&event, start, end)?;
                    expanded_events.extend(occurrences);
                } else {
                    // Non-recurring event
                    expanded_events.push(event);
                }
            } else {
                // No recurrence rule
                expanded_events.push(event);
            }
        }
        
        // Sort by start time
        expanded_events.sort_by(|a, b| a.start.cmp(&b.start));
        
        Ok(expanded_events)
    }
    
    /// Generate occurrences of a recurring event within a date range
    fn generate_occurrences(&self, event: &Event, range_start: DateTime<Local>, range_end: DateTime<Local>) -> Result<Vec<Event>> {
        let mut occurrences = Vec::new();
        
        if let Some(ref rrule) = event.recurrence_rule {
            let duration = event.end - event.start;
            
            // Parse COUNT if present
            let max_count = if let Some(count_start) = rrule.find("COUNT=") {
                let count_str = &rrule[count_start + 6..];
                let count_end = count_str.find(';').unwrap_or(count_str.len());
                count_str[..count_end].parse::<usize>().ok()
            } else {
                None
            };
            
            // Parse UNTIL date if present
            let until_date = if let Some(until_start) = rrule.find("UNTIL=") {
                let until_str = &rrule[until_start + 6..];
                let until_end = until_str.find(';').unwrap_or(until_str.len());
                let date_str = &until_str[..until_end];
                // Parse YYYYMMDD format
                if date_str.len() == 8 {
                    if let (Ok(year), Ok(month), Ok(day)) = (
                        date_str[0..4].parse::<i32>(),
                        date_str[4..6].parse::<u32>(),
                        date_str[6..8].parse::<u32>()
                    ) {
                        chrono::NaiveDate::from_ymd_opt(year, month, day)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            
            // Handle FREQ=WEEKLY with BYDAY
            if rrule.contains("FREQ=WEEKLY") {
                let interval = if rrule.contains("INTERVAL=2") { 2 } else { 1 }; // weeks
                
                // Parse BYDAY if present
                let byday_days = if let Some(byday_start) = rrule.find("BYDAY=") {
                    let byday_str = &rrule[byday_start + 6..];
                    let byday_end = byday_str.find(';').unwrap_or(byday_str.len());
                    let days_str = &byday_str[..byday_end];
                    
                    // Parse day codes (SU, MO, TU, WE, TH, FR, SA)
                    days_str.split(',')
                        .filter_map(|day| {
                            use chrono::Weekday;
                            match day.trim() {
                                "SU" => Some(Weekday::Sun),
                                "MO" => Some(Weekday::Mon),
                                "TU" => Some(Weekday::Tue),
                                "WE" => Some(Weekday::Wed),
                                "TH" => Some(Weekday::Thu),
                                "FR" => Some(Weekday::Fri),
                                "SA" => Some(Weekday::Sat),
                                _ => None,
                            }
                        })
                        .collect::<Vec<_>>()
                } else {
                    // No BYDAY specified, use the original event's day of week
                    vec![event.start.weekday()]
                };
                
                if !byday_days.is_empty() {
                    // Start from the beginning of the week containing the event start
                    let mut current_week_start = event.start.date_naive()
                        - chrono::Duration::days(event.start.weekday().num_days_from_monday() as i64);
                    let week_start_time = event.start.time();
                    
                    let mut week_count = 0;
                    
                    // Generate occurrences week by week
                    loop {
                        // Check if we've reached the COUNT limit (counting weeks, not individual days)
                        if let Some(max) = max_count {
                            if week_count >= max {
                                break;
                            }
                        }
                        
                        // Check UNTIL date before processing this week
                        if let Some(until) = until_date {
                            if current_week_start > until {
                                break;
                            }
                        }
                        
                        // Track if we added any valid occurrences this week
                        let mut week_has_valid_occurrence = false;
                        
                        // For each specified day in the week
                        for &target_weekday in &byday_days {
                            let days_offset = target_weekday.num_days_from_monday() as i64;
                            let occurrence_date = current_week_start + chrono::Duration::days(days_offset);
                            
                            // Check UNTIL date for this specific day
                            if let Some(until) = until_date {
                                if occurrence_date > until {
                                    continue;
                                }
                            }
                            
                            // Combine with the original time
                            if let Some(occurrence_datetime) = occurrence_date.and_time(week_start_time).and_local_timezone(Local).single() {
                                let occurrence_end = occurrence_datetime + duration;
                                
                                // Check if this occurrence is valid (not before original event, not an exception)
                                if occurrence_datetime >= event.start {
                                    let is_exception = if let Some(ref exceptions) = event.recurrence_exceptions {
                                        exceptions.iter().any(|ex| {
                                            ex.date_naive() == occurrence_datetime.date_naive()
                                        })
                                    } else {
                                        false
                                    };
                                    
                                    if !is_exception {
                                        week_has_valid_occurrence = true;
                                        
                                        // Only add to results if within the requested range
                                        if occurrence_datetime >= range_start && occurrence_datetime <= range_end {
                                            let mut occurrence = event.clone();
                                            occurrence.start = occurrence_datetime;
                                            occurrence.end = occurrence_end;
                                            occurrences.push(occurrence);
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Count this week if it had at least one valid occurrence
                        if week_has_valid_occurrence {
                            week_count += 1;
                        }
                        
                        // Move to next week (or skip weeks based on interval)
                        current_week_start = current_week_start + chrono::Duration::weeks(interval as i64);
                        
                        // Safety break
                        if current_week_start > range_end.date_naive() + chrono::Duration::days(365) {
                            break;
                        }
                    }
                }
            } else {
                // Handle non-weekly recurrence (DAILY, MONTHLY, YEARLY)
                
                // Check for BYMONTHDAY (first/last day of month)
                let bymonthday = if let Some(bymonthday_start) = rrule.find("BYMONTHDAY=") {
                    let bymonthday_str = &rrule[bymonthday_start + 11..];
                    let bymonthday_end = bymonthday_str.find(';').unwrap_or(bymonthday_str.len());
                    bymonthday_str[..bymonthday_end].parse::<i32>().ok()
                } else {
                    None
                };
                
                // Check for positional BYDAY (e.g., "1MO" = first Monday, "-1FR" = last Friday)
                let positional_byday = if let Some(byday_start) = rrule.find("BYDAY=") {
                    let byday_str = &rrule[byday_start + 6..];
                    let byday_end = byday_str.find(';').unwrap_or(byday_str.len());
                    let day_str = &byday_str[..byday_end];
                    
                    // Check if it's a positional BYDAY (e.g., "1MO" or "-1FR")
                    if day_str.len() > 2 {
                        if day_str.starts_with("1") && day_str.len() == 3 {
                            // First weekday (e.g., "1MO")
                            let weekday_code = &day_str[1..];
                            let weekday = match weekday_code {
                                "SU" => Some(chrono::Weekday::Sun),
                                "MO" => Some(chrono::Weekday::Mon),
                                "TU" => Some(chrono::Weekday::Tue),
                                "WE" => Some(chrono::Weekday::Wed),
                                "TH" => Some(chrono::Weekday::Thu),
                                "FR" => Some(chrono::Weekday::Fri),
                                "SA" => Some(chrono::Weekday::Sat),
                                _ => None,
                            };
                            weekday.map(|wd| (1, wd))
                        } else if day_str.starts_with("-1") && day_str.len() == 4 {
                            // Last weekday (e.g., "-1FR")
                            let weekday_code = &day_str[2..];
                            let weekday = match weekday_code {
                                "SU" => Some(chrono::Weekday::Sun),
                                "MO" => Some(chrono::Weekday::Mon),
                                "TU" => Some(chrono::Weekday::Tue),
                                "WE" => Some(chrono::Weekday::Wed),
                                "TH" => Some(chrono::Weekday::Thu),
                                "FR" => Some(chrono::Weekday::Fri),
                                "SA" => Some(chrono::Weekday::Sat),
                                _ => None,
                            };
                            weekday.map(|wd| (-1, wd))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                if rrule.contains("FREQ=MONTHLY") {
                    // Monthly recurrence
                    let interval = if let Some(interval_start) = rrule.find("INTERVAL=") {
                        let interval_str = &rrule[interval_start + 9..];
                        let interval_end = interval_str.find(';').unwrap_or(interval_str.len());
                        interval_str[..interval_end].parse::<i64>().unwrap_or(1)
                    } else {
                        1
                    };
                    
                    let mut current_date = event.start.date_naive();
                    let event_time = event.start.time();
                    let mut occurrence_count = 0;
                    
                    loop {
                        // Check if we've reached the COUNT limit
                        if let Some(max) = max_count {
                            if occurrence_count >= max {
                                break;
                            }
                        }
                        
                        // Check UNTIL date
                        if let Some(until) = until_date {
                            if current_date > until {
                                break;
                            }
                        }
                        
                        // Calculate the actual occurrence date based on pattern
                        let occurrence_date = if let Some(day) = bymonthday {
                            // First or last day of month
                            if day == 1 {
                                // First day of month
                                chrono::NaiveDate::from_ymd_opt(
                                    current_date.year(),
                                    current_date.month(),
                                    1
                                )
                            } else if day == -1 {
                                // Last day of month
                                let next_month = if current_date.month() == 12 {
                                    chrono::NaiveDate::from_ymd_opt(current_date.year() + 1, 1, 1)
                                } else {
                                    chrono::NaiveDate::from_ymd_opt(
                                        current_date.year(),
                                        current_date.month() + 1,
                                        1
                                    )
                                };
                                next_month.and_then(|d| d.pred_opt())
                            } else {
                                Some(current_date)
                            }
                        } else if let Some((position, weekday)) = positional_byday {
                            // First or last weekday of month
                            if position == 1 {
                                // First occurrence of weekday in month
                                let first_of_month = chrono::NaiveDate::from_ymd_opt(
                                    current_date.year(),
                                    current_date.month(),
                                    1
                                );
                                first_of_month.and_then(|fom| {
                                    let first_weekday = fom.weekday();
                                    let days_until_target = ((weekday.num_days_from_monday() as i32 
                                        - first_weekday.num_days_from_monday() as i32 + 7) % 7) as i64;
                                    Some(fom + chrono::Duration::days(days_until_target))
                                })
                            } else {
                                // Last occurrence of weekday in month
                                let next_month = if current_date.month() == 12 {
                                    chrono::NaiveDate::from_ymd_opt(current_date.year() + 1, 1, 1)
                                } else {
                                    chrono::NaiveDate::from_ymd_opt(
                                        current_date.year(),
                                        current_date.month() + 1,
                                        1
                                    )
                                };
                                next_month.and_then(|nm| nm.pred_opt()).and_then(|last_of_month| {
                                    let last_weekday = last_of_month.weekday();
                                    let days_back_to_target = ((last_weekday.num_days_from_monday() as i32 
                                        - weekday.num_days_from_monday() as i32 + 7) % 7) as i64;
                                    Some(last_of_month - chrono::Duration::days(days_back_to_target))
                                })
                            }
                        } else {
                            // Regular monthly recurrence on same day of month
                            Some(current_date)
                        };
                        
                        if let Some(occ_date) = occurrence_date {
                            // Check UNTIL date again for the calculated date
                            if let Some(until) = until_date {
                                if occ_date > until {
                                    break;
                                }
                            }
                            
                            if let Some(occurrence_datetime) = occ_date.and_time(event_time).and_local_timezone(Local).single() {
                                // Check if this is a valid occurrence (not before original event, not an exception)
                                if occurrence_datetime >= event.start {
                                    let is_exception = if let Some(ref exceptions) = event.recurrence_exceptions {
                                        exceptions.iter().any(|ex| {
                                            ex.date_naive() == occurrence_datetime.date_naive()
                                        })
                                    } else {
                                        false
                                    };
                                    
                                    if !is_exception {
                                        // Count this occurrence
                                        occurrence_count += 1;
                                        
                                        // Only add to results if within the requested range
                                        if occurrence_datetime >= range_start && occurrence_datetime <= range_end {
                                            let occurrence_end = occurrence_datetime + duration;
                                            let mut occurrence = event.clone();
                                            occurrence.start = occurrence_datetime;
                                            occurrence.end = occurrence_end;
                                            occurrences.push(occurrence);
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Move to next month(s)
                        let new_month = current_date.month() as i64 + interval;
                        let years_to_add = (new_month - 1) / 12;
                        let final_month = ((new_month - 1) % 12 + 1) as u32;
                        let final_year = current_date.year() as i64 + years_to_add;
                        
                        current_date = chrono::NaiveDate::from_ymd_opt(
                            final_year as i32,
                            final_month,
                            current_date.day().min(28) // Use day 28 to avoid month overflow issues
                        ).unwrap_or(current_date + chrono::Duration::days(30));
                        
                        // Safety break
                        if current_date > range_end.date_naive() + chrono::Duration::days(365) {
                            break;
                        }
                    }
                } else if rrule.contains("FREQ=YEARLY") {
                    // Yearly recurrence
                    let interval = if let Some(interval_start) = rrule.find("INTERVAL=") {
                        let interval_str = &rrule[interval_start + 9..];
                        let interval_end = interval_str.find(';').unwrap_or(interval_str.len());
                        interval_str[..interval_end].parse::<i32>().unwrap_or(1)
                    } else {
                        1
                    };
                    
                    let mut current_start = event.start;
                    let mut occurrence_count = 0;
                    
                    loop {
                        // Check if we've reached the COUNT limit
                        if let Some(max) = max_count {
                            if occurrence_count >= max {
                                break;
                            }
                        }
                        
                        // Check UNTIL date
                        if let Some(until) = until_date {
                            if current_start.date_naive() > until {
                                break;
                            }
                        }
                        
                        let current_end = current_start + duration;
                        
                        // Check if this is a valid occurrence (not an exception)
                        let is_exception = if let Some(ref exceptions) = event.recurrence_exceptions {
                            exceptions.iter().any(|ex| {
                                ex.date_naive() == current_start.date_naive()
                            })
                        } else {
                            false
                        };
                        
                        if !is_exception {
                            // Count this occurrence
                            occurrence_count += 1;
                            
                            // Only add to results if within the requested range
                            if current_start >= range_start && current_start <= range_end {
                                let mut occurrence = event.clone();
                                occurrence.start = current_start;
                                occurrence.end = current_end;
                                occurrences.push(occurrence);
                            }
                        }
                        
                        // Move to next year(s)
                        let new_year = current_start.year() + interval;
                        current_start = current_start
                            .with_year(new_year)
                            .unwrap_or(current_start + chrono::Duration::days(365 * interval as i64));
                        
                        // Safety break
                        if current_start > range_end + chrono::Duration::days(365) {
                            break;
                        }
                    }
                } else {
                    // Daily recurrence
                    let freq = 1; // days
                    let mut current_start = event.start;
                    let mut occurrence_count = 0;
                    
                    // Generate occurrences
                    while current_start <= event.end.max(range_end) {
                        // Check if we've reached the COUNT limit
                        if let Some(max) = max_count {
                            if occurrence_count >= max {
                                break;
                            }
                        }
                        
                        // Check UNTIL date
                        if let Some(until) = until_date {
                            if current_start.date_naive() > until {
                                break;
                            }
                        }
                        
                        let current_end = current_start + duration;
                        
                        // Check if this is a valid occurrence (not an exception)
                        let is_exception = if let Some(ref exceptions) = event.recurrence_exceptions {
                            exceptions.iter().any(|ex| {
                                ex.date_naive() == current_start.date_naive()
                            })
                        } else {
                            false
                        };
                        
                        if !is_exception {
                            // Count this occurrence towards the total
                            occurrence_count += 1;
                            
                            // Only add to results if within the requested range
                            if current_start >= range_start && current_start <= range_end {
                                let mut occurrence = event.clone();
                                occurrence.start = current_start;
                                occurrence.end = current_end;
                                occurrences.push(occurrence);
                            }
                        }
                        
                        // Move to next occurrence (daily)
                        current_start = current_start + chrono::Duration::days(freq);
                        
                        // Safety break to prevent infinite loops
                        if current_start > range_end + chrono::Duration::days(365) {
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(occurrences)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::Database;
    use chrono::Duration;
    
    fn setup_test_db() -> Database {
        let db = Database::new(":memory:").unwrap();
        db.initialize_schema().unwrap();
        db
    }
    
    fn sample_event() -> Event {
        let start = Local::now();
        let end = start + Duration::hours(1);
        Event::new("Test Event", start, end).unwrap()
    }
    
    #[test]
    fn test_create_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let event = sample_event();
        let result = service.create(event.clone());
        
        assert!(result.is_ok());
        let created = result.unwrap();
        assert!(created.id.is_some());
        assert_eq!(created.title, event.title);
        assert!(created.created_at.is_some());
        assert!(created.updated_at.is_some());
    }
    
    #[test]
    fn test_create_event_with_optional_fields() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let event = Event::builder()
            .title("Conference")
            .description("Annual tech conference")
            .location("Convention Center")
            .start(Local::now())
            .end(Local::now() + Duration::hours(8))
            .category("Work")
            .color("#FF5733")
            .build()
            .unwrap();
        
        let created = service.create(event.clone()).unwrap();
        assert_eq!(created.description, event.description);
        assert_eq!(created.location, event.location);
        assert_eq!(created.category, event.category);
        assert_eq!(created.color, event.color);
    }
    
    #[test]
    fn test_get_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let created = service.create(sample_event()).unwrap();
        let id = created.id.unwrap();
        
        let result = service.get(id);
        assert!(result.is_ok());
        
        let found = result.unwrap();
        assert!(found.is_some());
        
        let event = found.unwrap();
        assert_eq!(event.id, Some(id));
        assert_eq!(event.title, created.title);
    }
    
    #[test]
    fn test_get_nonexistent_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let result = service.get(999);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
    
    #[test]
    fn test_update_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let mut event = service.create(sample_event()).unwrap();
        event.title = "Updated Title".to_string();
        event.description = Some("New description".to_string());
        
        let result = service.update(&event);
        assert!(result.is_ok());
        
        let updated = service.get(event.id.unwrap()).unwrap().unwrap();
        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.description, Some("New description".to_string()));
    }
    
    #[test]
    fn test_update_nonexistent_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let mut event = sample_event();
        event.id = Some(999);
        
        let result = service.update(&event);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_delete_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let created = service.create(sample_event()).unwrap();
        let id = created.id.unwrap();
        
        let result = service.delete(id);
        assert!(result.is_ok());
        
        let found = service.get(id).unwrap();
        assert!(found.is_none());
    }
    
    #[test]
    fn test_delete_nonexistent_event() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let result = service.delete(999);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_list_all_events() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        service.create(sample_event()).unwrap();
        service.create(sample_event()).unwrap();
        service.create(sample_event()).unwrap();
        
        let events = service.list_all().unwrap();
        assert_eq!(events.len(), 3);
    }
    
    #[test]
    fn test_find_by_date_range() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let now = Local::now();
        
        // Event in the past
        let past_event = Event::new(
            "Past Event",
            now - Duration::days(2),
            now - Duration::days(2) + Duration::hours(1),
        ).unwrap();
        service.create(past_event).unwrap();
        
        // Event in range
        let current_event = Event::new(
            "Current Event",
            now,
            now + Duration::hours(1),
        ).unwrap();
        service.create(current_event).unwrap();
        
        // Event in future
        let future_event = Event::new(
            "Future Event",
            now + Duration::days(2),
            now + Duration::days(2) + Duration::hours(1),
        ).unwrap();
        service.create(future_event).unwrap();
        
        let events = service.find_by_date_range(
            now - Duration::hours(1),
            now + Duration::hours(2),
        ).unwrap();
        
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Current Event");
    }
    
    #[test]
    fn test_create_event_with_recurrence() {
        let db = setup_test_db();
        let service = EventService::new(db.connection());
        
        let event = Event::builder()
            .title("Weekly Meeting")
            .start(Local::now())
            .end(Local::now() + Duration::hours(1))
            .recurrence_rule("FREQ=WEEKLY;BYDAY=MO")
            .build()
            .unwrap();
        
        let created = service.create(event).unwrap();
        assert_eq!(created.recurrence_rule, Some("FREQ=WEEKLY;BYDAY=MO".to_string()));
        
        let retrieved = service.get(created.id.unwrap()).unwrap().unwrap();
        assert_eq!(retrieved.recurrence_rule, Some("FREQ=WEEKLY;BYDAY=MO".to_string()));
    }
}
