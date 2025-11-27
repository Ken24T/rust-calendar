use crate::models::settings::Settings;
use anyhow::Result;
use rusqlite::Row;

pub fn row_to_settings(row: &Row) -> Result<Settings, rusqlite::Error> {
    Ok(Settings {
        id: Some(row.get(0)?),
        theme: row.get(1)?,
        use_system_theme: row.get::<_, i32>(2).unwrap_or(0) != 0,
        first_day_of_week: row.get(3)?,
        time_format: row.get(4)?,
        date_format: row.get(5)?,
        show_my_day: row.get::<_, i32>(6)? != 0,
        my_day_position_right: row.get::<_, i32>(7)? != 0,
        show_ribbon: row.get::<_, i32>(8)? != 0,
        show_sidebar: row.get::<_, i32>(9).unwrap_or(1) != 0,
        show_week_numbers: row.get::<_, i32>(10).unwrap_or(0) != 0,
        current_view: row.get(11)?,
        default_event_duration: row.get(12)?,
        first_day_of_work_week: row.get(13)?,
        last_day_of_work_week: row.get(14)?,
        default_event_start_time: row.get(15)?,
        default_card_width: row.get(16)?,
        default_card_height: row.get(17)?,
        auto_create_countdown_on_import: row.get::<_, i32>(18).unwrap_or(0) != 0,
        edit_before_import: row.get::<_, i32>(19).unwrap_or(0) != 0,
    })
}
