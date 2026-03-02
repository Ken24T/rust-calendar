//! Template CRUD operations for the countdown repository.
//!
//! Handles loading, inserting, updating, and deleting countdown card
//! templates from the `countdown_card_templates` SQLite table.

use anyhow::{Context, Result};
use rusqlite::{params, Row};

use super::models::{
    CountdownCardTemplate, CountdownCardTemplateId, CountdownCardVisuals, RgbaColor,
    DEFAULT_TEMPLATE_ID,
};
use super::repository::CountdownRepository;

impl<'a> CountdownRepository<'a> {
    // ========== Template CRUD Operations ==========

    /// Get all card templates, ordered by name.
    pub fn get_all_templates(&self) -> Result<Vec<CountdownCardTemplate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name,
                    title_bg_r, title_bg_g, title_bg_b, title_bg_a,
                    title_fg_r, title_fg_g, title_fg_b, title_fg_a,
                    title_font_size,
                    body_bg_r, body_bg_g, body_bg_b, body_bg_a,
                    days_fg_r, days_fg_g, days_fg_b, days_fg_a,
                    days_font_size,
                    default_card_width, default_card_height
             FROM countdown_card_templates
             ORDER BY name",
        )?;

        let templates = stmt
            .query_map([], row_to_template)?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to fetch countdown card templates")?;

        Ok(templates)
    }

    /// Get a single template by ID.
    #[allow(dead_code)]
    pub fn get_template(
        &self,
        id: CountdownCardTemplateId,
    ) -> Result<Option<CountdownCardTemplate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name,
                    title_bg_r, title_bg_g, title_bg_b, title_bg_a,
                    title_fg_r, title_fg_g, title_fg_b, title_fg_a,
                    title_font_size,
                    body_bg_r, body_bg_g, body_bg_b, body_bg_a,
                    days_fg_r, days_fg_g, days_fg_b, days_fg_a,
                    days_font_size,
                    default_card_width, default_card_height
             FROM countdown_card_templates
             WHERE id = ?",
        )?;

        use rusqlite::OptionalExtension;
        stmt.query_row([id.0], row_to_template)
            .optional()
            .context("Failed to fetch countdown card template")
    }

    /// Insert a new template.  Returns the auto-generated row ID.
    pub fn insert_template(
        &self,
        template: &CountdownCardTemplate,
    ) -> Result<CountdownCardTemplateId> {
        self.conn
            .execute(
                "INSERT INTO countdown_card_templates (
                    name,
                    title_bg_r, title_bg_g, title_bg_b, title_bg_a,
                    title_fg_r, title_fg_g, title_fg_b, title_fg_a,
                    title_font_size,
                    body_bg_r, body_bg_g, body_bg_b, body_bg_a,
                    days_fg_r, days_fg_g, days_fg_b, days_fg_a,
                    days_font_size,
                    default_card_width, default_card_height
                ) VALUES (
                    ?1,
                    ?2, ?3, ?4, ?5,
                    ?6, ?7, ?8, ?9,
                    ?10,
                    ?11, ?12, ?13, ?14,
                    ?15, ?16, ?17, ?18,
                    ?19,
                    ?20, ?21
                )",
                params![
                    template.name,
                    template.visuals.title_bg_color.r,
                    template.visuals.title_bg_color.g,
                    template.visuals.title_bg_color.b,
                    template.visuals.title_bg_color.a,
                    template.visuals.title_fg_color.r,
                    template.visuals.title_fg_color.g,
                    template.visuals.title_fg_color.b,
                    template.visuals.title_fg_color.a,
                    template.visuals.title_font_size,
                    template.visuals.body_bg_color.r,
                    template.visuals.body_bg_color.g,
                    template.visuals.body_bg_color.b,
                    template.visuals.body_bg_color.a,
                    template.visuals.days_fg_color.r,
                    template.visuals.days_fg_color.g,
                    template.visuals.days_fg_color.b,
                    template.visuals.days_fg_color.a,
                    template.visuals.days_font_size,
                    template.default_card_width,
                    template.default_card_height,
                ],
            )
            .context("Failed to insert countdown card template")?;

        let id = self.conn.last_insert_rowid();
        Ok(CountdownCardTemplateId(id))
    }

    /// Update an existing template.
    pub fn update_template(&self, template: &CountdownCardTemplate) -> Result<bool> {
        let rows = self
            .conn
            .execute(
                "UPDATE countdown_card_templates SET
                    name = ?2,
                    title_bg_r = ?3, title_bg_g = ?4, title_bg_b = ?5, title_bg_a = ?6,
                    title_fg_r = ?7, title_fg_g = ?8, title_fg_b = ?9, title_fg_a = ?10,
                    title_font_size = ?11,
                    body_bg_r = ?12, body_bg_g = ?13, body_bg_b = ?14, body_bg_a = ?15,
                    days_fg_r = ?16, days_fg_g = ?17, days_fg_b = ?18, days_fg_a = ?19,
                    days_font_size = ?20,
                    default_card_width = ?21, default_card_height = ?22,
                    updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![
                    template.id.0,
                    template.name,
                    template.visuals.title_bg_color.r,
                    template.visuals.title_bg_color.g,
                    template.visuals.title_bg_color.b,
                    template.visuals.title_bg_color.a,
                    template.visuals.title_fg_color.r,
                    template.visuals.title_fg_color.g,
                    template.visuals.title_fg_color.b,
                    template.visuals.title_fg_color.a,
                    template.visuals.title_font_size,
                    template.visuals.body_bg_color.r,
                    template.visuals.body_bg_color.g,
                    template.visuals.body_bg_color.b,
                    template.visuals.body_bg_color.a,
                    template.visuals.days_fg_color.r,
                    template.visuals.days_fg_color.g,
                    template.visuals.days_fg_color.b,
                    template.visuals.days_fg_color.a,
                    template.visuals.days_font_size,
                    template.default_card_width,
                    template.default_card_height,
                ],
            )
            .context("Failed to update countdown card template")?;

        Ok(rows > 0)
    }

    /// Delete a template by ID.
    ///
    /// Categories referencing this template have their `template_id`
    /// set to NULL (falling back to global defaults).
    pub fn delete_template(&self, id: CountdownCardTemplateId) -> Result<bool> {
        if id.0 == DEFAULT_TEMPLATE_ID {
            anyhow::bail!("Cannot delete the default template");
        }

        // Clear template references in categories
        self.conn.execute(
            "UPDATE countdown_categories SET template_id = NULL WHERE template_id = ?",
            [id.0],
        )?;

        let rows = self
            .conn
            .execute(
                "DELETE FROM countdown_card_templates WHERE id = ?",
                [id.0],
            )
            .context("Failed to delete countdown card template")?;

        Ok(rows > 0)
    }
}

