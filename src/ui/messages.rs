// UI Messages
// All application messages for the iced event system

use super::view_type::ViewType;

/// Messages for the application
#[derive(Debug, Clone)]
pub enum Message {
    /// Toggle between light and dark theme (or show picker if 3+ themes)
    ToggleTheme,
    /// Show theme picker
    ShowThemePicker,
    /// Close theme picker
    CloseThemePicker,
    /// Select a theme from picker
    SelectTheme(String),
    /// Toggle My Day panel visibility
    ToggleMyDay,
    /// Toggle multi-day ribbon visibility
    ToggleRibbon,
    /// Switch to a different view
    SwitchView(ViewType),
    /// Open settings dialog
    OpenSettings,
    /// Close settings dialog
    CloseSettings,
    /// Update theme setting from dialog
    UpdateTheme(String),
    /// Update view setting from dialog
    UpdateView(String),
    /// Update My Day panel visibility from dialog
    UpdateShowMyDay(bool),
    /// Update My Day panel position from dialog
    UpdateMyDayPosition(String),
    /// Update Ribbon visibility from dialog
    UpdateShowRibbon(bool),
    /// Update time format setting
    UpdateTimeFormat(String),
    /// Update first day of week setting
    UpdateFirstDayOfWeek(String),
    /// Save settings from dialog
    SaveSettings,
    /// Open theme manager
    OpenThemeManager,
    /// Close theme manager
    CloseThemeManager,
    /// Delete a custom theme
    DeleteTheme(String),
    /// Exit the application
    Exit,
    /// Navigate to previous month
    PreviousMonth,
    /// Navigate to next month
    NextMonth,
    /// Navigate to previous day
    PreviousDay,
    /// Navigate to next day
    NextDay,
    /// Navigate to previous week
    PreviousWeek,
    /// Navigate to next week
    NextWeek,
    /// Navigate to previous quarter
    PreviousQuarter,
    /// Navigate to next quarter
    NextQuarter,
    /// Navigate to today
    GoToToday,
    /// Toggle date picker visibility
    ToggleDatePicker,
    /// Change to specific month
    ChangeMonth(u32),
    /// Change to specific year
    ChangeYear(i32),
    /// Navigate to specific date and switch to Week view
    GoToDateInWeekView(i32, u32, u32), // year, month, day
    /// Update time slot interval (15, 30, 45, or 60 minutes)
    UpdateTimeSlotInterval(u32),
}
