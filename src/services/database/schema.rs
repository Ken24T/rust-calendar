use anyhow::{Context, Result};
use rusqlite::Connection;

use super::migrations;

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    create_settings_table(conn)?;
    run_settings_migrations(conn)?;
    create_custom_themes_table(conn)?;
    seed_custom_themes(conn)?;
    insert_default_settings(conn)?;
    create_events_table(conn)?;
    super::schema_countdown::create_countdown_tables(conn)?;
    super::schema_countdown::run_countdown_migrations(conn)?;
    create_event_templates_table(conn)?;
    create_categories_table(conn)?;
    create_calendar_sources_table(conn)?;
    create_event_sync_map_table(conn)?;
    initialize_default_categories(conn)?;
    normalize_all_day_event_times(conn)?;
    Ok(())
}

fn create_settings_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            theme TEXT NOT NULL DEFAULT 'light',
            first_day_of_week INTEGER NOT NULL DEFAULT 0,
            time_format TEXT NOT NULL DEFAULT '12h',
            date_format TEXT NOT NULL DEFAULT 'MM/DD/YYYY',
            show_my_day INTEGER NOT NULL DEFAULT 0,
            my_day_position_right INTEGER NOT NULL DEFAULT 0,
            show_ribbon INTEGER NOT NULL DEFAULT 0,
            current_view TEXT NOT NULL DEFAULT 'Month',
            time_slot_interval INTEGER NOT NULL DEFAULT 60,
            default_event_duration INTEGER NOT NULL DEFAULT 60,
            first_day_of_work_week INTEGER NOT NULL DEFAULT 1,
            last_day_of_work_week INTEGER NOT NULL DEFAULT 5,
            default_event_start_time TEXT NOT NULL DEFAULT '08:00',
            default_card_width REAL NOT NULL DEFAULT 120.0,
            default_card_height REAL NOT NULL DEFAULT 110.0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .context("Failed to create settings table")?;

    Ok(())
}

fn run_settings_migrations(conn: &Connection) -> Result<()> {
    migrations::ensure_column(
        conn,
        "settings",
        "time_slot_interval",
        "ALTER TABLE settings ADD COLUMN time_slot_interval INTEGER NOT NULL DEFAULT 60",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "first_day_of_work_week",
        "ALTER TABLE settings ADD COLUMN first_day_of_work_week INTEGER NOT NULL DEFAULT 1",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "last_day_of_work_week",
        "ALTER TABLE settings ADD COLUMN last_day_of_work_week INTEGER NOT NULL DEFAULT 5",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "default_event_start_time",
        "ALTER TABLE settings ADD COLUMN default_event_start_time TEXT NOT NULL DEFAULT '08:00'",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "default_card_width",
        "ALTER TABLE settings ADD COLUMN default_card_width REAL NOT NULL DEFAULT 120.0",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "default_card_height",
        "ALTER TABLE settings ADD COLUMN default_card_height REAL NOT NULL DEFAULT 110.0",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "auto_create_countdown_on_import",
        "ALTER TABLE settings ADD COLUMN auto_create_countdown_on_import INTEGER NOT NULL DEFAULT 0",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "edit_before_import",
        "ALTER TABLE settings ADD COLUMN edit_before_import INTEGER NOT NULL DEFAULT 0",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "show_sidebar",
        "ALTER TABLE settings ADD COLUMN show_sidebar INTEGER NOT NULL DEFAULT 1",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "use_system_theme",
        "ALTER TABLE settings ADD COLUMN use_system_theme INTEGER NOT NULL DEFAULT 0",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "show_week_numbers",
        "ALTER TABLE settings ADD COLUMN show_week_numbers INTEGER NOT NULL DEFAULT 0",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "sidebar_width",
        "ALTER TABLE settings ADD COLUMN sidebar_width REAL NOT NULL DEFAULT 180.0",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "sync_startup_delay_minutes",
        "ALTER TABLE settings ADD COLUMN sync_startup_delay_minutes INTEGER NOT NULL DEFAULT 15",
    )?;

    migrations::ensure_column(
        conn,
        "settings",
        "minimize_to_tray",
        "ALTER TABLE settings ADD COLUMN minimize_to_tray INTEGER NOT NULL DEFAULT 0",
    )?;

    let had_time_slot = migrations::column_exists(conn, "settings", "time_slot_interval")?;
    let has_default_duration =
        migrations::column_exists(conn, "settings", "default_event_duration")?;

    if had_time_slot && !has_default_duration {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN default_event_duration INTEGER NOT NULL DEFAULT 60",
            [],
        )
        .context("Failed to add default_event_duration column")?;

        migrations::copy_column(
            conn,
            "settings",
            "time_slot_interval",
            "default_event_duration",
        )?;
    } else if !has_default_duration {
        conn.execute(
            "ALTER TABLE settings ADD COLUMN default_event_duration INTEGER NOT NULL DEFAULT 60",
            [],
        )
        .context("Failed to add default_event_duration column")?;
    }

    Ok(())
}

