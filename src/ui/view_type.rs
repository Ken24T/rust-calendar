// View Types
// Different calendar view modes

/// Calendar view types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    Day,
    WorkWeek,
    Week,
    Month,
    Quarter,
}
