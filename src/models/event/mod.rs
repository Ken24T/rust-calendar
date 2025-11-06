// Event module
// Implementation pending - Phase 2

pub struct Event {
    pub id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: chrono::DateTime<chrono::Local>,
    pub end: chrono::DateTime<chrono::Local>,
    pub all_day: bool,
    pub color: Option<String>,
    pub recurrence_id: Option<i64>,
}

impl Event {
    pub fn new(title: String, start: chrono::DateTime<chrono::Local>, end: chrono::DateTime<chrono::Local>) -> Self {
        Self {
            id: None,
            title,
            description: None,
            location: None,
            start,
            end,
            all_day: false,
            color: None,
            recurrence_id: None,
        }
    }
}
