// Recurrence module
// Implementation pending - Phase 2

pub struct RecurrenceRule {
    pub id: Option<i64>,
    pub frequency: Frequency,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<chrono::DateTime<chrono::Local>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frequency {
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
    Quarterly,
    Yearly,
}
