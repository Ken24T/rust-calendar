use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Element, Length};
use iced_aw::{Card, Modal};

use crate::ui::messages::Message;
use crate::ui::theme::CalendarTheme;

/// Helper to convert Color to hex string
fn color_to_hex(color: iced::Color) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8
    )
}

/// Helper to create a color picker button with label
fn color_field_row<'a>(
    label: &'a str,
    field_name: &'a str,
    color: iced::Color,
) -> Element<'a, Message> {
    row![
        text(label).size(13).width(Length::Fixed(180.0)),
        // Color preview box
        container(text(""))
            .width(Length::Fixed(30.0))
            .height(Length::Fixed(20.0))
            .style(move |_theme: &iced::Theme| {
                container::Appearance {
                    background: Some(iced::Background::Color(color)),
                    border: iced::Border {
                        color: iced::Color::from_rgb(0.5, 0.5, 0.5),
                        width: 1.0,
                        radius: 2.0.into(),
                    },
                    ..Default::default()
                }
            }),
        text(color_to_hex(color)).size(12),
        button(text("Pick").size(12))
            .on_press(Message::UpdateThemeColor(
                field_name.to_string(),
                color_to_hex(color)
            ))
            .padding([3, 8]),
    ]
    .spacing(8)
    .align_items(iced::Alignment::Center)
    .into()
}

/// Creates the theme creation/edit dialog
pub fn view<'a>(
    theme_name: &'a str,
    available_themes: &'a [String],
    selected_base: &'a str,
    creating_theme: Option<&CalendarTheme>,
    calendar_theme: &CalendarTheme,
    is_editing: bool,
) -> Element<'a, Message> {
    let _iced_theme = &calendar_theme.base;
    
    // Use the theme being created/edited, or fall back to current theme
    let theme = creating_theme.unwrap_or(calendar_theme);
    
    let title_text = if is_editing {
        "Edit Custom Theme"
    } else {
        "Create Custom Theme"
    };

    let content = column![
        // Theme name input
        text("Theme Name:").size(14),
        text_input("Enter theme name...", theme_name)
            .on_input(Message::UpdateThemeName)
            .padding(8),
        // Base theme selector (only for new themes)
        if !is_editing {
            column![
                text("Base on Existing Theme:").size(14),
                pick_list(
                    available_themes,
                    Some(selected_base.to_string()),
                    Message::SelectBaseTheme
                )
                .padding(8),
            ]
            .spacing(5)
        } else {
            column![]
        },
        text("Theme Colors:").size(14),
        // Scrollable color picker section
        column![
            color_field_row("App Background:", "app_background", theme.app_background),
            color_field_row("Calendar Background:", "calendar_background", theme.calendar_background),
            color_field_row("Weekend Background:", "weekend_background", theme.weekend_background),
            color_field_row("Today Background:", "today_background", theme.today_background),
            color_field_row("Today Border:", "today_border", theme.today_border),
            color_field_row("Day Background:", "day_background", theme.day_background),
            color_field_row("Day Border:", "day_border", theme.day_border),
            color_field_row("Primary Text:", "text_primary", theme.text_primary),
            color_field_row("Secondary Text:", "text_secondary", theme.text_secondary),
        ]
        .spacing(8),
        // Action buttons
        row![
            button(text("Cancel").size(14))
                .on_press(Message::CancelCreateTheme)
                .padding(8),
            button(text(if is_editing { "Save Changes" } else { "Save Theme" }).size(14))
                .on_press(Message::SaveCustomTheme)
                .padding(8),
        ]
        .spacing(10)
        .padding([10, 0, 0, 0]),
    ]
    .spacing(10)
    .padding(10);

    let card = Card::new(text(title_text).size(18), content)
        .width(Length::Fixed(500.0))
        .on_close(Message::CancelCreateTheme);

    Modal::new(container(text("")), Some(card))
        .backdrop(Message::CancelCreateTheme)
        .into()
}
