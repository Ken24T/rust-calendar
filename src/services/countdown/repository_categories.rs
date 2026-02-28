//! Category CRUD operations for the countdown repository.
//!
//! Handles loading, inserting, updating, and deleting countdown categories
//! from the `countdown_categories` SQLite table.

use anyhow::{Context, Result};
use rusqlite::{params, Row};

use super::models::{
    CountdownCardGeometry, CountdownCardVisuals, CountdownCategory, CountdownCategoryId,
    ContainerSortMode, RgbaColor,
};
use super::repository::CountdownRepository;

impl<'a> CountdownRepository<'a> {
    // ========== Category CRUD Operations ==========

    /// Get all countdown categories, ordered by display_order then name.
    pub fn get_all_categories(&self) -> Result<Vec<CountdownCategory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, display_order,
                    container_x, container_y, container_width, container_height,
                    default_title_bg_r, default_title_bg_g, default_title_bg_b, default_title_bg_a,
                    default_title_fg_r, default_title_fg_g, default_title_fg_b, default_title_fg_a,
                    default_title_font_size,
                    default_body_bg_r, default_body_bg_g, default_body_bg_b, default_body_bg_a,
                    default_days_fg_r, default_days_fg_g, default_days_fg_b, default_days_fg_a,
                    default_days_font_size,
                    default_card_width, default_card_height,
                    use_global_defaults,
                    is_collapsed, sort_mode
             FROM countdown_categories
             ORDER BY display_order, name",
        )?;

        let categories = stmt
            .query_map([], row_to_category)?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to fetch countdown categories")?;

        Ok(categories)
    }

    /// Get a single category by ID.
    #[allow(dead_code)]
    pub fn get_category(&self, id: CountdownCategoryId) -> Result<Option<CountdownCategory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, display_order,
                    container_x, container_y, container_width, container_height,
                    default_title_bg_r, default_title_bg_g, default_title_bg_b, default_title_bg_a,
                    default_title_fg_r, default_title_fg_g, default_title_fg_b, default_title_fg_a,
                    default_title_font_size,
                    default_body_bg_r, default_body_bg_g, default_body_bg_b, default_body_bg_a,
                    default_days_fg_r, default_days_fg_g, default_days_fg_b, default_days_fg_a,
                    default_days_font_size,
                    default_card_width, default_card_height,
                    use_global_defaults,
                    is_collapsed, sort_mode
             FROM countdown_categories
             WHERE id = ?",
        )?;

        use rusqlite::OptionalExtension;
        stmt.query_row([id.0], row_to_category)
            .optional()
            .context("Failed to fetch countdown category")
    }

    /// Insert a new category. Returns the auto-generated row ID.
    pub fn insert_category(&self, category: &CountdownCategory) -> Result<CountdownCategoryId> {
        let (cont_x, cont_y, cont_w, cont_h) = category
            .container_geometry
            .map(|g| (Some(g.x), Some(g.y), Some(g.width), Some(g.height)))
            .unwrap_or((None, None, None, None));

        let sort_mode_str = match category.sort_mode {
            ContainerSortMode::Date => "Date",
            ContainerSortMode::Manual => "Manual",
        };

        self.conn.execute(
            "INSERT INTO countdown_categories (
                name, display_order,
                container_x, container_y, container_width, container_height,
                default_title_bg_r, default_title_bg_g, default_title_bg_b, default_title_bg_a,
                default_title_fg_r, default_title_fg_g, default_title_fg_b, default_title_fg_a,
                default_title_font_size,
                default_body_bg_r, default_body_bg_g, default_body_bg_b, default_body_bg_a,
                default_days_fg_r, default_days_fg_g, default_days_fg_b, default_days_fg_a,
                default_days_font_size,
                default_card_width, default_card_height,
                use_global_defaults,
                is_collapsed, sort_mode
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14,
                ?15,
                ?16, ?17, ?18, ?19,
                ?20, ?21, ?22, ?23,
                ?24,
                ?25, ?26,
                ?27,
                ?28, ?29
            )",
            params![
                category.name,
                category.display_order,
                cont_x,
                cont_y,
                cont_w,
                cont_h,
                category.visual_defaults.title_bg_color.r,
                category.visual_defaults.title_bg_color.g,
                category.visual_defaults.title_bg_color.b,
                category.visual_defaults.title_bg_color.a,
                category.visual_defaults.title_fg_color.r,
                category.visual_defaults.title_fg_color.g,
                category.visual_defaults.title_fg_color.b,
                category.visual_defaults.title_fg_color.a,
                category.visual_defaults.title_font_size,
                category.visual_defaults.body_bg_color.r,
                category.visual_defaults.body_bg_color.g,
                category.visual_defaults.body_bg_color.b,
                category.visual_defaults.body_bg_color.a,
                category.visual_defaults.days_fg_color.r,
                category.visual_defaults.days_fg_color.g,
                category.visual_defaults.days_fg_color.b,
                category.visual_defaults.days_fg_color.a,
                category.visual_defaults.days_font_size,
                category.default_card_width,
                category.default_card_height,
                category.use_global_defaults,
                category.is_collapsed,
                sort_mode_str,
            ],
        )
        .context("Failed to insert countdown category")?;

        let id = self.conn.last_insert_rowid();
        Ok(CountdownCategoryId(id))
    }

    /// Update an existing category.
    pub fn update_category(&self, category: &CountdownCategory) -> Result<bool> {
        let (cont_x, cont_y, cont_w, cont_h) = category
            .container_geometry
            .map(|g| (Some(g.x), Some(g.y), Some(g.width), Some(g.height)))
            .unwrap_or((None, None, None, None));

        let sort_mode_str = match category.sort_mode {
            ContainerSortMode::Date => "Date",
            ContainerSortMode::Manual => "Manual",
        };

        let rows = self.conn.execute(
            "UPDATE countdown_categories SET
                name = ?2, display_order = ?3,
                container_x = ?4, container_y = ?5, container_width = ?6, container_height = ?7,
                default_title_bg_r = ?8, default_title_bg_g = ?9, default_title_bg_b = ?10, default_title_bg_a = ?11,
                default_title_fg_r = ?12, default_title_fg_g = ?13, default_title_fg_b = ?14, default_title_fg_a = ?15,
                default_title_font_size = ?16,
                default_body_bg_r = ?17, default_body_bg_g = ?18, default_body_bg_b = ?19, default_body_bg_a = ?20,
                default_days_fg_r = ?21, default_days_fg_g = ?22, default_days_fg_b = ?23, default_days_fg_a = ?24,
                default_days_font_size = ?25,
                default_card_width = ?26, default_card_height = ?27,
                use_global_defaults = ?28,
                is_collapsed = ?29, sort_mode = ?30,
                updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![
                category.id.0,
                category.name,
                category.display_order,
                cont_x,
                cont_y,
                cont_w,
                cont_h,
                category.visual_defaults.title_bg_color.r,
                category.visual_defaults.title_bg_color.g,
                category.visual_defaults.title_bg_color.b,
                category.visual_defaults.title_bg_color.a,
                category.visual_defaults.title_fg_color.r,
                category.visual_defaults.title_fg_color.g,
                category.visual_defaults.title_fg_color.b,
                category.visual_defaults.title_fg_color.a,
                category.visual_defaults.title_font_size,
                category.visual_defaults.body_bg_color.r,
                category.visual_defaults.body_bg_color.g,
                category.visual_defaults.body_bg_color.b,
                category.visual_defaults.body_bg_color.a,
                category.visual_defaults.days_fg_color.r,
                category.visual_defaults.days_fg_color.g,
                category.visual_defaults.days_fg_color.b,
                category.visual_defaults.days_fg_color.a,
                category.visual_defaults.days_font_size,
                category.default_card_width,
                category.default_card_height,
                category.use_global_defaults,
                category.is_collapsed,
                sort_mode_str,
            ],
        )
        .context("Failed to update countdown category")?;

        Ok(rows > 0)
    }

    /// Delete a category by ID. Cards in this category are reassigned to
    /// the default "General" category (id = 1) first.
    pub fn delete_category(&self, id: CountdownCategoryId) -> Result<bool> {
        use super::models::DEFAULT_CATEGORY_ID;

        if id.0 == DEFAULT_CATEGORY_ID {
            anyhow::bail!("Cannot delete the default 'General' category");
        }

        // Reassign cards to the default category
        self.conn.execute(
            "UPDATE countdown_cards SET category_id = ?1 WHERE category_id = ?2",
            params![DEFAULT_CATEGORY_ID, id.0],
        )?;

        let rows = self
            .conn
            .execute(
                "DELETE FROM countdown_categories WHERE id = ?",
                [id.0],
            )
            .context("Failed to delete countdown category")?;

        Ok(rows > 0)
    }
}

