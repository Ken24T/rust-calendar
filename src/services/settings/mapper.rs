use crate::models::settings::Settings;
use anyhow::Result;
use rusqlite::Row;

pub fn row_to_settings(row: &Row) -> Result<Settings, rusqlite::Error> {
    Ok(Settings {
        id: Some(row.get(0)?),
        theme: row.get(1)?,
        first_day_of_week: row.get(2)?,
        time_format: row.get(3)?,
        date_format: row.get(4)?,
        show_my_day: row.get::<_, i32>(5)? != 0,
        my_day_position_right: row.get::<_, i32>(6)? != 0,
        show_ribbon: row.get::<_, i32>(7)? != 0,
        current_view: row.get(8)?,
        default_event_duration: row.get(9)?,
        first_day_of_work_week: row.get(10)?,
        last_day_of_work_week: row.get(11)?,
        default_event_start_time: row.get(12)?,
    })
}
