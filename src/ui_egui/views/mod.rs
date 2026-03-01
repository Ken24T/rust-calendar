use chrono::{DateTime, Local, NaiveDate, NaiveTime, Timelike};

use crate::models::event::Event;
use crate::services::countdown::CountdownCategoryId;

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