// ========== Helper Functions ==========

fn row_to_category(row: &Row<'_>) -> rusqlite::Result<CountdownCategory> {
    let id: i64 = row.get(0)?;
    let name: String = row.get(1)?;
    let display_order: i32 = row.get(2)?;

    let cont_x: Option<f32> = row.get(3)?;
    let cont_y: Option<f32> = row.get(4)?;
    let cont_w: Option<f32> = row.get(5)?;
    let cont_h: Option<f32> = row.get(6)?;

    let container_geometry = if cont_x.is_some() && cont_y.is_some() {
        Some(CountdownCardGeometry {
            x: cont_x.unwrap_or(0.0),
            y: cont_y.unwrap_or(0.0),
            width: cont_w.unwrap_or(400.0),
            height: cont_h.unwrap_or(300.0),
        })
    } else {
        None
    };

    let visual_defaults = CountdownCardVisuals {
        accent_color: None,
        always_on_top: false,
        use_default_title_bg: false,
        title_bg_color: RgbaColor::new(row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?),
        use_default_title_fg: false,
        title_fg_color: RgbaColor::new(row.get(11)?, row.get(12)?, row.get(13)?, row.get(14)?),
        title_font_size: row.get(15)?,
        use_default_body_bg: false,
        body_bg_color: RgbaColor::new(row.get(16)?, row.get(17)?, row.get(18)?, row.get(19)?),
        use_default_days_fg: false,
        days_fg_color: RgbaColor::new(row.get(20)?, row.get(21)?, row.get(22)?, row.get(23)?),
        days_font_size: row.get(24)?,
    };

    let is_collapsed: bool = row.get(28)?;
    let sort_mode_str: String = row.get(29)?;
    let sort_mode = match sort_mode_str.as_str() {
        "Manual" => ContainerSortMode::Manual,
        _ => ContainerSortMode::Date,
    };

    Ok(CountdownCategory {
        id: CountdownCategoryId(id),
        name,
        display_order,
        container_geometry,
        visual_defaults,
        default_card_width: row.get(25)?,
        default_card_height: row.get(26)?,
        use_global_defaults: row.get(27)?,
        is_collapsed,
        sort_mode,
    })
}
