use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Element, Length};
use iced_aw::{Card, Modal};

use crate::ui::messages::Message;
use crate::ui::theme::CalendarTheme;

/// Creates the theme creation dialog
pub fn view<'a>(
    theme_name: &'a str,
    available_themes: &'a [String],
    selected_base: &'a str,
    _creating_theme: Option<&CalendarTheme>,
    calendar_theme: &CalendarTheme,
) -> Element<'a, Message> {
    let _iced_theme = &calendar_theme.base;

    let content = column![
        // Theme name input
        text("Theme Name:").size(14),
        text_input("Enter theme name...", theme_name)
            .on_input(Message::UpdateThemeName)
            .padding(8),
        // Base theme selector
        text("Base on Existing Theme:").size(14),
        pick_list(
            available_themes,
            Some(selected_base.to_string()),
            Message::SelectBaseTheme
        )
        .padding(8),
        // TODO: Add color pickers for each theme color
        // Will implement in next iteration
        text("(Color customization coming in next step)").size(12),
        // Action buttons
        row![
            button(text("Cancel").size(14))
                .on_press(Message::CancelCreateTheme)
                .padding(8),
            button(text("Save Theme").size(14))
                .on_press(Message::SaveCustomTheme)
                .padding(8),
        ]
        .spacing(10)
        .padding([10, 0, 0, 0]),
    ]
    .spacing(10)
    .padding(10);

    let card = Card::new(text("Create Custom Theme").size(18), content)
        .width(Length::Fixed(400.0))
        .on_close(Message::CancelCreateTheme);

    Modal::new(container(text("")), Some(card))
        .backdrop(Message::CancelCreateTheme)
        .into()
}
