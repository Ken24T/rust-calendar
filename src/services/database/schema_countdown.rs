//! Countdown-related database schema: table creation and migrations.

use anyhow::{Context, Result};
use rusqlite::Connection;

use super::migrations;

pub(super) fn create_countdown_tables(conn: &Connection) -> Result<()> {
    // Main countdown cards table
    // event_id can be NULL for standalone countdowns (not linked to an event)
    // ON DELETE CASCADE ensures cards are automatically deleted when their event is deleted
    conn.execute(
        "CREATE TABLE IF NOT EXISTS countdown_cards (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id INTEGER,
            event_title TEXT NOT NULL,
            start_at TEXT NOT NULL,
            title_override TEXT,
            auto_title_override INTEGER NOT NULL DEFAULT 0,
            comment TEXT,
            event_color TEXT,
            
            -- Geometry
            geometry_x REAL NOT NULL DEFAULT 50.0,
            geometry_y REAL NOT NULL DEFAULT 50.0,
            geometry_width REAL NOT NULL DEFAULT 138.0,
            geometry_height REAL NOT NULL DEFAULT 128.0,
            
            -- Visual settings
            accent_color TEXT,
            always_on_top INTEGER NOT NULL DEFAULT 0,
            compact_mode INTEGER NOT NULL DEFAULT 0,
            use_default_title_bg INTEGER NOT NULL DEFAULT 0,
            title_bg_r INTEGER NOT NULL DEFAULT 10,
            title_bg_g INTEGER NOT NULL DEFAULT 34,
            title_bg_b INTEGER NOT NULL DEFAULT 145,
            title_bg_a INTEGER NOT NULL DEFAULT 255,
            use_default_title_fg INTEGER NOT NULL DEFAULT 0,
            title_fg_r INTEGER NOT NULL DEFAULT 255,
            title_fg_g INTEGER NOT NULL DEFAULT 255,
            title_fg_b INTEGER NOT NULL DEFAULT 255,
            title_fg_a INTEGER NOT NULL DEFAULT 255,
            title_font_size REAL NOT NULL DEFAULT 20.0,
            use_default_body_bg INTEGER NOT NULL DEFAULT 0,
            body_bg_r INTEGER NOT NULL DEFAULT 103,
            body_bg_g INTEGER NOT NULL DEFAULT 176,
            body_bg_b INTEGER NOT NULL DEFAULT 255,
            body_bg_a INTEGER NOT NULL DEFAULT 255,
            use_default_days_fg INTEGER NOT NULL DEFAULT 0,
            days_fg_r INTEGER NOT NULL DEFAULT 15,
            days_fg_g INTEGER NOT NULL DEFAULT 32,
            days_fg_b INTEGER NOT NULL DEFAULT 70,
            days_fg_a INTEGER NOT NULL DEFAULT 255,
            days_font_size REAL NOT NULL DEFAULT 80.0,
            
            -- Auto-dismiss settings
            auto_dismiss_enabled INTEGER NOT NULL DEFAULT 0,
            auto_dismiss_on_event_start INTEGER NOT NULL DEFAULT 1,
            auto_dismiss_on_event_end INTEGER NOT NULL DEFAULT 0,
            auto_dismiss_delay_seconds INTEGER NOT NULL DEFAULT 10,
            
            -- Runtime state (not critical but useful to persist)
            last_computed_days INTEGER,
            last_warning_state TEXT,
            last_notification_time TEXT,
            
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            
            FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
        )",
        [],
    )
    .context("Failed to create countdown_cards table")?;

    // Global countdown settings (single row, similar to settings table)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS countdown_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            next_card_id INTEGER NOT NULL DEFAULT 1,
            
            -- App window geometry
            app_window_x REAL,
            app_window_y REAL,
            app_window_width REAL,
            app_window_height REAL,
            
            -- Visual defaults
            default_title_bg_r INTEGER NOT NULL DEFAULT 10,
            default_title_bg_g INTEGER NOT NULL DEFAULT 34,
            default_title_bg_b INTEGER NOT NULL DEFAULT 145,
            default_title_bg_a INTEGER NOT NULL DEFAULT 255,
            default_title_fg_r INTEGER NOT NULL DEFAULT 255,
            default_title_fg_g INTEGER NOT NULL DEFAULT 255,
            default_title_fg_b INTEGER NOT NULL DEFAULT 255,
            default_title_fg_a INTEGER NOT NULL DEFAULT 255,
            default_title_font_size REAL NOT NULL DEFAULT 20.0,
            default_body_bg_r INTEGER NOT NULL DEFAULT 103,
            default_body_bg_g INTEGER NOT NULL DEFAULT 176,
            default_body_bg_b INTEGER NOT NULL DEFAULT 255,
            default_body_bg_a INTEGER NOT NULL DEFAULT 255,
            default_days_fg_r INTEGER NOT NULL DEFAULT 15,
            default_days_fg_g INTEGER NOT NULL DEFAULT 32,
            default_days_fg_b INTEGER NOT NULL DEFAULT 70,
            default_days_fg_a INTEGER NOT NULL DEFAULT 255,
            default_days_font_size REAL NOT NULL DEFAULT 80.0,
            
            -- Notification config
            notifications_enabled INTEGER NOT NULL DEFAULT 1,
            use_visual_warnings INTEGER NOT NULL DEFAULT 1,
            use_system_notifications INTEGER NOT NULL DEFAULT 1,
            approaching_hours INTEGER NOT NULL DEFAULT 24,
            imminent_hours INTEGER NOT NULL DEFAULT 1,
            critical_minutes INTEGER NOT NULL DEFAULT 5,
            
            -- Auto-dismiss defaults
            auto_dismiss_enabled INTEGER NOT NULL DEFAULT 0,
            auto_dismiss_on_event_start INTEGER NOT NULL DEFAULT 1,
            auto_dismiss_on_event_end INTEGER NOT NULL DEFAULT 0,
            auto_dismiss_delay_seconds INTEGER NOT NULL DEFAULT 10,
            
            -- Container mode settings
            display_mode TEXT NOT NULL DEFAULT 'individual',
            container_x REAL,
            container_y REAL,
            container_width REAL,
            container_height REAL,
            card_order TEXT,
            
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .context("Failed to create countdown_settings table")?;

    // Insert default countdown settings
    conn.execute(
        "INSERT OR IGNORE INTO countdown_settings (id) VALUES (1)",
        [],
    )
    .context("Failed to insert default countdown settings")?;

    // Countdown categories table for grouping cards
    conn.execute(
        "CREATE TABLE IF NOT EXISTS countdown_categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            display_order INTEGER NOT NULL DEFAULT 0,

            -- Container geometry for CategoryContainers mode
            container_x REAL,
            container_y REAL,
            container_width REAL,
            container_height REAL,

            -- Category-level visual defaults (three-tier: Global → Category → Card)
            default_title_bg_r INTEGER NOT NULL DEFAULT 10,
            default_title_bg_g INTEGER NOT NULL DEFAULT 34,
            default_title_bg_b INTEGER NOT NULL DEFAULT 145,
            default_title_bg_a INTEGER NOT NULL DEFAULT 255,
            default_title_fg_r INTEGER NOT NULL DEFAULT 255,
            default_title_fg_g INTEGER NOT NULL DEFAULT 255,
            default_title_fg_b INTEGER NOT NULL DEFAULT 255,
            default_title_fg_a INTEGER NOT NULL DEFAULT 255,
            default_title_font_size REAL NOT NULL DEFAULT 20.0,
            default_body_bg_r INTEGER NOT NULL DEFAULT 103,
            default_body_bg_g INTEGER NOT NULL DEFAULT 176,
            default_body_bg_b INTEGER NOT NULL DEFAULT 255,
            default_body_bg_a INTEGER NOT NULL DEFAULT 255,
            default_days_fg_r INTEGER NOT NULL DEFAULT 15,
            default_days_fg_g INTEGER NOT NULL DEFAULT 32,
            default_days_fg_b INTEGER NOT NULL DEFAULT 70,
            default_days_fg_a INTEGER NOT NULL DEFAULT 255,
            default_days_font_size REAL NOT NULL DEFAULT 80.0,

            -- Default card dimensions for new cards in this category
            default_card_width REAL NOT NULL DEFAULT 120.0,
            default_card_height REAL NOT NULL DEFAULT 110.0,

            -- When true, category inherits global defaults instead of its own
            use_global_defaults INTEGER NOT NULL DEFAULT 1,

            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .context("Failed to create countdown_categories table")?;

    // Seed the default "General" category (id = 1)
    conn.execute(
        "INSERT OR IGNORE INTO countdown_categories (id, name, display_order) VALUES (1, 'General', 0)",
        [],
    )
    .context("Failed to seed default countdown category")?;

    // Card visual templates table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS countdown_card_templates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,

            -- Visual settings (same fields as countdown card visuals)
            title_bg_r INTEGER NOT NULL DEFAULT 10,
            title_bg_g INTEGER NOT NULL DEFAULT 34,
            title_bg_b INTEGER NOT NULL DEFAULT 145,
            title_bg_a INTEGER NOT NULL DEFAULT 255,
            title_fg_r INTEGER NOT NULL DEFAULT 255,
            title_fg_g INTEGER NOT NULL DEFAULT 255,
            title_fg_b INTEGER NOT NULL DEFAULT 255,
            title_fg_a INTEGER NOT NULL DEFAULT 255,
            title_font_size REAL NOT NULL DEFAULT 20.0,
            body_bg_r INTEGER NOT NULL DEFAULT 103,
            body_bg_g INTEGER NOT NULL DEFAULT 176,
            body_bg_b INTEGER NOT NULL DEFAULT 255,
            body_bg_a INTEGER NOT NULL DEFAULT 255,
            days_fg_r INTEGER NOT NULL DEFAULT 15,
            days_fg_g INTEGER NOT NULL DEFAULT 32,
            days_fg_b INTEGER NOT NULL DEFAULT 70,
            days_fg_a INTEGER NOT NULL DEFAULT 255,
            days_font_size REAL NOT NULL DEFAULT 80.0,

            -- Default card dimensions
            default_card_width REAL NOT NULL DEFAULT 120.0,
            default_card_height REAL NOT NULL DEFAULT 110.0,

            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .context("Failed to create countdown_card_templates table")?;

    // Seed the default template (id = 1)
    conn.execute(
        "INSERT OR IGNORE INTO countdown_card_templates (id, name) VALUES (1, 'Default')",
        [],
    )
    .context("Failed to seed default card template")?;

    Ok(())
}

