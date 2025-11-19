use chrono::{Datelike, Local, NaiveDate, Timelike, Weekday};
use iced::widget::{button, column, container, horizontal_rule, row, scrollable, text};
use iced::{Border, Element, Length, Theme};
use iced::widget::rule;
use iced_aw::menu::{Item, Menu, MenuBar};

use crate::models::event::Event;
use crate::ui::theme::CalendarTheme;
use crate::ui::messages::Message;

/// Create the Work Week view with configurable time slots showing 5 business days
pub fn create_workweek_view(
    current_date: NaiveDate,
    theme: &CalendarTheme,
    time_format: &str,
    time_slot_interval: u32,
    first_day_of_week: u8,
    events: &[Event],
) -> Element<'static, Message> {
    
    // Calculate the start of the week based on first_day_of_week setting
    let current_weekday = current_date.weekday().num_days_from_sunday();
    let start_weekday = first_day_of_week as u32;
    
    // Calculate days to subtract to get to the start of the week
    let days_back = if current_weekday >= start_weekday {
        current_weekday - start_weekday
    } else {
        7 + current_weekday - start_weekday
    };
    
    let week_start = current_date - chrono::Duration::days(days_back as i64);
    
    // Get only the first 5 days of the week (work week)
    let workweek_days: Vec<NaiveDate> = (0..5)
        .map(|i| week_start + chrono::Duration::days(i))
        .collect();
    
    // Work week navigation header
    let week_end = workweek_days[4];
    let date_range = if week_start.month() == week_end.month() {
        format!("{} {} - {}, {}", 
            week_start.format("%B"),
            week_start.day(),
            week_end.day(),
            week_start.year()
        )
    } else if week_start.year() == week_end.year() {
        format!("{} {} - {} {}, {}", 
            week_start.format("%B"),
            week_start.day(),
            week_end.format("%B"),
            week_end.day(),
            week_start.year()
        )
    } else {
        format!("{} {}, {} - {} {}, {}", 
            week_start.format("%B"),
            week_start.day(),
            week_start.year(),
            week_end.format("%B"),
            week_end.day(),
            week_end.year()
        )
    };
    
    // Interval display text
    let interval_text = match time_slot_interval {
        15 => "Interval: 15 min",
        30 => "Interval: 30 min",
        45 => "Interval: 45 min",
        60 => "Interval: 1 hour",
        _ => "Interval: 1 hour",
    };
    
    // Create interval menu with checkmarks for current selection
    let interval_menu = Menu::new(vec![
        Item::new(
            button(
                text(if time_slot_interval == 15 { "✓ 15 minutes" } else { "  15 minutes" }).size(12)
            )
            .on_press(Message::UpdateTimeSlotInterval(15))
            .padding([6, 12])
            .width(Length::Fill)
        ),
        Item::new(
            button(
                text(if time_slot_interval == 30 { "✓ 30 minutes" } else { "  30 minutes" }).size(12)
            )
            .on_press(Message::UpdateTimeSlotInterval(30))
            .padding([6, 12])
            .width(Length::Fill)
        ),
        Item::new(
            button(
                text(if time_slot_interval == 45 { "✓ 45 minutes" } else { "  45 minutes" }).size(12)
            )
            .on_press(Message::UpdateTimeSlotInterval(45))
            .padding([6, 12])
            .width(Length::Fill)
        ),
        Item::new(
            button(
                text(if time_slot_interval == 60 { "✓ 60 minutes (1 hour)" } else { "  60 minutes (1 hour)" }).size(12)
            )
            .on_press(Message::UpdateTimeSlotInterval(60))
            .padding([6, 12])
            .width(Length::Fill)
        ),
    ])
    .max_width(200.0)
    .offset(0.0)
    .spacing(0.0);
    
    let interval_menu_item = Item::with_menu(
        button(text(interval_text).size(12))
            .padding([4, 8]),
        interval_menu
    );
    
    let interval_menu_bar = MenuBar::new(vec![interval_menu_item]);
    
    let header = column![
        row![
            button(text("◀").size(16))
                .on_press(Message::PreviousWeek)
                .padding(8),
            button(text("Today").size(14))
                .on_press(Message::GoToToday)
                .padding([8, 16]),
            container(text(&date_range).size(16))
                .width(Length::Fill)
                .center_x(),
            interval_menu_bar,
            button(text("▶").size(16))
                .on_press(Message::NextWeek)
                .padding(8),
        ]
        .spacing(10)
        .padding(10)
        .align_items(iced::Alignment::Center),
    ]
    .spacing(0);
    
    // Calculate total time slots for the day
    let total_slots = 1440 / time_slot_interval; // Total minutes in a day / interval
    
    // Get current time for highlighting
    let now = Local::now().naive_local();
    let today = now.date();
    let current_total_minutes = now.hour() * 60 + now.minute();
    
    // Day headers
    let day_headers: Vec<Element<Message>> = workweek_days.iter().map(|date| {
        let day_name = date.format("%a").to_string(); // Mon, Tue, etc.
        let day_number = date.day();
        let is_today = *date == today;
        
        let header_text = column![
            text(&day_name).size(12),
            text(format!("{}", day_number)).size(16),
        ]
        .align_items(iced::Alignment::Center)
        .spacing(2);
        
        let theme_colors = theme.clone();
        container(header_text)
            .padding(8)
            .width(Length::FillPortion(1))
            .center_x()
            .style(move |_theme: &Theme| {
                container::Appearance {
                    background: if is_today {
                        Some(iced::Background::Color(theme_colors.today_background))
                    } else {
                        Some(iced::Background::Color(theme_colors.day_background))
                    },
                    border: Border {
                        color: theme_colors.day_border,
                        width: 0.5,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
    }).collect();
    
    // Add spacer for time column, then day headers
    let day_headers_row = row![
        // Spacer to match time label width
        container(text("")).width(Length::Fixed(70.0)),
        row(day_headers).spacing(0).width(Length::Fill),
    ]
    .spacing(0)
    .width(Length::Fill);
    
    // Time column (left side) and day columns
    let mut grid_rows: Vec<Element<Message>> = Vec::new();
    
    for slot_index in 0..total_slots {
        let total_minutes_elapsed = slot_index * time_slot_interval;
        let hour = total_minutes_elapsed / 60;
        let minute = total_minutes_elapsed % 60;
        
        // Format time based on user preference
        let time_string = if time_format == "12-hour" {
            let period = if hour < 12 { "AM" } else { "PM" };
            let display_hour = if hour == 0 { 12 } else if hour > 12 { hour - 12 } else { hour };
            format!("{:2}:{:02} {}", display_hour, minute, period)
        } else {
            format!("{:2}:{:02}", hour, minute)
        };
        
        // Time label
        let time_label = container(text(&time_string).size(10))
            .padding(4)
            .width(Length::Fixed(70.0))
            .style(move |theme: &Theme| {
                let palette = theme.extended_palette();
                container::Appearance {
                    border: Border {
                        color: palette.background.strong.color,
                        width: 0.5,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            });
        
        // Create cells for each work day
        let day_cells: Vec<Element<Message>> = workweek_days.iter().map(|date| {
            let is_today = *date == today;
            let is_current_slot = is_today && 
                current_total_minutes >= total_minutes_elapsed && 
                current_total_minutes < (total_minutes_elapsed + time_slot_interval);
            
            // Find events for this day and time slot
            let slot_start_minutes = total_minutes_elapsed;
            let slot_end_minutes = total_minutes_elapsed + time_slot_interval;
            
            let mut event_widgets = column![].spacing(1);
            for event in events {
                // Check if event is on this date
                if event.start.date_naive() == *date || event.end.date_naive() == *date {
                    let event_start_minutes = (event.start.hour() * 60 + event.start.minute()) as u32;
                    
                    // Only show event in the slot where it starts
                    if event_start_minutes >= slot_start_minutes && event_start_minutes < slot_end_minutes {
                        let event_btn = button(
                            text(event.display_label()).size(9)
                        )
                        .on_press(Message::EditEvent(event.id.unwrap_or(0)))
                        .padding([2, 4]);
                        
                        event_widgets = event_widgets.push(event_btn);
                    }
                }
            }
            
            let theme_colors = theme.clone();
            let year = date.year();
            let month = date.month();
            let day = date.day();
            
            let cell_bg = if is_current_slot {
                theme_colors.today_background
            } else {
                theme_colors.day_background
            };
            
            // Wrap in clickable button for creating new events
            button(
                container(event_widgets)
                    .padding(1)
                    .height(Length::Fixed(40.0))
                    .width(Length::Fill)
                    .style(move |_theme: &Theme| {
                        container::Appearance {
                            background: Some(iced::Background::Color(cell_bg)),
                            border: Border {
                                color: theme_colors.day_border,
                                width: 0.5,
                                radius: 0.0.into(),
                            },
                            ..Default::default()
                        }
                    })
            )
            .on_press(Message::OpenEventDialogWithDate(year, month, day, "FREQ=WEEKLY".to_string()))
            .width(Length::FillPortion(1))
            .style(iced::theme::Button::Text)
            .into()
        }).collect();
        
        let row_content = row![
            time_label,
            row(day_cells).spacing(0).width(Length::Fill),
        ]
        .spacing(0)
        .width(Length::Fill);
        
        grid_rows.push(row_content.into());
        
        // Add horizontal rule between rows (except after last row)
        if slot_index < total_slots - 1 {
            let border_color = theme.day_border;
            grid_rows.push(
                horizontal_rule(1).style(move |_theme: &Theme| {
                    rule::Appearance {
                        color: border_color,
                        width: 1,
                        radius: 0.0.into(),
                        fill_mode: rule::FillMode::Full,
                    }
                })
                .into()
            );
        }
    }
    
    let grid = column(grid_rows)
        .spacing(0)
        .width(Length::Fill);
    
    let content = column![
        header,
        day_headers_row,
        scrollable(grid)
            .height(Length::Fill)
            .width(Length::Fill),
    ]
    .spacing(0)
    .width(Length::Fill)
    .height(Length::Fill);
    
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
