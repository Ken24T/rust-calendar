use iced::widget::{button, column, container, pick_list, row, text, text_input, slider, scrollable};
use iced::{Color, Element, Length};
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
    is_selected: bool,
) -> Element<'a, Message> {
    let color_box = button(
            container(text(""))
                .width(Length::Fixed(30.0))
                .height(Length::Fixed(20.0))
                .style(move |_theme: &iced::Theme| {
                    container::Appearance {
                        background: Some(iced::Background::Color(color)),
                        border: iced::Border {
                            color: if is_selected { 
                                iced::Color::from_rgb(0.3, 0.6, 1.0) 
                            } else { 
                                iced::Color::from_rgb(0.5, 0.5, 0.5) 
                            },
                            width: if is_selected { 2.0 } else { 1.0 },
                            radius: 2.0.into(),
                        },
                        ..Default::default()
                    }
                })
        )
        .on_press(Message::OpenColorPicker(field_name.to_string()))
        .padding(0);

    row![
        text(label).size(13).width(Length::Fixed(160.0)),
        color_box,
        text(color_to_hex(color)).size(11),
    ]
    .spacing(6)
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
    show_color_picker: bool,
    color_picker_color: Color,
    color_picker_field: &'a str,
) -> Element<'a, Message> {
    let _iced_theme = &calendar_theme.base;
    
    // Use the theme being created/edited, or fall back to current theme
    let theme = creating_theme.unwrap_or(calendar_theme);
    
    let title_text = if is_editing {
        "Edit Custom Theme"
    } else {
        "Create Custom Theme"
    };

    // Left side - theme settings and color list
    let left_side = column![
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
        scrollable(
            column![
                color_field_row("App Background:", "app_background", theme.app_background, color_picker_field == "app_background"),
                color_field_row("Calendar Background:", "calendar_background", theme.calendar_background, color_picker_field == "calendar_background"),
                color_field_row("Weekend Background:", "weekend_background", theme.weekend_background, color_picker_field == "weekend_background"),
                color_field_row("Today Background:", "today_background", theme.today_background, color_picker_field == "today_background"),
                color_field_row("Today Border:", "today_border", theme.today_border, color_picker_field == "today_border"),
                color_field_row("Day Background:", "day_background", theme.day_background, color_picker_field == "day_background"),
                color_field_row("Day Border:", "day_border", theme.day_border, color_picker_field == "day_border"),
                color_field_row("Primary Text:", "text_primary", theme.text_primary, color_picker_field == "text_primary"),
                color_field_row("Secondary Text:", "text_secondary", theme.text_secondary, color_picker_field == "text_secondary"),
            ]
            .spacing(6)
        )
        .height(Length::Fixed(280.0)),
    ]
    .spacing(8)
    .width(Length::Fixed(310.0));

    // Right side - integrated color picker (only show if a color is selected)
    let right_side = if show_color_picker && !color_picker_field.is_empty() {
        let r = (color_picker_color.r * 255.0) as u8;
        let g = (color_picker_color.g * 255.0) as u8;
        let b = (color_picker_color.b * 255.0) as u8;

        // Format field name for display
        let display_name = color_picker_field.replace("_", " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        let field_r = color_picker_field.to_string();
        let field_g = color_picker_field.to_string();
        let field_b = color_picker_field.to_string();
        let field_hex = color_picker_field.to_string();
        let field_r_input = color_picker_field.to_string();
        let field_g_input = color_picker_field.to_string();
        let field_b_input = color_picker_field.to_string();

        column![
            text(display_name).size(14),
            // Color swatch display
            container(text(""))
                .width(Length::Fixed(120.0))
                .height(Length::Fixed(40.0))
                .style(move |_theme: &iced::Theme| {
                    container::Appearance {
                        background: Some(iced::Background::Color(color_picker_color)),
                        border: iced::Border {
                            color: Color::from_rgb(0.5, 0.5, 0.5),
                            width: 2.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                }),
            // Hex input
            row![
                text("Hex:").size(12).width(Length::Fixed(40.0)),
                text_input("#RRGGBB", &format!("#{:02X}{:02X}{:02X}", r, g, b))
                    .on_input(move |val| Message::UpdateHexInput(field_hex.clone(), val))
                    .padding(6)
                    .size(13)
                    .width(Length::Fixed(100.0)),
            ]
            .spacing(8)
            .align_items(iced::Alignment::Center),
            // RGB inputs
            row![
                text("R:").size(12).width(Length::Fixed(40.0)),
                text_input("0-255", &r.to_string())
                    .on_input(move |val| Message::UpdateRGBInput(field_r_input.clone(), "r".to_string(), val))
                    .padding(6)
                    .size(13)
                    .width(Length::Fixed(60.0)),
            ]
            .spacing(8)
            .align_items(iced::Alignment::Center),
            row![
                text("G:").size(12).width(Length::Fixed(40.0)),
                text_input("0-255", &g.to_string())
                    .on_input(move |val| Message::UpdateRGBInput(field_g_input.clone(), "g".to_string(), val))
                    .padding(6)
                    .size(13)
                    .width(Length::Fixed(60.0)),
            ]
            .spacing(8)
            .align_items(iced::Alignment::Center),
            row![
                text("B:").size(12).width(Length::Fixed(40.0)),
                text_input("0-255", &b.to_string())
                    .on_input(move |val| Message::UpdateRGBInput(field_b_input.clone(), "b".to_string(), val))
                    .padding(6)
                    .size(13)
                    .width(Length::Fixed(60.0)),
            ]
            .spacing(8)
            .align_items(iced::Alignment::Center),
            // RGB Sliders for fine-tuning
            column![
                slider(0..=255, r, move |val| {
                    Message::UpdateColorSlider(field_r.clone(), "r".to_string(), val)
                }).width(Length::Fill),
                slider(0..=255, g, move |val| {
                    Message::UpdateColorSlider(field_g.clone(), "g".to_string(), val)
                }).width(Length::Fill),
                slider(0..=255, b, move |val| {
                    Message::UpdateColorSlider(field_b.clone(), "b".to_string(), val)
                }).width(Length::Fill),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .padding(10)
        .width(Length::Fixed(240.0))
    } else {
        column![
            text("← Select a color").size(12),
        ]
        .spacing(10)
        .padding(10)
        .width(Length::Fixed(240.0))
    };

    let main_content = row![
        left_side,
        container(row![])
            .width(Length::Fixed(1.0))
            .height(Length::Fill)
            .style(|_theme: &iced::Theme| {
                container::Appearance {
                    background: Some(iced::Background::Color(Color::from_rgb(0.5, 0.5, 0.5))),
                    ..Default::default()
                }
            }),
        right_side,
    ]
    .spacing(10);

    let content = column![
        main_content,
        // Action buttons at bottom
        row![
            button(text("Cancel").size(14))
                .on_press(Message::CancelCreateTheme)
                .padding([8, 25]),
            button(text(if is_editing { "Save Changes" } else { "Save Theme" }).size(14))
                .on_press(Message::SaveCustomTheme)
                .padding([8, 25]),
        ]
        .spacing(10)
        .padding([10, 0, 0, 0]),
    ]
    .spacing(8)
    .padding(12);

    let card = Card::new(text(title_text).size(16), content)
        .width(Length::Fixed(600.0))
        .max_height(600.0);

    Modal::new(container(text("")), Some(card))
        .backdrop(Message::CancelCreateTheme)
        .into()
}