// ========== Helper Functions ==========

fn row_to_template(row: &Row<'_>) -> rusqlite::Result<CountdownCardTemplate> {
    let id: i64 = row.get(0)?;
    let name: String = row.get(1)?;

    let title_bg = RgbaColor::new(row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?);
    let title_fg = RgbaColor::new(row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?);
    let title_font_size: f32 = row.get::<_, f64>(10)? as f32;
    let body_bg = RgbaColor::new(row.get(11)?, row.get(12)?, row.get(13)?, row.get(14)?);
    let days_fg = RgbaColor::new(row.get(15)?, row.get(16)?, row.get(17)?, row.get(18)?);
    let days_font_size: f32 = row.get::<_, f64>(19)? as f32;
    let default_card_width: f32 = row.get::<_, f64>(20)? as f32;
    let default_card_height: f32 = row.get::<_, f64>(21)? as f32;

    Ok(CountdownCardTemplate {
        id: CountdownCardTemplateId(id),
        name,
        visuals: CountdownCardVisuals {
            accent_color: None,
            always_on_top: false,
            use_default_title_bg: false,
            title_bg_color: title_bg,
            use_default_title_fg: false,
            title_fg_color: title_fg,
            title_font_size,
            use_default_body_bg: false,
            body_bg_color: body_bg,
            use_default_days_fg: false,
            days_fg_color: days_fg,
            days_font_size,
        },
        default_card_width,
        default_card_height,
    })
}
