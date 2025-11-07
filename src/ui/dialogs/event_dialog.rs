// Event creation/edit dialog

use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input, Column, Row};
use iced::{Element, Length, Theme};
use iced::widget::text_input::Id;
use iced_aw::DatePicker;
use crate::models::event::Event;
use crate::ui::messages::Message;
use crate::ui::theme::CalendarTheme;
use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone};

/// State for the event dialog
#[derive(Debug, Clone)]
pub struct EventDialogState {
    pub event_id: Option<i64>,
    pub title: String,
    pub description: String,
    pub location: String,
    pub start_date: String,  // YYYY-MM-DD
    pub start_time: String,  // HH:MM
    pub end_date: String,    // YYYY-MM-DD
    pub end_time: String,    // HH:MM
    pub all_day: bool,
    pub category: String,
    pub color: String,
    pub recurrence: String,
    pub is_editing: bool,
    pub validation_error: Option<String>,
    pub show_start_date_picker: bool,
    pub show_end_date_picker: bool,
}

impl EventDialogState {
    pub fn new() -> Self {
        let now = Local::now();
        let start = now;
        let end = now + chrono::Duration::hours(1);
        
        Self {
            event_id: None,
            title: String::new(),
            description: String::new(),
            location: String::new(),
            start_date: start.format("%Y-%m-%d").to_string(),
            start_time: start.format("%H:%M").to_string(),
            end_date: end.format("%Y-%m-%d").to_string(),
            end_time: end.format("%H:%M").to_string(),
            all_day: false,
            category: String::new(),
            color: "#3B82F6".to_string(),
            recurrence: "None".to_string(),
            is_editing: false,
            validation_error: None,
            show_start_date_picker: false,
            show_end_date_picker: false,
        }
    }
    
    pub fn from_event(event: &Event) -> Self {
        Self {
            event_id: event.id,
            title: event.title.clone(),
            description: event.description.clone().unwrap_or_default(),
            location: event.location.clone().unwrap_or_default(),
            start_date: event.start.format("%Y-%m-%d").to_string(),
            start_time: event.start.format("%H:%M").to_string(),
            end_date: event.end.format("%Y-%m-%d").to_string(),
            end_time: event.end.format("%H:%M").to_string(),
            all_day: event.all_day,
            category: event.category.clone().unwrap_or_default(),
            color: event.color.clone().unwrap_or_else(|| "#3B82F6".to_string()),
            recurrence: event.recurrence_rule.clone().unwrap_or_else(|| "None".to_string()),
            is_editing: true,
            validation_error: None,
            show_start_date_picker: false,
            show_end_date_picker: false,
        }
    }
    
    /// Check if the form is valid for saving
    pub fn is_valid(&self) -> bool {
        // Title is required
        if self.title.trim().is_empty() {
            return false;
        }
        
        // Try to parse dates
        let start_date_valid = NaiveDate::parse_from_str(&self.start_date, "%Y-%m-%d").is_ok();
        let end_date_valid = NaiveDate::parse_from_str(&self.end_date, "%Y-%m-%d").is_ok();
        
        if !start_date_valid || !end_date_valid {
            return false;
        }
        
        // If not all-day, times must be valid
        if !self.all_day {
            let start_time_valid = NaiveTime::parse_from_str(&self.start_time, "%H:%M").is_ok();
            let end_time_valid = NaiveTime::parse_from_str(&self.end_time, "%H:%M").is_ok();
            
            if !start_time_valid || !end_time_valid {
                return false;
            }
        }
        
        // Parse full datetimes to check ordering
        let start_datetime = if self.all_day {
            Self::parse_date(&self.start_date).ok()
        } else {
            Self::parse_datetime(&self.start_date, &self.start_time).ok()
        };
        
        let end_datetime = if self.all_day {
            Self::parse_date(&self.end_date).ok()
        } else {
            Self::parse_datetime(&self.end_date, &self.end_time).ok()
        };
        
        // Both must parse and end must be after start
        match (start_datetime, end_datetime) {
            (Some(start), Some(end)) => end > start,
            _ => false,
        }
    }
    
