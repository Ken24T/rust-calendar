use chrono::{Datelike, Local, NaiveDate, Timelike};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Border, Element, Length, Theme};
use iced::alignment::{Horizontal, Vertical};

use crate::ui::theme::CalendarTheme;
use crate::ui::messages::Message;

/// Create the Day view with configurable time slots
pub fn create_day_view(
    current_date: NaiveDate,
    calendar_theme: &CalendarTheme,
    time_format: &str,
    time_slot_interval: u32,
) -> Element<'static, Message> {
    let today = Local::now().naive_local().date();
    let is_today = current_date == today;
    let now = Local::now().naive_local();
    
    // Header with date and navigation
    let day_name = current_date.format("%A").to_string();
    let date_string = current_date.format("%B %-d, %Y").to_string();
    
    let header = column![
        row![
            button(text("◀").size(16))
                .on_press(Message::PreviousDay)
                .padding(8),
            button(text("Today").size(14))
                .on_press(Message::GoToToday)
                .padding([8, 16]),
            container(
                column![
                    text(&day_name).size(20),
                    text(&date_string).size(14),
                ]
                .align_items(iced::Alignment::Center)
            )
            .width(Length::Fill)
            .center_x(),
            button(text("▶").size(16))
                .on_press(Message::NextDay)
                .padding(8),
        ]
        .spacing(10)
        .padding(10)
        .align_items(iced::Alignment::Center),
    ]
    .spacing(5);

    // Calculate number of slots and slot height based on interval
    let slots_per_hour = 60 / time_slot_interval;
    let total_slots = 24 * slots_per_hour;
    let slot_height = time_slot_interval; // 15min=15px, 30min=30px, 45min=45px, 60min=60px
    
    // Time slots
    let mut time_slots = column![].spacing(0);
    
    let use_24h = time_format == "24h";
    
    // Generate time slots based on interval
    for slot_index in 0..total_slots {
        let hour = slot_index / slots_per_hour;
        let minute = (slot_index % slots_per_hour) * time_slot_interval;
        
        // Only show time label for on-the-hour slots
        let time_label = if minute == 0 {
            if use_24h {
                format!("{:02}:00", hour)
            } else {
                let (display_hour, period) = if hour == 0 {
                    (12, "AM")
                } else if hour < 12 {
                    (hour, "AM")
                } else if hour == 12 {
                    (12, "PM")
                } else {
                    (hour - 12, "PM")
                };
                format!("{:2}:00 {}", display_hour, period)
            }
        } else {
            // Show minutes for non-hour slots
            if use_24h {
                format!("{:02}:{:02}", hour, minute)
            } else {
                let (display_hour, period) = if hour == 0 {
                    (12, "AM")
                } else if hour < 12 {
                    (hour, "AM")
                } else if hour == 12 {
                    (12, "PM")
                } else {
                    (hour - 12, "PM")
                };
                format!("{:2}:{:02} {}", display_hour, minute, period)
            }
        };

        // Highlight current time slot if viewing today
        let is_current_slot = if is_today {
            let current_hour = now.hour() as u32;
            let current_minute = now.minute();
            let current_slot = current_hour * slots_per_hour + (current_minute / time_slot_interval);
            slot_index == current_slot
        } else {
            false
        };

        let theme_colors = calendar_theme.clone();
        let slot_container = container(
            row![
                container(text(&time_label).size(12))
                    .width(80)
                    .padding([8, 10]),
                container(text("")) // Event area - will be populated later
                    .width(Length::Fill)
                    .height(Length::Fixed(slot_height as f32))
                    .style(move |_theme: &Theme| {
                        container::Appearance {
                            background: Some(iced::Background::Color(
                                if is_current_slot {
                                    theme_colors.today_background
                                } else {
                                    theme_colors.day_background
                                }
                            )),
                            border: Border {
                                color: theme_colors.day_border,
                                width: 1.0,
                                radius: 0.0.into(),
                            },
                            ..Default::default()
                        }
                    }),
            ]
            .spacing(0)
        );

        time_slots = time_slots.push(slot_container);
    }

    // Scrollable time slots
    let scrollable_content = scrollable(time_slots)
        .height(Length::Fill);

    let theme_bg = calendar_theme.calendar_background;
    container(
        column![
            header,
            scrollable_content,
        ]
        .spacing(10)
    )
    .padding(10)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_theme: &Theme| {
        container::Appearance {
            background: Some(iced::Background::Color(theme_bg)),
            ..Default::default()
        }
    })
    .into()
}
