#![allow(dead_code)]

// UI models module
// Implementation pending - Phase 3

#[allow(dead_code)]
pub struct ViewConfig {
    pub current_view: ViewType,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewType {
    Day,
    WorkWeek,
    Week,
    #[default]
    Month,
    Quarter,
    Year,
    Agenda,
}
