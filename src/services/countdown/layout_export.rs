//! Export and import of countdown layout configuration as JSON.
//!
//! The layout file captures the visual and structural configuration needed
//! to reproduce a countdown setup on another machine:
//! - Card templates (colours, fonts, default dimensions)
//! - Categories (names, ordering, template assignments, orientation, dimensions)
//! - Global visual defaults
//! - Display mode and container geometry
//!
//! **Not included:** individual countdown cards (these are tied to specific
//! events and are not portable).

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::models::{
    CountdownCardTemplate, CountdownCardVisuals,
    CountdownCategory, CountdownDisplayMode, LayoutOrientation,
};
use super::service::CountdownService;

/// Schema version for forward-compatibility checks.
const LAYOUT_SCHEMA_VERSION: u32 = 1;

/// A portable representation of countdown layout configuration.
///
/// This can be serialised to JSON and loaded on a different machine to
/// reproduce the same category structure, templates, and visual defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountdownLayoutExport {
    /// Schema version — importers should reject files with a higher version.
    pub schema_version: u32,
    /// Human-readable description (optional).
    #[serde(default)]
    pub description: String,
    /// Global visual defaults for countdown cards.
    pub visual_defaults: CountdownCardVisuals,
    /// Display mode preference.
    pub display_mode: CountdownDisplayMode,
    /// Reusable card visual templates.
    #[serde(default)]
    pub templates: Vec<CountdownCardTemplate>,
    /// Category definitions (names, ordering, template assignments, orientation).
    #[serde(default)]
    pub categories: Vec<ExportedCategory>,
}

/// A category definition for export — contains only the fields that are
/// needed to reproduce the layout (excludes runtime state like `is_collapsed`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedCategory {
    pub name: String,
    pub display_order: i32,
    /// Name of the assigned template (resolved by name on import, not by ID).
    #[serde(default)]
    pub template_name: Option<String>,
    #[serde(default)]
    pub orientation: LayoutOrientation,
    #[serde(default = "default_card_width")]
    pub default_card_width: f32,
    #[serde(default = "default_card_height")]
    pub default_card_height: f32,
}

fn default_card_width() -> f32 {
    120.0
}

fn default_card_height() -> f32 {
    110.0
}

impl CountdownService {
    /// Build a portable layout export from the current countdown state.
    pub fn export_layout(&self) -> CountdownLayoutExport {
        let categories = self
            .categories
            .iter()
            .map(|cat| {
                let template_name = cat.template_id.and_then(|tid| {
                    self.templates
                        .iter()
                        .find(|t| t.id == tid)
                        .map(|t| t.name.clone())
                });
                ExportedCategory {
                    name: cat.name.clone(),
                    display_order: cat.display_order,
                    template_name,
                    orientation: cat.orientation,
                    default_card_width: cat.default_card_width,
                    default_card_height: cat.default_card_height,
                }
            })
            .collect();

        CountdownLayoutExport {
            schema_version: LAYOUT_SCHEMA_VERSION,
            description: String::new(),
            visual_defaults: self.visual_defaults.clone(),
            display_mode: self.display_mode,
            templates: self.templates.clone(),
            categories,
        }
    }

    /// Write the current layout to a JSON file.
    pub fn export_layout_to_file(&self, path: &Path) -> Result<()> {
        let layout = self.export_layout();
        let json = serde_json::to_string_pretty(&layout)
            .context("Failed to serialise countdown layout")?;
        std::fs::write(path, json).context("Failed to write countdown layout file")?;
        Ok(())
    }

    /// Read a layout from a JSON file.
    pub fn read_layout_file(path: &Path) -> Result<CountdownLayoutExport> {
        let json =
            std::fs::read_to_string(path).context("Failed to read countdown layout file")?;
        let layout: CountdownLayoutExport =
            serde_json::from_str(&json).context("Failed to parse countdown layout JSON")?;

        if layout.schema_version > LAYOUT_SCHEMA_VERSION {
            anyhow::bail!(
                "Layout file uses schema version {} but this app only supports up to version {}",
                layout.schema_version,
                LAYOUT_SCHEMA_VERSION,
            );
        }

        Ok(layout)
    }

