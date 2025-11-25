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
            default_event_start_time, default_card_width, default_card_height
        )
        VALUES (1, 'light', 0, '12h', 'MM/DD/YYYY', 0, 0, 0, 'Month', 60, 1, 5, '08:00', 120.0, 110.0)",
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