pub(super) fn run_countdown_migrations(conn: &Connection) -> Result<()> {
    // Add container mode columns to countdown_settings
    migrations::ensure_column(
        conn,
        "countdown_settings",
        "display_mode",
        "ALTER TABLE countdown_settings ADD COLUMN display_mode TEXT NOT NULL DEFAULT 'IndividualWindows'",
    )?;

    migrations::ensure_column(
        conn,
        "countdown_settings",
        "container_geometry_x",
        "ALTER TABLE countdown_settings ADD COLUMN container_geometry_x REAL",
    )?;

    migrations::ensure_column(
        conn,
        "countdown_settings",
        "container_geometry_y",
        "ALTER TABLE countdown_settings ADD COLUMN container_geometry_y REAL",
    )?;

    migrations::ensure_column(
        conn,
        "countdown_settings",
        "container_geometry_width",
        "ALTER TABLE countdown_settings ADD COLUMN container_geometry_width REAL",
    )?;

    migrations::ensure_column(
        conn,
        "countdown_settings",
        "container_geometry_height",
        "ALTER TABLE countdown_settings ADD COLUMN container_geometry_height REAL",
    )?;

    migrations::ensure_column(
        conn,
        "countdown_settings",
        "card_order",
        "ALTER TABLE countdown_settings ADD COLUMN card_order TEXT",
    )?;

    // Add event_start and event_end columns for enhanced tooltip display
    migrations::ensure_column(
        conn,
        "countdown_cards",
        "event_start",
        "ALTER TABLE countdown_cards ADD COLUMN event_start TEXT",
    )?;

    migrations::ensure_column(
        conn,
        "countdown_cards",
        "event_end",
        "ALTER TABLE countdown_cards ADD COLUMN event_end TEXT",
    )?;

    // Reset use_default_* flags to 0 for existing cards (one-time migration)
    // This ensures checkboxes start unchecked by default
    migrate_use_default_flags(conn)?;

    // Add category_id column to countdown_cards (defaults to 1 = "General")
    // Note: SQLite forbids REFERENCES in ALTER TABLE ADD COLUMN with a non-NULL default,
    // so we omit the FK constraint here; referential integrity is enforced by application logic.
    migrations::ensure_column(
        conn,
        "countdown_cards",
        "category_id",
        "ALTER TABLE countdown_cards ADD COLUMN category_id INTEGER NOT NULL DEFAULT 1",
    )?;

    // Add is_collapsed column to countdown_categories
    migrations::ensure_column(
        conn,
        "countdown_categories",
        "is_collapsed",
        "ALTER TABLE countdown_categories ADD COLUMN is_collapsed INTEGER NOT NULL DEFAULT 0",
    )?;

    // Add sort_mode column to countdown_categories
    migrations::ensure_column(
        conn,
        "countdown_categories",
        "sort_mode",
        "ALTER TABLE countdown_categories ADD COLUMN sort_mode TEXT NOT NULL DEFAULT 'Date'",
    )?;

    // Add template_id column to countdown_categories
    migrations::ensure_column(
        conn,
        "countdown_categories",
        "template_id",
        "ALTER TABLE countdown_categories ADD COLUMN template_id INTEGER",
    )?;

    // Add orientation column to countdown_categories
    migrations::ensure_column(
        conn,
        "countdown_categories",
        "orientation",
        "ALTER TABLE countdown_categories ADD COLUMN orientation TEXT NOT NULL DEFAULT 'Auto'",
    )?;

    // Migrate existing category visual defaults into templates (one-time)
    migrate_category_visuals_to_templates(conn)?;

    Ok(())
}

