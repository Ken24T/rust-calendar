use chrono::{Datelike, Local, NaiveDate};
use iced::widget::{button, column, container, row, text};
use iced::{Border, Element, Length, Theme};
use iced::alignment::Horizontal;

use crate::ui::theme::CalendarTheme;
use crate::ui::messages::Message;

/// Create the Month view with calendar grid
pub fn create_month_view(
    current_date: NaiveDate,
    calendar_theme: &CalendarTheme,
) -> Element<'static, Message> {
    let today = Local::now().naive_local().date();
    
    // Month header with navigation
    let month_name = current_date.format("%B %Y").to_string();
    let month_year_button = button(text(&month_name).size(20))
        .on_press(Message::ToggleDatePicker)
        .padding([8, 16]);
        
    let header = row![
        button(text("◀").size(16))
            .on_press(Message::PreviousMonth)
            .padding(8),
        button(text("Today").size(14))
            .on_press(Message::GoToToday)
            .padding([8, 16]),
        container(month_year_button)
            .width(Length::Fill)
            .center_x(),
        button(text("▶").size(16))
            .on_press(Message::NextMonth)
            .padding(8),
    ]
    .spacing(10)
    .padding(10)
    .align_items(iced::Alignment::Center);

    // Day of week headers
    let day_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let mut day_header_row = row![].spacing(2);
    
    for day in &day_names {
        let day_header = container(
            text(*day)
                .size(14)
                .horizontal_alignment(Horizontal::Center)
        )
        .width(Length::FillPortion(1))
        .padding(8)
        .center_x();
        
        day_header_row = day_header_row.push(day_header);
    }

    // Calculate first day of month and total days
    let first_of_month = current_date.with_day(1).unwrap();
    let first_weekday = first_of_month.weekday().num_days_from_sunday() as i32;
    let days_in_month = get_days_in_month(current_date.year(), current_date.month());
    
    // Build calendar grid (6 rows of 7 days)
    let mut calendar_grid = column![].spacing(2);
    let mut day_counter = 1 - first_weekday;
    
    for _week in 0..6 {
        let mut week_row = row![].spacing(2);
        
        for _day_of_week in 0..7 {
            let day_cell = if day_counter < 1 || day_counter > days_in_month {
                // Empty cell for days outside current month
                container(text(""))
                    .width(Length::FillPortion(1))
                    .height(80)
                    .padding(5)
            } else {
                // Day cell
                let date = NaiveDate::from_ymd_opt(
                    current_date.year(),
                    current_date.month(),
                    day_counter as u32
                ).unwrap();
                
                let is_today = date == today;
                let is_weekend = date.weekday().num_days_from_sunday() == 0 
                    || date.weekday().num_days_from_sunday() == 6;
                
                let day_text = text(format!("{}", day_counter))
                    .size(14);
                
                let mut cell_container = container(day_text)
                    .width(Length::FillPortion(1))
                    .height(80)
                    .padding(5);
                
                // Style based on day type using custom theme colors
                if is_today {
                    let theme_colors = calendar_theme.clone();
                    cell_container = cell_container
                        .style(move |_theme: &Theme| {
                            container::Appearance {
                                background: Some(iced::Background::Color(theme_colors.today_background)),
                                border: Border {
                                    color: theme_colors.today_border,
                                    width: 2.0,
                                    radius: 4.0.into(),
                                },
                                ..Default::default()
                            }
                        });
                } else if is_weekend {
                    let theme_colors = calendar_theme.clone();
                    cell_container = cell_container
                        .style(move |_theme: &Theme| {
                            container::Appearance {
                                background: Some(iced::Background::Color(theme_colors.weekend_background)),
                                border: Border {
                                    color: theme_colors.day_border,
                                    width: 1.0,
                                    radius: 2.0.into(),
                                },
                                ..Default::default()
                            }
                        });
                } else {
                    let theme_colors = calendar_theme.clone();
                    cell_container = cell_container
                        .style(move |_theme: &Theme| {
                            container::Appearance {
                                background: Some(iced::Background::Color(theme_colors.day_background)),
                                border: Border {
                                    color: theme_colors.day_border,
                                    width: 1.0,
                                    radius: 2.0.into(),
                                },
                                ..Default::default()
                            }
                        });
                }
                
                cell_container
            };
            
            week_row = week_row.push(day_cell);
            day_counter += 1;
        }
        
        calendar_grid = calendar_grid.push(week_row);
    }

    let theme_bg = calendar_theme.calendar_background;
    container(
        column![
            header,
            day_header_row,
            calendar_grid,
        ]
        .spacing(5)
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

/// Helper function to get days in a month
fn get_days_in_month(year: i32, month: u32) -> i32 {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap()
    .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
    .num_days() as i32
}