    /// Apply an imported layout to the current service state.
    ///
    /// This **merges** with existing data:
    /// - Templates with the same name are updated; new templates are added.
    /// - Categories with the same name are updated; new categories are added.
    /// - Visual defaults and display mode are replaced.
    /// - Container geometry is **not** imported (not portable across screens).
    ///
    /// Returns a summary of what was changed.
    pub fn import_layout(&mut self, layout: CountdownLayoutExport) -> ImportSummary {
        let mut summary = ImportSummary::default();

        // --- Visual defaults ---
        self.visual_defaults = layout.visual_defaults;
        self.display_mode = layout.display_mode;

        // --- Templates ---
        for imported in &layout.templates {
            if let Some(existing) = self
                .templates
                .iter_mut()
                .find(|t| t.name == imported.name)
            {
                // Update existing template visuals/dimensions (keep existing ID)
                existing.visuals = imported.visuals.clone();
                existing.default_card_width = imported.default_card_width;
                existing.default_card_height = imported.default_card_height;
                summary.templates_updated += 1;
            } else {
                // Add new template with a fresh ID
                let new_id = self.next_template_id();
                let mut new_template = imported.clone();
                new_template.id = new_id;
                self.templates.push(new_template);
                summary.templates_added += 1;
            }
        }

        // --- Categories ---
        for imported_cat in &layout.categories {
            // Resolve template by name
            let template_id = imported_cat.template_name.as_ref().and_then(|name| {
                self.templates.iter().find(|t| &t.name == name).map(|t| t.id)
            });

            if let Some(existing) = self
                .categories
                .iter_mut()
                .find(|c| c.name == imported_cat.name)
            {
                // Update existing category
                existing.display_order = imported_cat.display_order;
                existing.template_id = template_id;
                existing.orientation = imported_cat.orientation;
                existing.default_card_width = imported_cat.default_card_width;
                existing.default_card_height = imported_cat.default_card_height;
                summary.categories_updated += 1;
            } else {
                // Add new category
                let new_cat = CountdownCategory {
                    id: self.next_category_id(),
                    name: imported_cat.name.clone(),
                    display_order: imported_cat.display_order,
                    container_geometry: None,
                    template_id,
                    orientation: imported_cat.orientation,
                    visual_defaults: CountdownCardVisuals::default(),
                    default_card_width: imported_cat.default_card_width,
                    default_card_height: imported_cat.default_card_height,
                    use_global_defaults: true,
                    is_collapsed: false,
                    sort_mode: Default::default(),
                };
                self.categories.push(new_cat);
                summary.categories_added += 1;
            }
        }

        self.dirty = true;
        summary
    }
}

/// Summary of an import operation, used for user feedback.
#[derive(Debug, Default)]
pub struct ImportSummary {
    pub templates_added: usize,
    pub templates_updated: usize,
    pub categories_added: usize,
    pub categories_updated: usize,
}

