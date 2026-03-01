use chrono::{DateTime, Local, NaiveDate, NaiveTime, Timelike};

use crate::models::event::Event;
use crate::services::countdown::CountdownCategoryId;

/// Per-frame cache of countdown categories stored in egui's temp data.
///
/// Written by `render_main_panel` each frame so context menus can read
/// it without threading the list through every function layer.
#[derive(Clone, Default)]
pub struct CountdownCategoriesCache(pub Vec<(CountdownCategoryId, String)>);

/// Unique ID used for `CountdownCategoriesCache` in egui's `IdTypeMap`.
const COUNTDOWN_CATEGORIES_CACHE_ID: &str = "countdown_categories_cache";

/// Render the "Create Countdown" context-menu item(s).
///
/// - If only one category exists, shows a single button.
/// - If multiple categories exist, shows a submenu to pick the target container.
pub fn render_countdown_menu_items(
    ui: &mut egui::Ui,
    event: &Event,
    countdown_requests: &mut Vec<CountdownRequest>,
) {
    let categories = ui.ctx().data(|data| {
        data.get_temp::<CountdownCategoriesCache>(egui::Id::new(COUNTDOWN_CATEGORIES_CACHE_ID))
    })
    .map(|c| c.0)
    .unwrap_or_default();

    if categories.len() <= 1 {
        // Single category — simple button
        if ui.button("⏱ Create Countdown").clicked() {
            countdown_requests.push(CountdownRequest::from_event(event));
            ui.close_menu();
        }
    } else {
        // Multiple categories — show submenu
        ui.menu_button("⏱ Create Countdown", |ui| {
            for (cat_id, cat_name) in &categories {
                if ui.button(cat_name).clicked() {
                    let mut req = CountdownRequest::from_event(event);
                    req.category_id = Some(*cat_id);
                    countdown_requests.push(req);
                    ui.close_menu();
                }
            }
        });
    }
}

mod day_context_menu;
pub mod day_view;
mod day_event_rendering;
mod day_time_slot;
mod event_helpers;
mod event_rendering;
mod month_context_menu;
mod month_day_cell;
pub mod month_view;
mod palette;
pub mod quarter_view;
mod time_grid;
mod time_grid_cell;
mod time_grid_context_menu;
pub mod week_shared;
pub mod week_view;
pub mod workweek_view;

pub use event_helpers::*;

#[derive(Clone, Debug)]
pub struct CountdownRequest {
    pub event_id: Option<i64>,
    pub title: String,
    pub start_at: DateTime<Local>,
    pub end_at: DateTime<Local>,
    pub color: Option<String>,
    pub body: Option<String>,
    #[allow(dead_code)]
    pub display_label: Option<String>,
    /// Target category for the new card (None = default/General).
    pub category_id: Option<CountdownCategoryId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CountdownMenuState {
    Hidden,
    Active,
    Available,
}

#[derive(Clone, Copy, Debug)]
pub struct AutoFocusRequest {
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
}

impl AutoFocusRequest {
    pub fn from_event(event: &Event) -> Self {
        Self {
            date: event.start.date_naive(),
            time: (!event.all_day).then(|| event.start.time()),
        }
    }

    pub fn matches_slot(
        &self,
        date: NaiveDate,
        slot_start: NaiveTime,
        slot_end: NaiveTime,
    ) -> bool {
        if self.date != date {
            return false;
        }

        let target_time = self
            .time
            .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());

        let target_secs = target_time.num_seconds_from_midnight();
        let slot_start_secs = slot_start.num_seconds_from_midnight();
        let slot_end_secs = slot_end.num_seconds_from_midnight();

        // slot_end for final slot can be 23:59:59, so treat it as inclusive.
        if slot_start_secs <= target_secs && target_secs < slot_end_secs {
            return true;
        }

        slot_end_secs == target_secs
    }
}
