#![allow(dead_code)]

// UI models module
// Implementation pending - Phase 3

pub struct ViewConfig {
    pub current_view: ViewType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    Day,
    WorkWeek,
    Week,
    Month,
    Quarter,
    Year,
    Agenda,
}

impl Default for ViewType {
    fn default() -> Self {
        ViewType::Month
    }
}
