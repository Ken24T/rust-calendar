use chrono::Datelike;
use iced::Element;

use crate::ui::{components, messages::Message};

use super::CalendarApp;

impl CalendarApp {
    pub(crate) fn render_view(&self) -> Element<Message> {
        components::build_view(
            self.current_view,
            self.show_my_day,
            self.show_ribbon,
            self.my_day_position_right,
            &self.theme,
            &self.calendar_theme,
            self.create_calendar_view(),
            self.show_settings_dialog,
            self.show_theme_manager,
            self.show_create_theme,
            self.is_editing_theme,
            self.show_date_picker,
            self.show_theme_picker,
            &self.available_themes,
            &self.theme_name,
            &self.creating_theme_name,
            &self.creating_base_theme,
            self.creating_theme.as_ref(),
            self.current_date.year(),
            self.current_date.month(),
            &self.time_format,
            self.first_day_of_week,
            self.first_day_of_work_week,
            self.last_day_of_work_week,
            self.time_slot_interval,
            &self.default_event_start_time,
            self.show_color_picker,
            self.color_picker_color,
            &self.color_picker_field,
            self.show_event_dialog,
            self.event_dialog_state.as_ref(),
        )
    }
}