fn create_custom_themes_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS custom_themes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            is_dark INTEGER NOT NULL DEFAULT 0,
            app_background TEXT NOT NULL,
            calendar_background TEXT NOT NULL,
            weekend_background TEXT NOT NULL,
            today_background TEXT NOT NULL,
            today_border TEXT NOT NULL,
            day_background TEXT NOT NULL,
            day_border TEXT NOT NULL,
            text_primary TEXT NOT NULL,
            text_secondary TEXT NOT NULL,
            header_background TEXT,
            header_text TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .context("Failed to create custom_themes table")?;

    // Migration for header colors
    migrations::ensure_column(
        conn,
        "custom_themes",
        "header_background",
        "ALTER TABLE custom_themes ADD COLUMN header_background TEXT",
    )?;
    migrations::ensure_column(
        conn,
        "custom_themes",
        "header_text",
        "ALTER TABLE custom_themes ADD COLUMN header_text TEXT",
    )?;

    Ok(())
}

fn seed_custom_themes(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO custom_themes (name, is_dark, app_background, calendar_background,
            weekend_background, today_background, today_border, day_background, day_border,
            text_primary, text_secondary)
         VALUES ('Light', 0, '0.98,0.98,0.98', '1.0,1.0,1.0', '0.96,0.96,0.96',
                 '0.85,0.92,1.0', '0.3,0.5,0.9', '1.0,1.0,1.0', '0.85,0.85,0.85',
                 '0.1,0.1,0.1', '0.4,0.4,0.4')",
        [],
    )
    .context("Failed to insert light theme")?;

    conn.execute(
        "INSERT OR IGNORE INTO custom_themes (name, is_dark, app_background, calendar_background,
            weekend_background, today_background, today_border, day_background, day_border,
            text_primary, text_secondary)
         VALUES ('Dark', 1, '0.12,0.12,0.12', '0.15,0.15,0.15', '0.18,0.18,0.18',
                 '0.2,0.3,0.5', '0.4,0.6,1.0', '0.15,0.15,0.15', '0.3,0.3,0.3',
                 '0.95,0.95,0.95', '0.7,0.7,0.7')",
        [],
    )
    .context("Failed to insert dark theme")?;

    Ok(())
}

fn insert_default_settings(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO settings (
            id, theme, first_day_of_week, time_format, date_format,
            show_my_day, my_day_position_right, show_ribbon, current_view,
            default_event_duration, first_day_of_work_week, last_day_of_work_week,
            default_event_start_time, default_card_width, default_card_height,
            sync_startup_delay_minutes
        )
        VALUES (1, 'light', 0, '12h', 'MM/DD/YYYY', 0, 0, 0, 'Month', 60, 1, 5, '08:00', 120.0, 110.0, 15)",
        [],
    )
    .context("Failed to insert default settings")?;

    Ok(())
}

fn create_events_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            description TEXT,
            location TEXT,
            start_datetime TEXT NOT NULL,
            end_datetime TEXT NOT NULL,
            is_all_day INTEGER NOT NULL DEFAULT 0,
            category TEXT,
            color TEXT,
            recurrence_rule TEXT,
            recurrence_exceptions TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .context("Failed to create events table")?;

    Ok(())
}

fn create_event_templates_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS event_templates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            description TEXT,
            location TEXT,
            duration_minutes INTEGER NOT NULL DEFAULT 60,
            all_day INTEGER NOT NULL DEFAULT 0,
            category TEXT,
            color TEXT,
            recurrence_rule TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .context("Failed to create event_templates table")?;

    Ok(())
}

