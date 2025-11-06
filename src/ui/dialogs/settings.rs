//! Settings dialog for configuring application preferences

use iced::{
    widget::{button, checkbox, column, pick_list, row, text},
    Element, Length,
};
use iced_aw::Card;

use crate::ui::messages::Message;
use crate::ui::view_type::ViewType;

/// Create the settings dialog
pub fn create_settings_dialog<'a>(
    available_themes: &'a [String],
    current_theme_name: &'a str,
    current_view: ViewType,
    show_my_day: bool,
    my_day_position_right: bool,
    show_ribbon: bool,
    time_format: &'a str,
    first_day_of_week: u8,
    time_slot_interval: u32,
) -> Element<'a, Message> {
    // Theme setting - use available themes from database
    let theme_label = text("Theme:").size(14);
    let theme_picker = pick_list(
        available_themes.to_vec(),
        Some(current_theme_name.to_string()),
        Message::UpdateTheme
    );
    
    let manage_themes_button = button(text("Manage Themes...").size(12))
        .on_press(Message::OpenThemeManager)
        .padding([5, 10]);

    // View setting
    let view_label = text("Default View:").size(14);
    let current_view_str = match current_view {
        ViewType::Day => "Day",
        ViewType::WorkWeek => "Work Week",
        ViewType::Week => "Week",
        ViewType::Month => "Month",
        ViewType::Quarter => "Quarter",
    };
    let view_picker = pick_list(
        vec!["Day", "Work Week", "Week", "Month", "Quarter"],
        Some(current_view_str),
        |view| {
            let view_enum = match view {
                "Work Week" => "WorkWeek",
                _ => view,
            };
            Message::UpdateView(view_enum.to_string())
        }
    );

    // My Day panel checkbox
    let my_day_checkbox = checkbox("Show My Day Panel", show_my_day)
        .on_toggle(Message::UpdateShowMyDay);

    // My Day position setting
    let my_day_position_label = text("My Day Position:").size(14);
    let current_position = if my_day_position_right { "Right" } else { "Left" };
    let my_day_position_picker = pick_list(
        vec!["Left", "Right"],
        Some(current_position),
        |position| Message::UpdateMyDayPosition(position.to_string())
    );

    // Ribbon checkbox
    let ribbon_checkbox = checkbox("Show Multi-Day Ribbon", show_ribbon)
        .on_toggle(Message::UpdateShowRibbon);

    // Time format setting
    let time_format_label = text("Time Format:").size(14);
    let time_format_picker = pick_list(
        vec!["12h", "24h"],
        Some(time_format),
        |format| Message::UpdateTimeFormat(format.to_string())
    );

    // First day of week setting
    let first_day_label = text("First Day of Week:").size(14);
    let current_day_idx = first_day_of_week as usize;
    let day_names = vec!["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    let current_day_name = if current_day_idx < day_names.len() {
        day_names[current_day_idx]
    } else {
        "Sunday"
    };
    
    let first_day_picker = pick_list(
        day_names,
        Some(current_day_name),
        |selected| {
            let day_num = match selected {
                "Sunday" => "0",
                "Monday" => "1",
                "Tuesday" => "2",
                "Wednesday" => "3",
                "Thursday" => "4",
                "Friday" => "5",
                "Saturday" => "6",
                _ => "0",
            };
            Message::UpdateFirstDayOfWeek(day_num.to_string())
        }
    );

    let save_button = button(text("Save").size(14))
        .on_press(Message::SaveSettings)
        .padding([10, 30]);

    let cancel_button = button(text("Cancel").size(14))
        .on_press(Message::CloseSettings)
        .padding([10, 30]);

    // Custom header with close button
    let close_btn = button(text("×").size(24))
        .on_press(Message::CloseSettings)
        .padding(5);
    
    let header = row![
        text("Settings").size(24),
        text("").width(Length::Fill), // Spacer
        close_btn
    ]
    .align_items(iced::Alignment::Center);

    Card::new(
        header,
        column![
            text("Display Settings:").size(16),
            row![theme_label, theme_picker, manage_themes_button].spacing(10),
            row![view_label, view_picker].spacing(10),
            my_day_checkbox,
            row![my_day_position_label, my_day_position_picker].spacing(10),
            ribbon_checkbox,
            text("").size(15),
            text("General Settings:").size(16),
            row![time_format_label, time_format_picker].spacing(10),
            row![first_day_label, first_day_picker].spacing(10),
        ]
        .spacing(8)
    )
    .foot(
        row![save_button, cancel_button]
            .spacing(10)
            .padding([0, 10, 10, 10])
    )
    .max_width(600.0)
    .into()
}
