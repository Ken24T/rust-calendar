use iced::widget::{button, row, text};
use iced::{Element, Length, Theme};
use iced_aw::menu::{Item, Menu, MenuBar};

use crate::ui::messages::Message;
use crate::ui::view_type::ViewType;

/// Create the top menu bar with standard Windows menu structure
pub fn create_menu_bar(
    current_view: ViewType,
    show_my_day: bool,
    show_ribbon: bool,
    theme: &Theme,
) -> Element<'static, Message> {
    let file_menu = Item::with_menu(
        button(text("File").size(14)).padding([5, 10]),
        Menu::new(vec![
            Item::new(
                button(text("Exit").size(13))
                    .on_press(Message::Exit)
                    .padding([8, 20])
                    .width(Length::Fill)
            ),
        ])
        .max_width(180.0)
        .offset(0.0)
        .spacing(0.0)
    );

    let edit_menu = Item::with_menu(
        button(text("Edit").size(14)).padding([5, 10]),
        Menu::new(vec![
            Item::new(
                button(text("Settings").size(13))
                    .on_press(Message::OpenSettings)
                    .padding([8, 20])
                    .width(Length::Fill)
            ),
        ])
        .max_width(180.0)
        .offset(0.0)
        .spacing(0.0)
    );

    // Views submenu
    let views_submenu = Menu::new(vec![
        Item::new(
            button(
                text(if current_view == ViewType::Day { "✓ Day" } else { "  Day" }).size(13)
            )
            .on_press(Message::SwitchView(ViewType::Day))
            .padding([8, 20])
            .width(Length::Fill)
        ),
        Item::new(
            button(
                text(if current_view == ViewType::WorkWeek { "✓ Work Week" } else { "  Work Week" }).size(13)
            )
            .on_press(Message::SwitchView(ViewType::WorkWeek))
            .padding([8, 20])
            .width(Length::Fill)
        ),
        Item::new(
            button(
                text(if current_view == ViewType::Week { "✓ Week" } else { "  Week" }).size(13)
            )
            .on_press(Message::SwitchView(ViewType::Week))
            .padding([8, 20])
            .width(Length::Fill)
        ),
        Item::new(
            button(
                text(if current_view == ViewType::Month { "✓ Month" } else { "  Month" }).size(13)
            )
            .on_press(Message::SwitchView(ViewType::Month))
            .padding([8, 20])
            .width(Length::Fill)
        ),
        Item::new(
            button(
                text(if current_view == ViewType::Quarter { "✓ Quarter" } else { "  Quarter" }).size(13)
            )
            .on_press(Message::SwitchView(ViewType::Quarter))
            .padding([8, 20])
            .width(Length::Fill)
        ),
    ])
    .max_width(180.0)
    .offset(0.0)
    .spacing(0.0);

    let view_menu = Item::with_menu(
        button(text("View").size(14)).padding([5, 10]),
        Menu::new(vec![
            Item::new(
                button(
                    text(if show_my_day { "✓ My Day" } else { "  My Day" }).size(13)
                )
                .on_press(Message::ToggleMyDay)
                .padding([8, 20])
                .width(Length::Fill)
            ),
            Item::new(
                button(
                    text(if show_ribbon { "✓ Ribbon" } else { "  Ribbon" }).size(13)
                )
                .on_press(Message::ToggleRibbon)
                .padding([8, 20])
                .width(Length::Fill)
            ),
            Item::with_menu(
                button(text("Views ▶").size(13))
                    .padding([8, 20])
                    .width(Length::Fill),
                views_submenu
            ),
        ])
        .max_width(180.0)
        .offset(0.0)
        .spacing(0.0)
    );

    let menu_bar = MenuBar::new(vec![file_menu, edit_menu, view_menu]);

    let toolbar = row![
        menu_bar,
        // Spacer
        text("").width(Length::Fill),
        // Theme toggle with fixed width
        button(text(if matches!(theme, Theme::Light) { "🌙" } else { "☀️" }).size(16))
            .on_press(Message::ToggleTheme)
            .padding(8)
            .width(45),
    ]
    .padding(5)
    .spacing(5);

    toolbar.into()
}
