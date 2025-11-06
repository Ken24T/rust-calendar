use chrono::{Datelike, Local, NaiveDate, Timelike};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Border, Element, Length, Theme};
use iced::alignment::{Horizontal, Vertical};

use crate::ui::theme::CalendarTheme;
use crate::ui::messages::Message;

/// Create the Day view with hourly time slots
pub fn create_day_view(
    current_date: NaiveDate,
    calendar_theme: &CalendarTheme,
    time_format: &str,
) -> Element<'static, Message> {
    let today = Local::now().naive_local().date();
    let is_today = current_date == today;
    
    // Header with date and navigation
    let day_name = current_date.format("%A").to_string();
    let date_string = current_date.format("%B %-d, %Y").to_string();
    
    let header = column![
        row![
            button(text("◀").size(16))
                .on_press(Message::PreviousMonth) // Reusing month navigation for now
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
                .on_press(Message::NextMonth) // Reusing month navigation for now
                .padding(8),
        ]
        .spacing(10)
        .padding(10)
        .align_items(iced::Alignment::Center),
    ]
    .spacing(5);

    // Time slots (24 hours)
    let mut time_slots = column![].spacing(0);
    
    let use_24h = time_format == "24h";
    
    for hour in 0..24 {
        // Format time based on preference
        let time_label = if use_24h {
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
        };

        // Highlight current hour if viewing today
        let is_current_hour = if is_today {
            Local::now().hour() as usize == hour
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
                    .height(60)
                    .style(move |_theme: &Theme| {
                        container::Appearance {
                            background: Some(iced::Background::Color(
                                if is_current_hour {
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
