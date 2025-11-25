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
        show_sidebar: row.get::<_, i32>(8).unwrap_or(1) != 0,
        current_view: row.get(9)?,
        default_event_duration: row.get(10)?,
        first_day_of_work_week: row.get(11)?,
        last_day_of_work_week: row.get(12)?,
        default_event_start_time: row.get(13)?,
        default_card_width: row.get(14)?,
        default_card_height: row.get(15)?,
        auto_create_countdown_on_import: row.get::<_, i32>(16).unwrap_or(0) != 0,
        edit_before_import: row.get::<_, i32>(17).unwrap_or(0) != 0,
    })
}
