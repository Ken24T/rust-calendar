//! Theme manager dialog for managing custom themes

use iced::{
    widget::{button, column, row, text},
    Element, Length,
};
use iced_aw::Card;

use crate::ui::messages::Message;

/// Create the theme manager dialog
pub fn create_theme_manager_dialog(
    available_themes: &[String],
    current_theme_name: &str,
) -> Element<'static, Message> {
    let mut theme_list: Vec<Element<Message>> = vec![
        text("Available Themes:").size(16).into(),
    ];
    
    // List all themes with delete buttons for custom themes
    for theme_name in available_themes {
        let is_builtin = theme_name == "Light" || theme_name == "Dark";
        let is_current = theme_name == current_theme_name;
        
        let theme_text = if is_current {
            format!("✓ {}", theme_name)
        } else {
            theme_name.clone()
        };
        
        if is_builtin {
            // Built-in themes - just show the name
            theme_list.push(
                row![
                    text(theme_text).size(14).width(Length::Fill),
                    text("(Built-in)").size(12),
                ]
                .spacing(10)
                .padding(5)
                .into()
            );
        } else {
            // Custom themes - show with delete button
            let delete_button = button(text("Delete").size(12))
                .on_press(Message::DeleteTheme(theme_name.clone()))
                .padding([5, 10]);
            
            theme_list.push(
                row![
                    text(theme_text).size(14).width(Length::Fill),
                    delete_button,
                ]
                .spacing(10)
                .padding(5)
                .into()
            );
        }
    }
    
    let close_button = button(text("Close").size(14))
        .on_press(Message::CloseThemeManager)
        .padding([10, 30]);

    // Custom header with close button
    let close_btn = button(text("×").size(24))
        .on_press(Message::CloseThemeManager)
        .padding(5);
    
    let header = row![
        text("Manage Themes").size(20),
        text("").width(Length::Fill), // Spacer
        close_btn
    ]
    .align_items(iced::Alignment::Center);

    Card::new(
        header,
        column(theme_list).spacing(5)
    )
    .foot(
        column![
            text("Note: Create custom themes via the database for now.").size(11),
            row![close_button]
                .spacing(10)
                .padding([10, 10, 10, 10])
        ]
    )
    .max_width(400.0)
    .into()
}
