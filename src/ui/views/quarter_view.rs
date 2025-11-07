use chrono::{Datelike, Local, NaiveDate};
use iced::widget::{button, column, container, mouse_area, row, scrollable, text};
use iced::{Border, Element, Length, Theme};
use iced::alignment::Horizontal;

use crate::ui::theme::CalendarTheme;
use crate::ui::messages::Message;

/// Get the number of days in a given month
fn get_days_in_month(year: i32, month: u32) -> i32 {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .unwrap()
            .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
            .num_days() as i32
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .unwrap()
            .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
            .num_days() as i32
    }
}

/// Create a single month mini-calendar for the quarter view
fn create_mini_month(
    year: i32,
    month: u32,
    today: NaiveDate,
    calendar_theme: &CalendarTheme,
) -> Element<'static, Message> {
    // Month name
    let date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let month_name = date.format("%B %Y").to_string();
    
    let month_header = container(
        text(&month_name).size(14)
    )
    .width(Length::Fill)
    .center_x()
    .padding([5, 0]);
    
    // Day of week headers
    let day_names = ["S", "M", "T", "W", "T", "F", "S"];
    let mut day_header_row = row![].spacing(1);
    
    for day in &day_names {
        let day_header = container(
            text(*day)
                .size(10)
                .horizontal_alignment(Horizontal::Center)
        )
        .width(Length::FillPortion(1))
        .padding(2)
        .center_x();
        
        day_header_row = day_header_row.push(day_header);
    }
    
    // Calculate first day of month and total days
    let first_of_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let first_weekday = first_of_month.weekday().num_days_from_sunday() as i32;
    let days_in_month = get_days_in_month(year, month);
    
    // Build calendar grid (6 rows of 7 days)
    let mut calendar_grid = column![].spacing(1);
    let mut day_counter = 1 - first_weekday;
    
    for _week in 0..6 {
        let mut week_row = row![].spacing(1);
        
        for _day_of_week in 0..7 {
            let day_cell = if day_counter < 1 || day_counter > days_in_month {
                // Empty cell for days outside current month
                container(text(""))
                    .width(Length::FillPortion(1))
                    .height(30)
                    .padding(2)
            } else {
                // Day cell
                let date = NaiveDate::from_ymd_opt(year, month, day_counter as u32).unwrap();
                let is_today = date == today;
                let is_weekend = date.weekday().num_days_from_sunday() == 0 
                    || date.weekday().num_days_from_sunday() == 6;
                
                // Day cell - make it clickable using mouse_area for clean styling
                // The container provides the colored background
                let day_text_widget = container(
                    text(format!("{}", day_counter))
                        .size(10)
                        .horizontal_alignment(Horizontal::Center)
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y();
                
                let clickable_day = mouse_area(day_text_widget)
                    .on_press(Message::GoToDateInWeekView(year, month, day_counter as u32));
                
                // Style based on day type - apply styling to container
                let cell_container = if is_today {
                    let theme_colors = calendar_theme.clone();
                    container(clickable_day)
                        .width(Length::FillPortion(1))
                        .height(30)
                        .padding(2)
                        .center_x()
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
                        })
                } else if is_weekend {
                    let theme_colors = calendar_theme.clone();
                    container(clickable_day)
                        .width(Length::FillPortion(1))
                        .height(30)
                        .padding(2)
                        .center_x()
                        .style(move |_theme: &Theme| {
                            container::Appearance {
                                background: Some(iced::Background::Color(theme_colors.weekend_background)),
                                border: Border {
                                    color: theme_colors.day_border,
                                    width: 0.5,
                                    radius: 2.0.into(),
                                },
                                ..Default::default()
                            }
                        })
                } else {
                    let theme_colors = calendar_theme.clone();
                    container(clickable_day)
                        .width(Length::FillPortion(1))
                        .height(30)
                        .padding(2)
                        .center_x()
                        .style(move |_theme: &Theme| {
                            container::Appearance {
                                background: Some(iced::Background::Color(theme_colors.day_background)),
                                border: Border {
                                    color: theme_colors.day_border,
                                    width: 0.5,
                                    radius: 2.0.into(),
                                },
                                ..Default::default()
                            }
                        })
                };
                
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
            month_header,
            day_header_row,
            calendar_grid,
        ]
        .spacing(3)
    )
    .padding(8)
    .width(Length::FillPortion(1))
    .style(move |_theme: &Theme| {
        container::Appearance {
            background: Some(iced::Background::Color(theme_bg)),
            border: Border {
                color: iced::Color::from_rgb(0.7, 0.7, 0.7),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

/// Create the Quarter view showing 3 months
pub fn create_quarter_view(
    current_date: NaiveDate,
    calendar_theme: &CalendarTheme,
) -> Element<'static, Message> {
    let today = Local::now().naive_local().date();
    
    // Calculate which quarter we're in and get the 3 months
    let current_month = current_date.month();
    let current_year = current_date.year();
    
    // Determine quarter start month (1, 4, 7, or 10)
    let quarter_start_month = ((current_month - 1) / 3) * 3 + 1;
    
    // Get the three months of the quarter
    let mut months = Vec::new();
    for i in 0..3 {
        let month = quarter_start_month + i;
        let year = current_year;
        months.push((year, month));
    }
    
    // Quarter name
    let quarter_num = (quarter_start_month - 1) / 3 + 1;
    let quarter_name = format!("Q{} {}", quarter_num, current_year);
    
    // Header with navigation
    let header = row![
        button(text("◀").size(16))
            .on_press(Message::PreviousQuarter)
            .padding(8),
        button(text("Today").size(14))
            .on_press(Message::GoToToday)
            .padding([8, 16]),
        container(text(&quarter_name).size(20))
            .width(Length::Fill)
            .center_x(),
        button(text("▶").size(16))
            .on_press(Message::NextQuarter)
            .padding(8),
    ]
    .spacing(10)
    .padding(10)
    .align_items(iced::Alignment::Center);
    
    // Create the three month mini-calendars
    let month1 = create_mini_month(months[0].0, months[0].1, today, calendar_theme);
    let month2 = create_mini_month(months[1].0, months[1].1, today, calendar_theme);
    let month3 = create_mini_month(months[2].0, months[2].1, today, calendar_theme);
    
    // Layout the three months in a row
    let months_row = row![month1, month2, month3]
        .spacing(10)
        .padding(10);
    
    let content = column![
        header,
        scrollable(months_row)
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .spacing(0)
    .width(Length::Fill)
    .height(Length::Fill);
    
    let theme_bg = calendar_theme.app_background;
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(10)
        .style(move |_theme: &Theme| {
            container::Appearance {
                background: Some(iced::Background::Color(theme_bg)),
                ..Default::default()
            }
        })
        .into()
}
