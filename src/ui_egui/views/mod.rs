use chrono::{DateTime, Local};

use crate::models::event::Event;

pub mod day_view;
pub mod month_view;
mod palette;
pub mod quarter_view;
pub mod week_view;
pub mod workweek_view;

#[derive(Clone, Debug)]
pub struct CountdownRequest {
    pub event_id: Option<i64>,
    pub title: String,
    pub start_at: DateTime<Local>,
    pub color: Option<String>,
    pub body: Option<String>,
}

impl CountdownRequest {
    pub fn from_event(event: &Event) -> Self {
        Self {
            event_id: event.id,
            title: event.title.clone(),
            start_at: event.start,
            color: event.color.clone(),
            body: event.description.clone(),
        }
    }
}