fn create_categories_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL,
            icon TEXT,
            is_system INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .context("Failed to create categories table")?;

    Ok(())
}

fn create_calendar_sources_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS calendar_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            source_type TEXT NOT NULL DEFAULT 'google_ics',
            ics_url TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            poll_interval_minutes INTEGER NOT NULL DEFAULT 15 CHECK (poll_interval_minutes > 0),
            last_sync_at TEXT,
            last_sync_status TEXT,
            last_error TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(name)
        )",
        [],
    )
    .context("Failed to create calendar_sources table")?;

    Ok(())
}

fn create_event_sync_map_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS event_sync_map (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL,
            external_uid TEXT NOT NULL,
            local_event_id INTEGER NOT NULL,
            external_last_modified TEXT,
            external_etag_hash TEXT,
            last_seen_at TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(source_id, external_uid),
            FOREIGN KEY (source_id) REFERENCES calendar_sources(id) ON DELETE CASCADE,
            FOREIGN KEY (local_event_id) REFERENCES events(id) ON DELETE CASCADE
        )",
        [],
    )
    .context("Failed to create event_sync_map table")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_event_sync_map_source_id ON event_sync_map(source_id)",
        [],
    )
    .context("Failed to create event_sync_map source index")?;

    Ok(())
}

fn initialize_default_categories(conn: &Connection) -> Result<()> {
    use crate::services::category::CategoryService;
    
    let service = CategoryService::new(conn);
    service.initialize_defaults()?;
    
    Ok(())
}

/// One-shot migration: normalise all-day events so start/end times are midnight
/// and the end date uses the iCal exclusive-end convention (one day past the last
/// visible day).  Events that already have midnight times are left untouched.
fn normalize_all_day_event_times(conn: &Connection) -> Result<()> {
    use chrono::{DateTime, Local, NaiveTime};

    let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

    let mut stmt = conn
        .prepare(
            "SELECT id, start_datetime, end_datetime
             FROM events
             WHERE is_all_day = 1",
        )
        .context("Failed to query all-day events for normalisation")?;

    let rows: Vec<(i64, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .context("Failed to read all-day event rows")?
        .filter_map(|r| r.ok())
        .collect();

    let mut update_stmt = conn
        .prepare(
            "UPDATE events SET start_datetime = ?1, end_datetime = ?2
             WHERE id = ?3",
        )
        .context("Failed to prepare all-day event update")?;

    for (id, start_str, end_str) in &rows {
        let Ok(start_dt) = DateTime::parse_from_rfc3339(start_str) else {
            continue;
        };
        let Ok(end_dt) = DateTime::parse_from_rfc3339(end_str) else {
            continue;
        };

        let start_local = start_dt.with_timezone(&Local);
        let end_local = end_dt.with_timezone(&Local);

        let start_needs_fix = start_local.time() != midnight;
        let end_needs_fix = end_local.time() != midnight;

        if !start_needs_fix && !end_needs_fix {
            continue;
        }

        // Normalise start to midnight of same date
        let new_start = start_local
            .date_naive()
            .and_time(midnight)
            .and_local_timezone(Local)
            .single()
            .unwrap_or(start_local);

        // For the end, treat the stored end_date as the user-intended inclusive
        // last day, then add one day (iCal exclusive-end convention).
        let inclusive_end_date = end_local.date_naive();
        let exclusive_end_date = inclusive_end_date
            .succ_opt()
            .unwrap_or(inclusive_end_date);
        let new_end = exclusive_end_date
            .and_time(midnight)
            .and_local_timezone(Local)
            .single()
            .unwrap_or(end_local);

        update_stmt
            .execute(rusqlite::params![
                new_start.to_rfc3339(),
                new_end.to_rfc3339(),
                id,
            ])
            .with_context(|| {
                format!("Failed to normalise all-day event id={}", id)
            })?;

        log::info!(
            "Normalised all-day event id={}: start {} → {}, end {} → {}",
            id,
            start_str,
            new_start.to_rfc3339(),
            end_str,
            new_end.to_rfc3339()
        );
    }

    Ok(())
}