impl std::fmt::Display for ImportSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if self.templates_added > 0 {
            parts.push(format!("{} templates added", self.templates_added));
        }
        if self.templates_updated > 0 {
            parts.push(format!("{} templates updated", self.templates_updated));
        }
        if self.categories_added > 0 {
            parts.push(format!("{} categories added", self.categories_added));
        }
        if self.categories_updated > 0 {
            parts.push(format!("{} categories updated", self.categories_updated));
        }
        if parts.is_empty() {
            write!(f, "No changes applied")
        } else {
            write!(f, "{}", parts.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::countdown::models::{
        CountdownCardTemplateId, CountdownCategory, CountdownCategoryId,
        DEFAULT_CATEGORY_ID, DEFAULT_TEMPLATE_ID, LayoutOrientation,
    };

    fn make_test_service() -> CountdownService {
        let mut service = CountdownService::new();
        // Seed a default "General" category for tests
        if service.categories.is_empty() {
            service.categories.push(CountdownCategory {
                id: CountdownCategoryId(DEFAULT_CATEGORY_ID),
                name: "General".to_string(),
                display_order: 0,
                container_geometry: None,
                template_id: None,
                orientation: LayoutOrientation::Landscape,
                visual_defaults: super::CountdownCardVisuals::default(),
                default_card_width: 120.0,
                default_card_height: 110.0,
                use_global_defaults: true,
                is_collapsed: false,
                sort_mode: Default::default(),
            });
        }
        service
    }

    #[test]
    fn test_export_roundtrip() {
        let service = make_test_service();
        let layout = service.export_layout();

        let json = serde_json::to_string_pretty(&layout).unwrap();
        let parsed: CountdownLayoutExport = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.schema_version, LAYOUT_SCHEMA_VERSION);
        assert_eq!(parsed.templates.len(), layout.templates.len());
        assert_eq!(parsed.categories.len(), layout.categories.len());
    }

    #[test]
    fn test_import_adds_new_templates_and_categories() {
        let mut service = make_test_service();

        let layout = CountdownLayoutExport {
            schema_version: 1,
            description: "Test".into(),
            visual_defaults: CountdownCardVisuals::default(),
            display_mode: CountdownDisplayMode::CategoryContainers,
            templates: vec![CountdownCardTemplate {
                id: CountdownCardTemplateId(99),
                name: "Custom".into(),
                visuals: CountdownCardVisuals::default(),
                default_card_width: 150.0,
                default_card_height: 130.0,
            }],
            categories: vec![ExportedCategory {
                name: "Work".into(),
                display_order: 1,
                template_name: Some("Custom".into()),
                orientation: LayoutOrientation::Landscape,
                default_card_width: 150.0,
                default_card_height: 130.0,
            }],
        };

        let summary = service.import_layout(layout);

        assert_eq!(summary.templates_added, 1);
        assert_eq!(summary.categories_added, 1);
        // The imported template should have a new ID (not 99)
        let custom = service.templates.iter().find(|t| t.name == "Custom").unwrap();
        assert_ne!(custom.id.0, 99);
        // The new category should reference the custom template
        let work = service.categories.iter().find(|c| c.name == "Work").unwrap();
        assert_eq!(work.template_id, Some(custom.id));
    }

    #[test]
    fn test_import_updates_existing_by_name() {
        let mut service = make_test_service();
        let default_template_id = service.templates[0].id;

        let layout = CountdownLayoutExport {
            schema_version: 1,
            description: String::new(),
            visual_defaults: CountdownCardVisuals::default(),
            display_mode: CountdownDisplayMode::IndividualWindows,
            templates: vec![CountdownCardTemplate {
                id: CountdownCardTemplateId(DEFAULT_TEMPLATE_ID),
                name: "Default".into(),
                visuals: CountdownCardVisuals::default(),
                default_card_width: 200.0,
                default_card_height: 180.0,
            }],
            categories: vec![ExportedCategory {
                name: "General".into(),
                display_order: 5,
                template_name: Some("Default".into()),
                orientation: LayoutOrientation::Portrait,
                default_card_width: 200.0,
                default_card_height: 180.0,
            }],
        };

        let summary = service.import_layout(layout);

        assert_eq!(summary.templates_updated, 1);
        assert_eq!(summary.templates_added, 0);
        assert_eq!(summary.categories_updated, 1);
        assert_eq!(summary.categories_added, 0);

        // Template ID should be preserved
        let tmpl = service.templates.iter().find(|t| t.name == "Default").unwrap();
        assert_eq!(tmpl.id, default_template_id);
        assert_eq!(tmpl.default_card_width, 200.0);

        // Category should be updated
        let cat = service.categories.iter().find(|c| c.name == "General").unwrap();
        assert_eq!(cat.display_order, 5);
        assert_eq!(cat.orientation, LayoutOrientation::Portrait);
    }
}