    pub fn to_event(&self) -> Result<Event, String> {
        // Validate required fields
        if self.title.trim().is_empty() {
            return Err("Title is required".to_string());
        }
        
        // Parse dates and times
        let start_datetime = if self.all_day {
            Self::parse_date(&self.start_date)?
        } else {
            Self::parse_datetime(&self.start_date, &self.start_time)?
        };
        
        let end_datetime = if self.all_day {
            Self::parse_date(&self.end_date)?
        } else {
            Self::parse_datetime(&self.end_date, &self.end_time)?
        };
        
        // Validate that end is after start
        if end_datetime <= start_datetime {
            return Err("End date/time must be after start date/time".to_string());
        }
        
        let mut builder = Event::builder()
            .title(&self.title)
            .start(start_datetime)
            .end(end_datetime)
            .all_day(self.all_day);
        
        if !self.description.is_empty() {
            builder = builder.description(&self.description);
        }
        
        if !self.location.is_empty() {
            builder = builder.location(&self.location);
        }
        
        if !self.category.is_empty() {
            builder = builder.category(&self.category);
        }
        
        if !self.color.is_empty() {
            builder = builder.color(&self.color);
        }
        
        if !self.recurrence.is_empty() && self.recurrence != "None" {
            builder = builder.recurrence_rule(&self.recurrence);
        }
        
        let mut event = builder.build().map_err(|e| e.to_string())?;
        
        // Preserve ID if editing
        if self.is_editing {
            event.id = self.event_id;
        }
        
        Ok(event)
    }
    
    fn parse_date(date_str: &str) -> Result<DateTime<Local>, String> {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| format!("Invalid date format: {}", date_str))?;
        
        let time = NaiveTime::from_hms_opt(0, 0, 0)
            .ok_or_else(|| "Failed to create time".to_string())?;
        
        let naive_dt = date.and_time(time);
        
        Local.from_local_datetime(&naive_dt)
            .single()
            .ok_or_else(|| "Invalid date/time".to_string())
    }
    
    fn parse_datetime(date_str: &str, time_str: &str) -> Result<DateTime<Local>, String> {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| format!("Invalid date format: {}", date_str))?;
        
        let time = NaiveTime::parse_from_str(time_str, "%H:%M")
            .map_err(|_| format!("Invalid time format: {}", time_str))?;
        
        let naive_dt = date.and_time(time);
        
        Local.from_local_datetime(&naive_dt)
            .single()
            .ok_or_else(|| "Invalid date/time".to_string())
    }
}

