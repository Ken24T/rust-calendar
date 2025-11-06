// Settings module
// Implementation pending - Phase 1

pub struct Settings {
    pub id: Option<i64>,
    pub theme: String,
    pub first_day_of_week: u8,
    pub time_format: String,
    pub date_format: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: Some(1),
            theme: "light".to_string(),
            first_day_of_week: 0, // Sunday
            time_format: "12h".to_string(),
            date_format: "MM/DD/YYYY".to_string(),
        }
    }
}