/// One-time migration to set use_default_* flags to 0 for all existing cards
fn migrate_use_default_flags(conn: &Connection) -> Result<()> {
    // Check if we've already run this migration by looking for a marker
    let marker_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM countdown_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !marker_exists {
        return Ok(()); // No settings row yet, nothing to migrate
    }

    // Check if migration was already done by checking if any card has use_default_* = 1
    // If all cards have 0, we've already migrated (or there are no cards)
    let needs_migration: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM countdown_cards WHERE 
             use_default_title_bg = 1 OR use_default_title_fg = 1 OR 
             use_default_body_bg = 1 OR use_default_days_fg = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if needs_migration {
        log::info!("Migrating countdown cards: setting use_default_* flags to 0");
        conn.execute(
            "UPDATE countdown_cards SET 
             use_default_title_bg = 0, 
             use_default_title_fg = 0, 
             use_default_body_bg = 0, 
             use_default_days_fg = 0",
            [],
        )?;
    }

    Ok(())
}

/// One-time migration: for each category that has `use_global_defaults = 0`
/// and no `template_id`, create a template from its visual defaults and
/// link it.
fn migrate_category_visuals_to_templates(conn: &Connection) -> Result<()> {
    use rusqlite::params;

    /// Helper struct for migration rows (avoids clippy::type_complexity).
    struct MigrationRow {
        cat_id: i64,
        cat_name: String,
        tbg: [u8; 4],
        tfg: [u8; 4],
        title_font: f64,
        bbg: [u8; 4],
        dfg: [u8; 4],
        days_font: f64,
        card_w: f64,
        card_h: f64,
    }

    // Find categories with custom visuals but no template yet
    let mut stmt = conn.prepare(
        "SELECT id, name,
                default_title_bg_r, default_title_bg_g, default_title_bg_b, default_title_bg_a,
                default_title_fg_r, default_title_fg_g, default_title_fg_b, default_title_fg_a,
                default_title_font_size,
                default_body_bg_r, default_body_bg_g, default_body_bg_b, default_body_bg_a,
                default_days_fg_r, default_days_fg_g, default_days_fg_b, default_days_fg_a,
                default_days_font_size,
                default_card_width, default_card_height
         FROM countdown_categories
         WHERE use_global_defaults = 0 AND (template_id IS NULL)",
    )?;

    let rows: Vec<MigrationRow> = stmt
        .query_map([], |row| {
            Ok(MigrationRow {
                cat_id: row.get(0)?,
                cat_name: row.get(1)?,
                tbg: [row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?],
                tfg: [row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?],
                title_font: row.get(10)?,
                bbg: [row.get(11)?, row.get(12)?, row.get(13)?, row.get(14)?],
                dfg: [row.get(15)?, row.get(16)?, row.get(17)?, row.get(18)?],
                days_font: row.get(19)?,
                card_w: row.get(20)?,
                card_h: row.get(21)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for r in &rows
    {
        let tmpl_name = format!("{} (migrated)", r.cat_name);

        // Check if a template with this name already exists (idempotent)
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM countdown_card_templates WHERE name = ?",
                [&tmpl_name],
                |row| row.get(0),
            )
            .unwrap_or(false);
        if exists {
            continue;
        }

        log::info!(
            "Migrating category '{}' (id={}) visual defaults to template '{}'",
            r.cat_name, r.cat_id, tmpl_name
        );

        conn.execute(
            "INSERT INTO countdown_card_templates (
                name,
                title_bg_r, title_bg_g, title_bg_b, title_bg_a,
                title_fg_r, title_fg_g, title_fg_b, title_fg_a,
                title_font_size,
                body_bg_r, body_bg_g, body_bg_b, body_bg_a,
                days_fg_r, days_fg_g, days_fg_b, days_fg_a,
                days_font_size,
                default_card_width, default_card_height
            ) VALUES (?1, ?2,?3,?4,?5, ?6,?7,?8,?9, ?10, ?11,?12,?13,?14, ?15,?16,?17,?18, ?19, ?20,?21)",
            params![
                tmpl_name,
                r.tbg[0], r.tbg[1], r.tbg[2], r.tbg[3],
                r.tfg[0], r.tfg[1], r.tfg[2], r.tfg[3],
                r.title_font,
                r.bbg[0], r.bbg[1], r.bbg[2], r.bbg[3],
                r.dfg[0], r.dfg[1], r.dfg[2], r.dfg[3],
                r.days_font,
                r.card_w, r.card_h,
            ],
        )?;

        let tmpl_id = conn.last_insert_rowid();

        conn.execute(
            "UPDATE countdown_categories SET template_id = ? WHERE id = ?",
            params![tmpl_id, r.cat_id],
        )?;
    }

    Ok(())
}
