use std::sync::{Arc, Mutex};

use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use iced::{Color, Command, Element, Theme};

use crate::models::event::Event;
use crate::services::database::Database;
use crate::ui::dialogs::EventDialogState;
use crate::ui::messages::Message;
use crate::ui::theme::CalendarTheme;
use crate::ui::view_type::ViewType;
use crate::ui::{utils, views};

/// Main Calendar Application state shared across submodules
pub struct CalendarApp {
    pub(crate) theme: Theme,
    pub(crate) calendar_theme: CalendarTheme,
    pub(crate) theme_name: String,
    pub(crate) available_themes: Vec<String>,
    pub(crate) show_my_day: bool,
    pub(crate) my_day_position_right: bool,
    pub(crate) show_ribbon: bool,
    pub(crate) current_view: ViewType,
    pub(crate) db: Arc<Mutex<Database>>,
    pub(crate) show_settings_dialog: bool,
    pub(crate) time_format: String,
    pub(crate) first_day_of_week: u8,
    pub(crate) first_day_of_work_week: u8,
    pub(crate) last_day_of_work_week: u8,
    pub(crate) date_format: String,
    pub(crate) current_date: NaiveDate,
    pub(crate) show_date_picker: bool,
    pub(crate) show_theme_picker: bool,
    pub(crate) show_theme_manager: bool,
    pub(crate) time_slot_interval: u32,
    pub(crate) default_event_start_time: String,
    pub(crate) show_create_theme: bool,
    pub(crate) is_editing_theme: bool,
    pub(crate) editing_theme_original_name: String,
    pub(crate) creating_theme_name: String,
    pub(crate) creating_base_theme: String,
    pub(crate) creating_theme: Option<CalendarTheme>,
    pub(crate) show_color_picker: bool,
    pub(crate) color_picker_field: String,
    pub(crate) color_picker_color: Color,
    pub(crate) show_event_dialog: bool,
    pub(crate) event_dialog_state: Option<EventDialogState>,
    pub(crate) events: Vec<Event>,
}

impl CalendarApp {
    pub(crate) fn create(db_path: String) -> (Self, Command<Message>) {
        let init_data = utils::initialize_app(&db_path);

        let mut app = Self {
            theme: init_data.theme,
            calendar_theme: init_data.calendar_theme,
            theme_name: init_data.theme_name,
            available_themes: init_data.available_themes,
            show_my_day: init_data.show_my_day,
            my_day_position_right: init_data.my_day_position_right,
            show_ribbon: init_data.show_ribbon,
            current_view: init_data.current_view,
            db: init_data.db,
            show_settings_dialog: false,
            time_format: init_data.time_format,
            first_day_of_week: init_data.first_day_of_week,
            first_day_of_work_week: init_data.first_day_of_work_week,
            last_day_of_work_week: init_data.last_day_of_work_week,
            date_format: init_data.date_format,
            current_date: init_data.current_date,
            show_date_picker: false,
            show_theme_picker: false,
            show_theme_manager: false,
            time_slot_interval: init_data.time_slot_interval,
            default_event_start_time: init_data.default_event_start_time,
            show_create_theme: false,
            is_editing_theme: false,
            editing_theme_original_name: String::new(),
            creating_theme_name: String::new(),
            creating_base_theme: "Light".to_string(),
            creating_theme: None,
            show_color_picker: false,
            color_picker_field: String::new(),
            color_picker_color: Color::BLACK,
            show_event_dialog: false,
            event_dialog_state: None,
            events: Vec::new(),
        };

        app.load_events();

        (app, Command::none())
    }

