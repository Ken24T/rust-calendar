use iced::widget::{column, container, row};
use iced::{Element, Length, Theme};
use iced_aw::Modal;

use crate::ui::theme::CalendarTheme;
use crate::ui::messages::Message;
use crate::ui::view_type::ViewType;
use crate::ui::{components, dialogs, helpers};

/// Build the main application view with all UI components
pub fn build_view<'a>(
    current_view: ViewType,
    show_my_day: bool,
    show_ribbon: bool,
    my_day_position_right: bool,
    theme: &Theme,
    calendar_theme: &CalendarTheme,
    calendar_view: Element<'a, Message>,
    show_settings_dialog: bool,
    show_theme_manager: bool,
    show_create_theme: bool,
    show_date_picker: bool,
    show_theme_picker: bool,
    available_themes: &'a [String],
    theme_name: &'a str,
    creating_theme_name: &'a str,
    creating_base_theme: &'a str,
    creating_theme: Option<&CalendarTheme>,
    current_date_year: i32,
    current_date_month: u32,
    time_format: &'a str,
    first_day_of_week: u8,
    time_slot_interval: u32,
) -> Element<'a, Message> {
    // Main layout structure
    let mut content = column![].spacing(0);

    // Top menu bar
    let menu_bar = components::create_menu_bar(
        current_view,
        show_my_day,
        show_ribbon,
        theme,
    );
    content = content.push(menu_bar);

    // Multi-day ribbon (if visible)
    if show_ribbon {
        let ribbon = helpers::create_ribbon();
        content = content.push(ribbon);
    }

    // Main content area: My Day panel + Calendar view
    let main_content = if show_my_day {
        if my_day_position_right {
            row![
                calendar_view,
                helpers::create_my_day_panel(),
            ]
            .spacing(2)
        } else {
            row![
                helpers::create_my_day_panel(),
                calendar_view,
            ]
            .spacing(2)
        }
    } else {
        row![calendar_view]
    };
    
    content = content.push(main_content);

    let app_bg = calendar_theme.app_background;
    let base_view = container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme: &Theme| {
            container::Appearance {
                background: Some(iced::Background::Color(app_bg)),
                ..Default::default()
            }
        });

    // Show modal dialog if settings or date picker is open
    if show_settings_dialog {
        Modal::new(base_view, Some(dialogs::create_settings_dialog(
            available_themes,
            theme_name,
            current_view,
            show_my_day,
            my_day_position_right,
            show_ribbon,
            time_format,
            first_day_of_week,
            time_slot_interval,
        )))
            .backdrop(Message::CloseSettings)
            .into()
    } else if show_theme_manager {
        Modal::new(base_view, Some(dialogs::create_theme_manager_dialog(available_themes, theme_name)))
            .backdrop(Message::CloseThemeManager)
            .into()
    } else if show_create_theme {
        dialogs::theme_creator::view(
            creating_theme_name,
            available_themes,
            creating_base_theme,
            creating_theme,
            calendar_theme,
        )
    } else if show_date_picker {
        Modal::new(base_view, Some(dialogs::create_date_picker_dialog(
            current_date_year,
            current_date_month
        )))
            .backdrop(Message::ToggleDatePicker)
            .into()
    } else if show_theme_picker {
        Modal::new(base_view, Some(dialogs::create_theme_picker_dialog(available_themes, theme_name)))
            .backdrop(Message::CloseThemePicker)
            .into()
    } else {
        base_view.into()
    }
}