impl Default for EventDialogState {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the event dialog view
pub fn create_event_dialog<'a>(
    state: &'a EventDialogState,
    theme: &CalendarTheme,
) -> Element<'a, Message> {
    let title_text = if state.is_editing {
        "Edit Event"
    } else {
        "Create Event"
    };
    
    let _iced_theme = &theme.base;
    
    let mut content = Column::new()
        .spacing(12)
        .padding(16)
        .width(Length::Fixed(450.0));
    
    // Title
    content = content.push(
        text(title_text)
            .size(20)
    );
    
    // Validation error
    if let Some(error) = &state.validation_error {
        content = content.push(
            container(text(error).style(iced::theme::Text::Color(iced::Color::from_rgb(0.8, 0.2, 0.2))))
                .padding(8)
                .style(|_theme: &iced::Theme| {
                    container::Appearance {
                        background: Some(iced::Background::Color(iced::Color::from_rgba(0.8, 0.2, 0.2, 0.1))),
                        border: iced::Border {
                            color: iced::Color::from_rgb(0.8, 0.2, 0.2),
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                })
        );
    }
    
    // Title input
    content = content.push(
        column![
            text("Title *").size(12),
            text_input("Event title", &state.title)
                .on_input(Message::UpdateEventTitle)
                .id(Id::new("title"))
                .padding(8)
        ]
        .spacing(4)
    );
    
    // Description
    content = content.push(
        column![
            text("Description").size(12),
            text_input("Event description", &state.description)
                .on_input(Message::UpdateEventDescription)
                .id(Id::new("description"))
                .padding(8)
        ]
        .spacing(4)
    );
    
    // Location
    content = content.push(
        column![
            text("Location").size(12),
            text_input("Event location", &state.location)
                .on_input(Message::UpdateEventLocation)
                .id(Id::new("location"))
                .padding(8)
        ]
        .spacing(4)
    );
    
    // All-day checkbox
    content = content.push(
        checkbox("All-day event", state.all_day)
            .on_toggle(Message::ToggleEventAllDay)
            .size(16)
    );
    
    // Start date/time
    if state.all_day {
        // Parse current start date for date picker
        let start_date_parsed = NaiveDate::parse_from_str(&state.start_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Local::now().date_naive());
        let aw_date: iced_aw::date_picker::Date = start_date_parsed.into();
        
        content = content.push(
            column![
                text("Start Date *").size(12),
                DatePicker::new(
                    state.show_start_date_picker,
                    aw_date,
                    button(text(&state.start_date).size(12))
                        .on_press(Message::OpenStartDatePicker)
                        .padding(8)
                        .width(Length::Fill),
                    Message::CancelStartDatePicker,
                    Message::SubmitStartDate
                )
            ]
            .spacing(4)
        );
    } else {
        // Parse current start date for date picker
        let start_date_parsed = NaiveDate::parse_from_str(&state.start_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Local::now().date_naive());
        let aw_date: iced_aw::date_picker::Date = start_date_parsed.into();
        
        content = content.push(
            column![
                text("Start *").size(12),
                row![
                    container(
                        DatePicker::new(
                            state.show_start_date_picker,
                            aw_date,
                            button(text(&state.start_date).size(12))
                                .on_press(Message::OpenStartDatePicker)
                                .padding(8)
                                .width(Length::Fill),
                            Message::CancelStartDatePicker,
                            Message::SubmitStartDate
                        )
                    )
                    .width(Length::FillPortion(2)),
                    text_input("HH:MM", &state.start_time)
                        .on_input(Message::UpdateEventStartTime)
                        .id(Id::new("start_time"))
                        .padding(8)
                        .width(Length::FillPortion(1)),
                ]
                .spacing(8)
            ]
            .spacing(4)
        );
    }
    
    // End date/time
    if state.all_day {
        // Parse current end date for date picker
        let end_date_parsed = NaiveDate::parse_from_str(&state.end_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Local::now().date_naive());
        let aw_date: iced_aw::date_picker::Date = end_date_parsed.into();
        
        content = content.push(
            column![
                text("End Date *").size(12),
                DatePicker::new(
                    state.show_end_date_picker,
                    aw_date,
                    button(text(&state.end_date).size(12))
                        .on_press(Message::OpenEndDatePicker)
                        .padding(8)
                        .width(Length::Fill),
                    Message::CancelEndDatePicker,
                    Message::SubmitEndDate
                )
            ]
            .spacing(4)
        );
    } else {
        // Parse current end date for date picker
        let end_date_parsed = NaiveDate::parse_from_str(&state.end_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Local::now().date_naive());
        let aw_date: iced_aw::date_picker::Date = end_date_parsed.into();
        
        content = content.push(
            column![
                text("End *").size(12),
                row![
                    container(
                        DatePicker::new(
                            state.show_end_date_picker,
                            aw_date,
                            button(text(&state.end_date).size(12))
                                .on_press(Message::OpenEndDatePicker)
                                .padding(8)
                                .width(Length::Fill),
                            Message::CancelEndDatePicker,
                            Message::SubmitEndDate
                        )
                    )
                    .width(Length::FillPortion(2)),
                    text_input("HH:MM", &state.end_time)
                        .on_input(Message::UpdateEventEndTime)
                        .id(Id::new("end_time"))
                        .padding(8)
                        .width(Length::FillPortion(1)),
                ]
                .spacing(8)
            ]
            .spacing(4)
        );
    }
    
    // Category
    content = content.push(
        column![
            text("Category").size(12),
            text_input("e.g., Work, Personal", &state.category)
                .on_input(Message::UpdateEventCategory)
                .id(Id::new("category"))
                .padding(8)
        ]
        .spacing(4)
    );
    
    // Color
    content = content.push(
        column![
            text("Color").size(12),
            row![
                text_input("#RRGGBB", &state.color)
                    .on_input(Message::UpdateEventColor)
                    .id(Id::new("color"))
                    .padding(8)
                    .width(Length::FillPortion(3)),
                container(text("  "))
                    .width(Length::Fixed(30.0))
                    .height(Length::Fixed(30.0))
            ]
            .spacing(8)
        ]
        .spacing(4)
    );
    
    // Recurrence
    let recurrence_options = vec![
        "None".to_string(),
        "FREQ=DAILY".to_string(),
        "FREQ=WEEKLY".to_string(),
        "FREQ=MONTHLY".to_string(),
        "FREQ=YEARLY".to_string(),
    ];
    
    content = content.push(
        column![
            text("Recurrence").size(12),
            pick_list(
                recurrence_options,
                Some(state.recurrence.clone()),
                Message::UpdateEventRecurrence
            )
            .padding(8)
        ]
        .spacing(4)
    );
    
    // Validation hint - show when form is invalid
    if !state.is_valid() {
        let hint_text = if state.title.trim().is_empty() {
            "Title is required"
        } else {
            "End date/time must be after start date/time"
        };
        
        content = content.push(
            container(text(hint_text).size(12))
                .padding(8)
                .style(|_theme: &Theme| {
                    container::Appearance {
                        background: Some(iced::Background::Color(iced::Color::from_rgb(1.0, 0.9, 0.9))),
                        border: iced::Border {
                            color: iced::Color::from_rgb(0.8, 0.0, 0.0),
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                })
        );
    }
    
    // Buttons
    let mut button_row = Row::new().spacing(8);
    
    // Save button - only enabled if form is valid
    let save_button = button(text("Save").size(14))
        .padding(8);
    
    let save_button = if state.is_valid() {
        save_button.on_press(Message::SaveEvent)
    } else {
        save_button
    };
    
    button_row = button_row.push(save_button);
    
    button_row = button_row.push(
        button(text("Cancel").size(14))
            .on_press(Message::CloseEventDialog)
            .padding(8)
    );
    
    if state.is_editing {
        if let Some(id) = state.event_id {
            button_row = button_row.push(
                button(text("Delete").size(14))
                    .on_press(Message::ConfirmDeleteEvent(id))
                    .padding(8)
            );
        }
    }
    
    content = content.push(button_row);
    
    // Wrap in scrollable with max height and proper background
    let scrollable_content = iced::widget::scrollable(content)
        .height(Length::Fixed(500.0));
    
    let dialog_container = container(scrollable_content)
        .width(Length::Fixed(450.0))
        .padding(0)
        .style(|theme: &iced::Theme| {
            container::Appearance {
                background: Some(iced::Background::Color(
                    if theme.palette().background == iced::Color::WHITE {
                        iced::Color::WHITE
                    } else {
                        iced::Color::from_rgb(0.15, 0.15, 0.15)
                    }
                )),
                border: iced::Border {
                    color: iced::Color::from_rgb(0.5, 0.5, 0.5),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: iced::Shadow {
                    color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 10.0,
                },
                ..Default::default()
            }
        });
    
    container(dialog_container)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
}

/// Parse color from hex string
fn parse_color(hex: &str) -> iced::Color {
    let hex = hex.trim_start_matches('#');
    
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return iced::Color::from_rgb(
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
            );
        }
    }
    
    // Default color if parsing fails
    iced::Color::from_rgb(0.23, 0.51, 0.96) // #3B82F6
}
