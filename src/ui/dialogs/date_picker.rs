//! Date picker dialog for selecting month and year

use iced::{
    widget::{button, column, pick_list, row, text},
    Element, Length,
};
use iced_aw::Card;

use crate::ui::messages::Message;

/// Month names for the picker
const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December"
];

/// Create the date picker dialog
pub fn create_date_picker_dialog(
    current_year: i32,
    current_month: u32,
) -> Element<'static, Message> {
    // Year selector - show current year +/- 5 years
    let year_label = text("Year:").size(14);
    let years: Vec<i32> = ((current_year - 5)..=(current_year + 5)).collect();
    let year_picker = pick_list(
        years,
        Some(current_year),
        Message::ChangeYear
    );
    
    // Month selector
    let month_label = text("Month:").size(14);
    
    let current_month_name = MONTH_NAMES.get((current_month - 1) as usize)
        .unwrap_or(&"January");
    
    let month_picker = pick_list(
        MONTH_NAMES.to_vec(),
        Some(*current_month_name),
        |selected| {
            let month_num = match selected {
                "January" => 1,
                "February" => 2,
                "March" => 3,
                "April" => 4,
                "May" => 5,
                "June" => 6,
                "July" => 7,
                "August" => 8,
                "September" => 9,
                "October" => 10,
                "November" => 11,
                "December" => 12,
                _ => 1,
            };
            Message::ChangeMonth(month_num)
        }
    );
    
    let cancel_button = button(text("Cancel").size(14))
        .on_press(Message::ToggleDatePicker)
        .padding([10, 30]);

    // Custom header with close button
    let close_btn = button(text("X").size(20))
        .on_press(Message::ToggleDatePicker)
        .padding(8);
    
    let header = row![
        text("Select Date").size(20),
        text("").width(Length::Fill),
        close_btn
    ]
    .align_items(iced::Alignment::Center);

    Card::new(
        header,
        column![
            row![year_label, year_picker].spacing(10),
            row![month_label, month_picker].spacing(10),
        ]
        .spacing(15)
    )
    .foot(
        row![cancel_button]
            .spacing(10)
            .padding([0, 10, 10, 10])
    )
    .max_width(350.0)
    .into()
}
