//! Theme picker dialog for selecting calendar themes

use iced::{
    widget::{button, column, row, text},
    Element, Length,
};
use iced_aw::Card;

use crate::ui::messages::Message;

/// Create the theme picker dialog
pub fn create_theme_picker_dialog(
    available_themes: &[String],
    current_theme_name: &str,
) -> Element<'static, Message> {
    let theme_buttons: Vec<Element<Message>> = available_themes.iter()
        .map(|theme_name| {
            let is_current = theme_name == current_theme_name;
            let button_text = if is_current {
                format!("✓ {}", theme_name)
            } else {
                theme_name.clone()
            };
            
            button(text(button_text).size(14))
                .on_press(Message::SelectTheme(theme_name.clone()))
                .padding([10, 20])
                .width(Length::Fill)
                .into()
        })
        .collect();

    let cancel_button = button(text("Cancel").size(14))
        .on_press(Message::CloseThemePicker)
        .padding([10, 30]);

    // Custom header with close button
    let close_btn = button(text("X").size(20))
        .on_press(Message::CloseThemePicker)
        .padding(8);
    
    let header = row![
        text("Select Theme").size(20),
        text("").width(Length::Fill),
        close_btn
    ]
    .align_items(iced::Alignment::Center);

    Card::new(
        header,
        column(theme_buttons).spacing(10)
    )
    .foot(
        row![cancel_button]
            .spacing(10)
            .padding([0, 10, 10, 10])
    )
    .max_width(300.0)
    .into()
}
