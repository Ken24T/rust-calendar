//! UI helper functions for creating common UI components

use iced::{
    widget::{column, container, text},
    Element, Length,
};

use crate::ui::messages::Message;

/// Create a placeholder ribbon for multi-day events
pub fn create_ribbon() -> Element<'static, Message> {
    container(
        text("Multi-Day Event Ribbon (Coming Soon)")
            .size(14)
    )
    .padding(10)
    .width(Length::Fill)
    .into()
}

/// Create the My Day panel showing today's date and events
pub fn create_my_day_panel() -> Element<'static, Message> {
    container(
        column![
            text("My Day").size(18),
            text("Thu, Nov 6, 2025").size(14),
            text(""),
            text("No events today").size(12),
        ]
        .spacing(10)
    )
    .padding(15)
    .width(250)
    .height(Length::Fill)
    .into()
}

/// Create a placeholder view for unimplemented views
pub fn create_placeholder_view(title: &str) -> Element<'static, Message> {
    container(
        column![
            text(title).size(24),
            text(""),
            text("This view is under development").size(14),
        ]
        .spacing(20)
    )
    .padding(20)
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x()
    .center_y()
    .into()
}
