// Date utility functions
// Implementation pending - Phase 1

use chrono::{DateTime, Local};

pub fn is_same_day(date1: DateTime<Local>, date2: DateTime<Local>) -> bool {
    date1.date_naive() == date2.date_naive()
}

pub fn start_of_day(date: DateTime<Local>) -> DateTime<Local> {
    date.date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(date.timezone())
        .unwrap()
}

pub fn end_of_day(date: DateTime<Local>) -> DateTime<Local> {
    date.date_naive()
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_local_timezone(date.timezone())
        .unwrap()
}
