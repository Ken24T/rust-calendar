//! Custom color picker dialog with text-based buttons

use iced::{
    widget::{button, column, container, row, slider, text},
    Color, Element, Length,
};
use iced_aw::Card;

use crate::ui::messages::Message;

/// Create a custom color picker dialog
pub fn create_color_picker_dialog(
    current_color: Color,
    field_name: &str,
) -> Element<Message> {
    let r = (current_color.r * 255.0) as u8;
    let g = (current_color.g * 255.0) as u8;
    let b = (current_color.b * 255.0) as u8;
    
    let field_name_owned = field_name.to_string();

    // Format field name for display
    let display_name = field_name.replace("_", " ")
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

    // Color preview
    let preview = container(text(""))
        .width(Length::Fixed(60.0))
        .height(Length::Fixed(60.0))
        .style(move |_theme: &iced::Theme| {
            container::Appearance {
                background: Some(iced::Background::Color(current_color)),
                border: iced::Border {
                    color: Color::from_rgb(0.5, 0.5, 0.5),
                    width: 2.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        });

    let hex_text = format!("#{:02X}{:02X}{:02X}", r, g, b);

    let field_name_r = field_name_owned.clone();
    let field_name_g = field_name_owned.clone();
    let field_name_b = field_name_owned.clone();

    let content = column![
        text(format!("Editing: {}", display_name)).size(14),
        row![
            preview,
            column![
                text(hex_text).size(16),
                text(format!("RGB({}, {}, {})", r, g, b)).size(12),
            ]
            .spacing(5)
        ]
        .spacing(15)
        .align_items(iced::Alignment::Center),
        row![
            text("R:").size(14).width(Length::Fixed(25.0)),
            slider(0..=255, r, move |val| {
                Message::UpdateColorSlider(field_name_r.clone(), "r".to_string(), val)
            }).width(Length::Fixed(200.0)),
            text(format!("{:3}", r)).size(14).width(Length::Fixed(35.0)),
        ]
        .spacing(10)
        .align_items(iced::Alignment::Center),
        row![
            text("G:").size(14).width(Length::Fixed(25.0)),
            slider(0..=255, g, move |val| {
                Message::UpdateColorSlider(field_name_g.clone(), "g".to_string(), val)
            }).width(Length::Fixed(200.0)),
            text(format!("{:3}", g)).size(14).width(Length::Fixed(35.0)),
        ]
        .spacing(10)
        .align_items(iced::Alignment::Center),
        row![
            text("B:").size(14).width(Length::Fixed(25.0)),
            slider(0..=255, b, move |val| {
                Message::UpdateColorSlider(field_name_b.clone(), "b".to_string(), val)
            }).width(Length::Fixed(200.0)),
            text(format!("{:3}", b)).size(14).width(Length::Fixed(35.0)),
        ]
        .spacing(10)
        .align_items(iced::Alignment::Center),
    ]
    .spacing(15)
    .padding(20)
    .align_items(iced::Alignment::Center);

    let cancel_button = button(text("Cancel").size(14))
        .on_press(Message::CancelColorPicker)
        .padding([8, 25]);

    let ok_button = button(text("OK").size(14))
        .on_press(Message::SubmitColor(Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)))
        .padding([8, 25]);

    Card::new(text("Color Picker").size(16), content)
        .foot(
            row![cancel_button, ok_button]
                .spacing(10)
                .padding([10, 10, 10, 10]),
        )
        .width(Length::Fixed(350.0))
        .into()
}