    pub(crate) fn load_events(&mut self) {
        use crate::services::event::EventService;

        let (start, end) = match self.current_view {
            ViewType::Day => {
                let start = self
                    .current_date
                    .and_hms_opt(0, 0, 0)
                    .and_then(|naive| Local.from_local_datetime(&naive).single());
                let end = self
                    .current_date
                    .and_hms_opt(23, 59, 59)
                    .and_then(|naive| Local.from_local_datetime(&naive).single());
                (start, end)
            }
            ViewType::WorkWeek | ViewType::Week => {
                let weekday = self.current_date.weekday().num_days_from_sunday();
                let days_from_start = (weekday as i64 - self.first_day_of_week as i64 + 7) % 7;
                let week_start = self.current_date - Duration::days(days_from_start);

                let days_in_view = if matches!(self.current_view, ViewType::WorkWeek) {
                    5
                } else {
                    7
                };
                let week_end = week_start + Duration::days(days_in_view - 1);

                let start = week_start
                    .and_hms_opt(0, 0, 0)
                    .and_then(|naive| Local.from_local_datetime(&naive).single());
                let end = week_end
                    .and_hms_opt(23, 59, 59)
                    .and_then(|naive| Local.from_local_datetime(&naive).single());
                (start, end)
            }
            ViewType::Month => {
                let first_day = NaiveDate::from_ymd_opt(
                    self.current_date.year(),
                    self.current_date.month(),
                    1,
                )
                .unwrap();
                let last_day = if self.current_date.month() == 12 {
                    NaiveDate::from_ymd_opt(self.current_date.year() + 1, 1, 1)
                        .and_then(|d| d.pred_opt())
                        .unwrap()
                } else {
                    NaiveDate::from_ymd_opt(
                        self.current_date.year(),
                        self.current_date.month() + 1,
                        1,
                    )
                    .and_then(|d| d.pred_opt())
                    .unwrap()
                };

                let start = first_day
                    .and_hms_opt(0, 0, 0)
                    .and_then(|naive| Local.from_local_datetime(&naive).single());
                let end = last_day
                    .and_hms_opt(23, 59, 59)
                    .and_then(|naive| Local.from_local_datetime(&naive).single());
                (start, end)
            }
            ViewType::Quarter => {
                let quarter_start_month = ((self.current_date.month() - 1) / 3) * 3 + 1;
                let first_day = NaiveDate::from_ymd_opt(self.current_date.year(), quarter_start_month, 1)
                    .unwrap();

                let quarter_end_month = quarter_start_month + 2;
                let last_day = if quarter_end_month == 12 {
                    NaiveDate::from_ymd_opt(self.current_date.year() + 1, 1, 1)
                        .and_then(|d| d.pred_opt())
                        .unwrap()
                } else {
                    NaiveDate::from_ymd_opt(self.current_date.year(), quarter_end_month + 1, 1)
                        .and_then(|d| d.pred_opt())
                        .unwrap()
                };

                let start = first_day
                    .and_hms_opt(0, 0, 0)
                    .and_then(|naive| Local.from_local_datetime(&naive).single());
                let end = last_day
                    .and_hms_opt(23, 59, 59)
                    .and_then(|naive| Local.from_local_datetime(&naive).single());
                (start, end)
            }
        };

        if let (Some(start), Some(end)) = (start, end) {
            if let Ok(db) = self.db.lock() {
                let event_service = EventService::new(db.connection());
                if let Ok(events) = event_service.expand_recurring_events(start, end) {
                    self.events = events;
                } else {
                    self.events.clear();
                }
            }
        } else {
            self.events.clear();
        }
    }

    pub(crate) fn save_settings(&self) {
        utils::save_settings(
            &self.db,
            &self.theme_name,
            self.show_my_day,
            self.my_day_position_right,
            self.show_ribbon,
            self.current_view,
            &self.time_format,
            self.first_day_of_week,
            self.first_day_of_work_week,
            self.last_day_of_work_week,
            &self.date_format,
            self.time_slot_interval,
            &self.default_event_start_time,
        );
    }

    pub(crate) fn create_calendar_view(&self) -> Element<Message> {
        match self.current_view {
            ViewType::Month => views::create_month_view(self.current_date, &self.calendar_theme),
            ViewType::Day => views::create_day_view(
                self.current_date,
                &self.calendar_theme,
                &self.time_format,
                self.time_slot_interval,
                &self.events,
            ),
            ViewType::Week => views::create_week_view(
                self.current_date,
                &self.calendar_theme,
                &self.time_format,
                self.time_slot_interval,
                self.first_day_of_week,
                &self.events,
            ),
            ViewType::WorkWeek => views::create_workweek_view(
                self.current_date,
                &self.calendar_theme,
                &self.time_format,
                self.time_slot_interval,
                self.first_day_of_week,
                &self.events,
            ),
            ViewType::Quarter => views::create_quarter_view(self.current_date, &self.calendar_theme),
        }
    }
}
