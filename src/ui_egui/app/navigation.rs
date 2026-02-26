use super::CalendarApp;
use crate::ui_egui::app::state::ViewType;
use chrono::{Datelike, Local, NaiveDate};

impl CalendarApp {
    pub(super) fn navigate_previous(&mut self) {
        self.current_date = match self.current_view {
            ViewType::Day => self.current_date - chrono::Duration::days(1),
            ViewType::Week | ViewType::WorkWeek => self.current_date - chrono::Duration::weeks(1),
            ViewType::Month => shift_month_preserving_day(self.current_date, -1),
        };
    }

    pub(super) fn navigate_next(&mut self) {
        self.current_date = match self.current_view {
            ViewType::Day => self.current_date + chrono::Duration::days(1),
            ViewType::Week | ViewType::WorkWeek => self.current_date + chrono::Duration::weeks(1),
            ViewType::Month => shift_month_preserving_day(self.current_date, 1),
        };
    }

    pub(super) fn navigate_up(&mut self) {
        self.current_date = match self.current_view {
            ViewType::Day => self.current_date - chrono::Duration::days(7),
            ViewType::Week | ViewType::WorkWeek => self.current_date - chrono::Duration::weeks(1),
            ViewType::Month => shift_month_preserving_day(self.current_date, -1),
        };
    }

    pub(super) fn navigate_down(&mut self) {
        self.current_date = match self.current_view {
            ViewType::Day => self.current_date + chrono::Duration::days(7),
            ViewType::Week | ViewType::WorkWeek => self.current_date + chrono::Duration::weeks(1),
            ViewType::Month => shift_month_preserving_day(self.current_date, 1),
        };
    }

    pub(super) fn jump_to_today(&mut self) {
        self.current_date = Local::now().date_naive();
    }
}

fn shift_month_preserving_day(current: NaiveDate, delta_months: i32) -> NaiveDate {
    let total_months = (current.year() * 12) + (current.month() as i32 - 1) + delta_months;
    let new_year = total_months.div_euclid(12);
    let new_month = total_months.rem_euclid(12) + 1;
    clamp_day(new_year, new_month as u32, current.day())
}

fn clamp_day(year: i32, month: u32, desired_day: u32) -> NaiveDate {
    let max_day = last_day_of_month(year, month);
    let day = desired_day.min(max_day);
    NaiveDate::from_ymd_opt(year, month, day)
        .or_else(|| NaiveDate::from_ymd_opt(year, month, max_day))
        .expect("valid calendar date")
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_of_next =
        NaiveDate::from_ymd_opt(next_year, next_month, 1).expect("valid next month");
    first_of_next.pred_opt().expect("previous day exists").day()
}
