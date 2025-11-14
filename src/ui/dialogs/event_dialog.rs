// Event creation/edit dialog

use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input, Column, Row};
use iced::{Element, Length, Theme, Color};
use iced::widget::text_input::Id;
use crate::models::event::Event;
use crate::ui::messages::Message;
use crate::ui::theme::CalendarTheme;
use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone, Timelike, Datelike};

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
    pub recurrence_days: Vec<String>, // For weekly: ["MO", "WE", "FR"]
    pub is_editing: bool,
    pub validation_error: Option<String>,
}

impl EventDialogState {
    pub fn new() -> Self {
        Self::with_settings(60, "08:00") // Default to 60 minutes, 8:00 AM
    }
    
    pub fn with_duration(duration_minutes: u32) -> Self {
        Self::with_settings(duration_minutes, "08:00")
    }
    
    pub fn with_settings(duration_minutes: u32, default_start_time: &str) -> Self {
        let now = Local::now();
        let today = now.date_naive();
        
        // Parse default start time (HH:MM format)
        let (start_hour, start_minute) = if let Some((h, m)) = default_start_time.split_once(':') {
            (
                h.parse::<u32>().unwrap_or(8),
                m.parse::<u32>().unwrap_or(0)
            )
        } else {
            (8, 0) // Fallback to 8:00 AM
        };
        
        // Create start time at default time
        let start = Local.with_ymd_and_hms(
            today.year(),
            today.month(),
            today.day(),
            start_hour,
            start_minute,
            0
        ).single().unwrap_or(now);
        
        let end = start + chrono::Duration::minutes(duration_minutes as i64);
        
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
            recurrence_days: Vec::new(),
            is_editing: false,
            validation_error: None,
        }
    }
    
    pub fn with_date_and_recurrence(year: i32, month: u32, day: u32, recurrence: String, duration_minutes: u32, default_start_time: &str) -> Self {
        // Parse default start time (HH:MM format)
        let (start_hour, start_minute) = if let Some((h, m)) = default_start_time.split_once(':') {
            (
                h.parse::<u32>().unwrap_or(8),
                m.parse::<u32>().unwrap_or(0)
            )
        } else {
            (8, 0) // Fallback to 8:00 AM
        };
        
        // Create start time at specified date and default time
        let start = Local.with_ymd_and_hms(
            year,
            month,
            day,
            start_hour,
            start_minute,
            0
        ).single().unwrap_or_else(|| Local::now());
        
        let end = start + chrono::Duration::minutes(duration_minutes as i64);
        
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
            recurrence,
            recurrence_days: Vec::new(),
            is_editing: false,
            validation_error: None,
        }
    }
    
    pub fn from_event(event: &Event) -> Self {
        // Parse recurrence_days from RRULE if it exists
        let recurrence_days = if let Some(rrule) = &event.recurrence_rule {
            extract_byday_from_rrule(rrule)
        } else {
            Vec::new()
        };
        
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
            recurrence_days,
            is_editing: true,
            validation_error: None,
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
            let mut rrule = self.recurrence.clone();
            
            // For weekly recurrence, add BYDAY if days are selected
            if self.recurrence.contains("FREQ=WEEKLY") && !self.recurrence_days.is_empty() {
                // Check if BYDAY already exists in the rrule
                if !rrule.contains("BYDAY=") {
                    rrule = format!("{};BYDAY={}", rrule, self.recurrence_days.join(","));
                }
            }
            
            builder = builder.recurrence_rule(&rrule);
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
    // Parse current start date
    let start_date_str = state.start_date.clone();
    let start_date_str_year = start_date_str.clone();
    let start_date_str_month = start_date_str.clone();
    let start_date_str_day = start_date_str.clone();
    let start_date_parsed = NaiveDate::parse_from_str(&start_date_str, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().date_naive());
    
    let years: Vec<String> = (2020..=2030).map(|y| y.to_string()).collect();
    let months: Vec<String> = (1..=12).map(|m| format!("{:02}", m)).collect();
    let days: Vec<String> = (1..=31).map(|d| format!("{:02}", d)).collect();
    
    if state.all_day {
        content = content.push(
            column![
                text("Start Date *").size(12),
                row![
                    pick_list(
                        years.clone(),
                        Some(start_date_parsed.year().to_string()),
                        move |year| {
                            let parts: Vec<&str> = start_date_str_year.split('-').collect();
                            let month = if parts.len() > 1 { parts[1] } else { "01" };
                            let day = if parts.len() > 2 { parts[2] } else { "01" };
                            Message::UpdateEventStartDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(2)),
                    pick_list(
                        months.clone(),
                        Some(format!("{:02}", start_date_parsed.month())),
                        move |month| {
                            let parts: Vec<&str> = start_date_str_month.split('-').collect();
                            let year = if parts.len() > 0 { parts[0] } else { "2025" };
                            let day = if parts.len() > 2 { parts[2] } else { "01" };
                            Message::UpdateEventStartDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(2)),
                    pick_list(
                        days.clone(),
                        Some(format!("{:02}", start_date_parsed.day())),
                        move |day| {
                            let parts: Vec<&str> = start_date_str_day.split('-').collect();
                            let year = if parts.len() > 0 { parts[0] } else { "2025" };
                            let month = if parts.len() > 1 { parts[1] } else { "01" };
                            Message::UpdateEventStartDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(2)),
                ]
                .spacing(4)
            ]
            .spacing(4)
        );
    } else {
        let start_date_str2 = state.start_date.clone();
        let start_date_str2_year = start_date_str2.clone();
        let start_date_str2_month = start_date_str2.clone();
        let start_date_str2_day = start_date_str2.clone();
        content = content.push(
            column![
                text("Start *").size(12),
                row![
                    pick_list(
                        years.clone(),
                        Some(start_date_parsed.year().to_string()),
                        move |year| {
                            let parts: Vec<&str> = start_date_str2_year.split('-').collect();
                            let month = if parts.len() > 1 { parts[1] } else { "01" };
                            let day = if parts.len() > 2 { parts[2] } else { "01" };
                            Message::UpdateEventStartDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(1)),
                    pick_list(
                        months.clone(),
                        Some(format!("{:02}", start_date_parsed.month())),
                        move |month| {
                            let parts: Vec<&str> = start_date_str2_month.split('-').collect();
                            let year = if parts.len() > 0 { parts[0] } else { "2025" };
                            let day = if parts.len() > 2 { parts[2] } else { "01" };
                            Message::UpdateEventStartDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(1)),
                    pick_list(
                        days.clone(),
                        Some(format!("{:02}", start_date_parsed.day())),
                        move |day| {
                            let parts: Vec<&str> = start_date_str2_day.split('-').collect();
                            let year = if parts.len() > 0 { parts[0] } else { "2025" };
                            let month = if parts.len() > 1 { parts[1] } else { "01" };
                            Message::UpdateEventStartDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(1)),
                    text_input("HH:MM", &state.start_time)
                        .on_input(Message::UpdateEventStartTime)
                        .id(Id::new("start_time"))
                        .padding(8)
                        .width(Length::FillPortion(1)),
                ]
                .spacing(4)
            ]
            .spacing(4)
        );
    }
    
    // End date/time
    // Parse current end date
    let end_date_str = state.end_date.clone();
    let end_date_str_year = end_date_str.clone();
    let end_date_str_month = end_date_str.clone();
    let end_date_str_day = end_date_str.clone();
    let end_date_parsed = NaiveDate::parse_from_str(&end_date_str, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().date_naive());
    
    if state.all_day {
        content = content.push(
            column![
                text("End Date *").size(12),
                row![
                    pick_list(
                        years.clone(),
                        Some(end_date_parsed.year().to_string()),
                        move |year| {
                            let parts: Vec<&str> = end_date_str_year.split('-').collect();
                            let month = if parts.len() > 1 { parts[1] } else { "01" };
                            let day = if parts.len() > 2 { parts[2] } else { "01" };
                            Message::UpdateEventEndDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(2)),
                    pick_list(
                        months.clone(),
                        Some(format!("{:02}", end_date_parsed.month())),
                        move |month| {
                            let parts: Vec<&str> = end_date_str_month.split('-').collect();
                            let year = if parts.len() > 0 { parts[0] } else { "2025" };
                            let day = if parts.len() > 2 { parts[2] } else { "01" };
                            Message::UpdateEventEndDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(2)),
                    pick_list(
                        days.clone(),
                        Some(format!("{:02}", end_date_parsed.day())),
                        move |day| {
                            let parts: Vec<&str> = end_date_str_day.split('-').collect();
                            let year = if parts.len() > 0 { parts[0] } else { "2025" };
                            let month = if parts.len() > 1 { parts[1] } else { "01" };
                            Message::UpdateEventEndDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(2)),
                ]
                .spacing(4)
            ]
            .spacing(4)
        );
    } else {
        let end_date_str2 = state.end_date.clone();
        let end_date_str2_year = end_date_str2.clone();
        let end_date_str2_month = end_date_str2.clone();
        let end_date_str2_day = end_date_str2.clone();
        content = content.push(
            column![
                text("End *").size(12),
                row![
                    pick_list(
                        years,
                        Some(end_date_parsed.year().to_string()),
                        move |year| {
                            let parts: Vec<&str> = end_date_str2_year.split('-').collect();
                            let month = if parts.len() > 1 { parts[1] } else { "01" };
                            let day = if parts.len() > 2 { parts[2] } else { "01" };
                            Message::UpdateEventEndDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(1)),
                    pick_list(
                        months,
                        Some(format!("{:02}", end_date_parsed.month())),
                        move |month| {
                            let parts: Vec<&str> = end_date_str2_month.split('-').collect();
                            let year = if parts.len() > 0 { parts[0] } else { "2025" };
                            let day = if parts.len() > 2 { parts[2] } else { "01" };
                            Message::UpdateEventEndDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(1)),
                    pick_list(
                        days,
                        Some(format!("{:02}", end_date_parsed.day())),
                        move |day| {
                            let parts: Vec<&str> = end_date_str2_day.split('-').collect();
                            let year = if parts.len() > 0 { parts[0] } else { "2025" };
                            let month = if parts.len() > 1 { parts[1] } else { "01" };
                            Message::UpdateEventEndDate(format!("{}-{}-{}", year, month, day))
                        }
                    )
                    .padding(8)
                    .width(Length::FillPortion(1)),
                    text_input("HH:MM", &state.end_time)
                        .on_input(Message::UpdateEventEndTime)
                        .id(Id::new("end_time"))
                        .padding(8)
                        .width(Length::FillPortion(1)),
                ]
                .spacing(4)
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
        "Daily".to_string(),
        "Weekly".to_string(),
        "Fortnightly".to_string(),
        "Monthly".to_string(),
        "Yearly".to_string(),
    ];
    
    content = content.push(
        column![
            text("Recurrence").size(12),
            pick_list(
                recurrence_options,
                Some(recurrence_to_display(&state.recurrence)),
                Message::UpdateEventRecurrence
            )
            .padding(8)
        ]
        .spacing(4)
    );
    
    // Show day of week selection for Weekly recurrence
    if state.recurrence.contains("FREQ=WEEKLY") {
        let days = vec![
            ("Sunday", "SU"),
            ("Monday", "MO"),
            ("Tuesday", "TU"),
            ("Wednesday", "WE"),
            ("Thursday", "TH"),
            ("Friday", "FR"),
            ("Saturday", "SA"),
        ];
        
        let mut day_checkboxes = column![
            text("Repeat on:").size(14).style(Color::from_rgb(0.0, 0.0, 0.0)),
        ].spacing(6);
        
        for (day_name, day_code) in days {
            let is_checked = state.recurrence_days.contains(&day_code.to_string());
            let day_code_string = day_code.to_string();
            
            day_checkboxes = day_checkboxes.push(
                checkbox(day_name, is_checked)
                    .on_toggle(move |_| Message::ToggleRecurrenceDay(day_code_string.clone()))
                    .size(20)
            );
        }
        
        content = content.push(
            container(day_checkboxes)
                .padding(12)
                .style(|_theme: &Theme| {
                    container::Appearance {
                        background: Some(iced::Background::Color(Color::WHITE)),
                        border: iced::Border {
                            color: Color::from_rgb(0.6, 0.6, 0.6),
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                })
        );
    }
    
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
    
    iced::Color::from_rgb(0.23, 0.51, 0.96)
}

/// Extract BYDAY values from an RRULE string
fn extract_byday_from_rrule(rrule: &str) -> Vec<String> {
    for part in rrule.split(';') {
        if part.starts_with("BYDAY=") {
            let days_str = part.trim_start_matches("BYDAY=");
            return days_str.split(',').map(|s| s.to_string()).collect();
        }
    }
    Vec::new()
}

/// Convert RRULE format to display label
fn recurrence_to_display(rrule: &str) -> String {
    match rrule {
        "FREQ=DAILY" => "Daily".to_string(),
        "FREQ=WEEKLY" => "Weekly".to_string(),
        "FREQ=WEEKLY;INTERVAL=2" => "Fortnightly".to_string(),
        "FREQ=MONTHLY" => "Monthly".to_string(),
        "FREQ=YEARLY" => "Yearly".to_string(),
        _ => "None".to_string(),
    }
}

/// Convert display label to RRULE format
pub fn display_to_recurrence(display: &str) -> String {
    match display {
        "Daily" => "FREQ=DAILY".to_string(),
        "Weekly" => "FREQ=WEEKLY".to_string(),
        "Fortnightly" => "FREQ=WEEKLY;INTERVAL=2".to_string(),
        "Monthly" => "FREQ=MONTHLY".to_string(),
        "Yearly" => "FREQ=YEARLY".to_string(),
        _ => "None".to_string(),
    }
}
