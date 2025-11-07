// Main Calendar Application
// Core iced Application implementation

use iced::{Application, Command, Element, Theme, Color};
use crate::services::database::Database;
use crate::services::theme::ThemeService;
use crate::ui::theme::CalendarTheme;
use crate::ui::messages::Message;
use crate::ui::view_type::ViewType;
use crate::ui::helpers;
use crate::ui::views;
use crate::ui::components;
use crate::ui::utils;
use std::sync::{Arc, Mutex};
use chrono::{Local, Datelike, NaiveDate, Duration};

/// Helper function to parse hex color string to iced Color
fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    
    Some(Color::from_rgb(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
    ))
}

/// Main Calendar Application
pub struct CalendarApp {
    /// Current theme (Light or Dark)
    theme: Theme,
    /// Custom calendar theme with colors
    calendar_theme: CalendarTheme,
    /// Current theme name
    theme_name: String,
    /// Available theme names
    available_themes: Vec<String>,
    /// Show/hide My Day panel
    show_my_day: bool,
    /// My Day panel position (true = right, false = left)
    my_day_position_right: bool,
    /// Show/hide multi-day ribbon
    show_ribbon: bool,
    /// Current view type
    current_view: ViewType,
    /// Database connection (wrapped in Arc<Mutex<>> for thread safety)
    db: Arc<Mutex<Database>>,
    /// Show/hide settings dialog
    show_settings_dialog: bool,
    /// Time format (12h or 24h)
    time_format: String,
    /// First day of week (0=Sunday, 1=Monday, etc.)
    first_day_of_week: u8,
    /// Date format
    date_format: String,
    /// Currently displayed date (for navigation)
    current_date: NaiveDate,
    /// Show/hide month/year picker
    show_date_picker: bool,
    /// Show/hide theme picker
    show_theme_picker: bool,
    /// Show/hide theme management dialog
    show_theme_manager: bool,
    /// Time slot interval in minutes (15, 30, 45, or 60)
    time_slot_interval: u32,
    /// Show theme creation dialog
    show_create_theme: bool,
    /// Whether we're editing an existing theme (vs creating new)
    is_editing_theme: bool,
    /// Original theme name when editing (for updating vs inserting)
    editing_theme_original_name: String,
    /// Name for the theme being created
    creating_theme_name: String,
    /// Base theme name to copy from
    creating_base_theme: String,
    /// Theme being created/edited
    creating_theme: Option<CalendarTheme>,
    /// Show color picker dialog
    show_color_picker: bool,
    /// Field name being edited in color picker
    color_picker_field: String,
    /// Current color in the picker
    color_picker_color: Color,
    /// Show event creation/edit dialog
    show_event_dialog: bool,
    /// Event dialog state
    event_dialog_state: Option<crate::ui::dialogs::EventDialogState>,
}

impl Application for CalendarApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = String;

    fn new(db_path: Self::Flags) -> (Self, Command<Self::Message>) {
        let init_data = utils::initialize_app(&db_path);

        (
            Self {
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
                date_format: init_data.date_format,
                current_date: init_data.current_date,
                show_date_picker: false,
                show_theme_picker: false,
                show_theme_manager: false,
                time_slot_interval: init_data.time_slot_interval,
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
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Rust Calendar")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::ToggleTheme => {
                // If only 2 themes, toggle between them. Otherwise, show picker
                if self.available_themes.len() <= 2 {
                    // Cycle to next theme in the list
                    let current_idx = self.available_themes.iter()
                        .position(|t| t == &self.theme_name)
                        .unwrap_or(0);
                    let next_idx = (current_idx + 1) % self.available_themes.len();
                    let next_theme_name = self.available_themes[next_idx].clone();
                    
                    // Load the theme (scope to release lock before save_settings)
                    {
                        let db = match self.db.lock() {
                            Ok(db) => db,
                            Err(_) => return Command::none(),
                        };
                        
                        let theme_service = ThemeService::new(&db);
                        if let Ok(calendar_theme) = theme_service.get_theme(&next_theme_name) {
                            self.theme_name = next_theme_name;
                            self.theme = calendar_theme.base.clone();
                            self.calendar_theme = calendar_theme;
                        }
                    } // Lock is released here
                    
                    self.save_settings();
                } else {
                    // Show theme picker for 3+ themes
                    self.show_theme_picker = true;
                }
            }
            Message::ShowThemePicker => {
                self.show_theme_picker = true;
            }
            Message::CloseThemePicker => {
                self.show_theme_picker = false;
            }
            Message::SelectTheme(theme_name) => {
                // Load the selected theme
                {
                    let db = match self.db.lock() {
                        Ok(db) => db,
                        Err(_) => return Command::none(),
                    };
                    
                    let theme_service = ThemeService::new(&db);
                    if let Ok(calendar_theme) = theme_service.get_theme(&theme_name) {
                        self.theme_name = theme_name;
                        self.theme = calendar_theme.base.clone();
                        self.calendar_theme = calendar_theme;
                    }
                } // Lock is released here
                
                self.show_theme_picker = false;
                self.save_settings();
            }
            Message::ToggleMyDay => {
                self.show_my_day = !self.show_my_day;
                self.save_settings();
            }
            Message::ToggleRibbon => {
                self.show_ribbon = !self.show_ribbon;
                self.save_settings();
            }
            Message::SwitchView(view_type) => {
                self.current_view = view_type;
                self.save_settings();
            }
            Message::OpenSettings => {
                self.show_settings_dialog = true;
            }
            Message::CloseSettings => {
                self.show_settings_dialog = false;
            }
            Message::UpdateTheme(theme_name) => {
                // Load the selected theme from database
                let db = match self.db.lock() {
                    Ok(db) => db,
                    Err(_) => return Command::none(),
                };
                
                let theme_service = ThemeService::new(&db);
                if let Ok(calendar_theme) = theme_service.get_theme(&theme_name) {
                    self.theme_name = theme_name;
                    self.theme = calendar_theme.base.clone();
                    self.calendar_theme = calendar_theme;
                }
            }
            Message::UpdateView(view_str) => {
                self.current_view = match view_str.as_str() {
                    "Day" => ViewType::Day,
                    "WorkWeek" => ViewType::WorkWeek,
                    "Week" => ViewType::Week,
                    "Month" => ViewType::Month,
                    "Quarter" => ViewType::Quarter,
                    _ => ViewType::Month,
                };
            }
            Message::UpdateShowMyDay(show) => {
                self.show_my_day = show;
            }
            Message::UpdateMyDayPosition(position) => {
                self.my_day_position_right = position == "Right";
            }
            Message::UpdateShowRibbon(show) => {
                self.show_ribbon = show;
            }
            Message::UpdateTimeFormat(format) => {
                self.time_format = format;
            }
            Message::UpdateFirstDayOfWeek(day) => {
                if let Ok(day_num) = day.parse::<u8>() {
                    if day_num <= 6 {
                        self.first_day_of_week = day_num;
                    }
                }
            }
            Message::SaveSettings => {
                self.save_settings();
                self.show_settings_dialog = false;
            }
            Message::OpenThemeManager => {
                self.show_settings_dialog = false;  // Close settings first
                self.show_theme_manager = true;
            }
            Message::CloseThemeManager => {
                self.show_theme_manager = false;
                self.show_settings_dialog = true;  // Reopen settings when closing theme manager
            }
            Message::DeleteTheme(theme_name) => {
                // Don't allow deletion of built-in themes
                if theme_name == "Light" || theme_name == "Dark" {
                    return Command::none();
                }
                
                // Don't allow deletion if it's the only theme left
                if self.available_themes.len() <= 1 {
                    return Command::none();
                }
                
                let was_active = self.theme_name == theme_name;
                
                // Delete the theme from database and reload themes
                let db = match self.db.lock() {
                    Ok(db) => db,
                    Err(_) => return Command::none(),
                };
                
                let theme_service = ThemeService::new(&db);
                let _ = theme_service.delete_theme(&theme_name);
                
                // Reload available themes
                if let Ok(themes) = theme_service.list_themes() {
                    self.available_themes = themes;
                }
                
                // If deleted theme was active, switch to Light
                if was_active {
                    if let Ok(calendar_theme) = theme_service.get_theme("Light") {
                        self.theme_name = "Light".to_string();
                        self.theme = calendar_theme.base.clone();
                        self.calendar_theme = calendar_theme;
                        
                        // Drop the lock before saving settings (which may need it)
                        drop(db);
                        self.save_settings();
                    }
                }
            }
            Message::StartCreateTheme => {
                self.show_theme_manager = false;
                self.show_create_theme = true;
                self.is_editing_theme = false;
                self.creating_theme_name = String::new();
                self.creating_base_theme = "Light".to_string();
                
                // Load the Light theme as default starting point
                let db = match self.db.lock() {
                    Ok(db) => db,
                    Err(_) => return Command::none(),
                };
                
                let theme_service = ThemeService::new(&db);
                if let Ok(theme) = theme_service.get_theme("Light") {
                    self.creating_theme = Some(theme);
                }
            }
            Message::StartEditTheme(theme_name) => {
                // Don't allow editing built-in themes
                if theme_name == "Light" || theme_name == "Dark" {
                    return Command::none();
                }
                
                self.show_theme_manager = false;
                self.show_create_theme = true;
                self.is_editing_theme = true;
                self.editing_theme_original_name = theme_name.clone();
                self.creating_theme_name = theme_name.clone();
                
                // Load the theme to edit
                let db = match self.db.lock() {
                    Ok(db) => db,
                    Err(_) => return Command::none(),
                };
                
                let theme_service = ThemeService::new(&db);
                if let Ok(theme) = theme_service.get_theme(&theme_name) {
                    self.creating_theme = Some(theme);
                }
            }
            Message::CancelCreateTheme => {
                self.show_create_theme = false;
                self.show_theme_manager = true;
                self.is_editing_theme = false;
                self.creating_theme = None;
            }
            Message::UpdateThemeName(name) => {
                self.creating_theme_name = name;
            }
            Message::SelectBaseTheme(base_theme_name) => {
                self.creating_base_theme = base_theme_name.clone();
                
                // Load the selected base theme
                let db = match self.db.lock() {
                    Ok(db) => db,
                    Err(_) => return Command::none(),
                };
                
                let theme_service = ThemeService::new(&db);
                if let Ok(theme) = theme_service.get_theme(&base_theme_name) {
                    self.creating_theme = Some(theme);
                }
            }
            Message::OpenColorPicker(field_name) => {
                // Get the current color for the field
                if let Some(theme) = &self.creating_theme {
                    let color = match field_name.as_str() {
                        "app_background" => theme.app_background,
                        "calendar_background" => theme.calendar_background,
                        "weekend_background" => theme.weekend_background,
                        "today_background" => theme.today_background,
                        "today_border" => theme.today_border,
                        "day_background" => theme.day_background,
                        "day_border" => theme.day_border,
                        "text_primary" => theme.text_primary,
                        "text_secondary" => theme.text_secondary,
                        _ => Color::BLACK,
                    };
                    
                    self.show_color_picker = true;
                    self.color_picker_field = field_name;
                    self.color_picker_color = color;
                }
            }
            Message::CancelColorPicker => {
                self.show_color_picker = false;
            }
            Message::UpdateColorSlider(field_name, channel, value) => {
                // Update the color in real-time as sliders move
                if let Some(theme) = &mut self.creating_theme {
                    let current_color = match field_name.as_str() {
                        "app_background" => &mut theme.app_background,
                        "calendar_background" => &mut theme.calendar_background,
                        "weekend_background" => &mut theme.weekend_background,
                        "today_background" => &mut theme.today_background,
                        "today_border" => &mut theme.today_border,
                        "day_background" => &mut theme.day_background,
                        "day_border" => &mut theme.day_border,
                        "text_primary" => &mut theme.text_primary,
                        "text_secondary" => &mut theme.text_secondary,
                        _ => return Command::none(),
                    };

                    let val = value as f32 / 255.0;
                    match channel.as_str() {
                        "r" => *current_color = Color::from_rgb(val, current_color.g, current_color.b),
                        "g" => *current_color = Color::from_rgb(current_color.r, val, current_color.b),
                        "b" => *current_color = Color::from_rgb(current_color.r, current_color.g, val),
                        _ => {}
                    }
                    
                    // Update the color picker color to show the new value
                    self.color_picker_color = *current_color;
                }
            }
            Message::SubmitColor(_color) => {
                // Color is already updated via UpdateColorSlider messages
                // Just close the picker
                self.show_color_picker = false;
            }
            Message::UpdateHexInput(field_name, hex_input) => {
                // Parse and update color from hex input
                if let Some(theme) = &mut self.creating_theme {
                    // Remove any # prefix and try to parse
                    let hex = hex_input.trim_start_matches('#');
                    
                    // Only parse if we have exactly 6 hex characters
                    if hex.len() == 6 {
                        if let (Ok(r), Ok(g), Ok(b)) = (
                            u8::from_str_radix(&hex[0..2], 16),
                            u8::from_str_radix(&hex[2..4], 16),
                            u8::from_str_radix(&hex[4..6], 16),
                        ) {
                            let color = Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
                            
                            match field_name.as_str() {
                                "app_background" => theme.app_background = color,
                                "calendar_background" => theme.calendar_background = color,
                                "weekend_background" => theme.weekend_background = color,
                                "today_background" => theme.today_background = color,
                                "today_border" => theme.today_border = color,
                                "day_background" => theme.day_background = color,
                                "day_border" => theme.day_border = color,
                                "text_primary" => theme.text_primary = color,
                                "text_secondary" => theme.text_secondary = color,
                                _ => {}
                            }
                            
                            // Update the color picker color to show the new value
                            self.color_picker_color = color;
                        }
                    }
                }
            }
            Message::UpdateRGBInput(field_name, channel, value_str) => {
                // Parse and update color from RGB input
                if let Some(theme) = &mut self.creating_theme {
                    if let Ok(value) = value_str.parse::<u8>() {
                        let current_color = match field_name.as_str() {
                            "app_background" => &mut theme.app_background,
                            "calendar_background" => &mut theme.calendar_background,
                            "weekend_background" => &mut theme.weekend_background,
                            "today_background" => &mut theme.today_background,
                            "today_border" => &mut theme.today_border,
                            "day_background" => &mut theme.day_background,
                            "day_border" => &mut theme.day_border,
                            "text_primary" => &mut theme.text_primary,
                            "text_secondary" => &mut theme.text_secondary,
                            _ => return Command::none(),
                        };

                        let val = value as f32 / 255.0;
                        match channel.as_str() {
                            "r" => *current_color = Color::from_rgb(val, current_color.g, current_color.b),
                            "g" => *current_color = Color::from_rgb(current_color.r, val, current_color.b),
                            "b" => *current_color = Color::from_rgb(current_color.r, current_color.g, val),
                            _ => {}
                        }
                        
                        // Update the color picker color to show the new value
                        self.color_picker_color = *current_color;
                    }
                }
            }
            Message::ColorPickerSubmit(field_name, color) => {
                // Update color from iced_aw ColorPicker
                if let Some(theme) = &mut self.creating_theme {
                    match field_name.as_str() {
                        "app_background" => theme.app_background = color,
                        "calendar_background" => theme.calendar_background = color,
                        "weekend_background" => theme.weekend_background = color,
                        "today_background" => theme.today_background = color,
                        "today_border" => theme.today_border = color,
                        "day_background" => theme.day_background = color,
                        "day_border" => theme.day_border = color,
                        "text_primary" => theme.text_primary = color,
                        "text_secondary" => theme.text_secondary = color,
                        _ => {}
                    }
                    
                    // Update the color picker color to show the new value
                    self.color_picker_color = color;
                }
            }
            Message::UpdateThemeColor(field_name, hex_color) => {
                // Parse hex color and update the specific field in creating_theme
                if let Some(theme) = &mut self.creating_theme {
                    // Parse hex color (format: #RRGGBB)
                    if let Some(color) = parse_hex_color(&hex_color) {
                        match field_name.as_str() {
                            "app_background" => theme.app_background = color,
                            "calendar_background" => theme.calendar_background = color,
                            "weekend_background" => theme.weekend_background = color,
                            "today_background" => theme.today_background = color,
                            "today_border" => theme.today_border = color,
                            "day_background" => theme.day_background = color,
                            "day_border" => theme.day_border = color,
                            "text_primary" => theme.text_primary = color,
                            "text_secondary" => theme.text_secondary = color,
                            _ => {}
                        }
                    }
                }
            }
            Message::SaveCustomTheme => {
                if self.creating_theme_name.trim().is_empty() {
                    return Command::none(); // Don't save without a name
                }
                
                if let Some(theme) = &self.creating_theme {
                    let db = match self.db.lock() {
                        Ok(db) => db,
                        Err(_) => return Command::none(),
                    };
                    
                    let theme_service = ThemeService::new(&db);
                    
                    // If editing and name changed, delete the old theme first
                    if self.is_editing_theme && self.creating_theme_name != self.editing_theme_original_name {
                        let _ = theme_service.delete_theme(&self.editing_theme_original_name);
                    }
                    
                    if theme_service.save_theme(theme, &self.creating_theme_name).is_ok() {
                        // Reload available themes
                        if let Ok(themes) = theme_service.list_themes() {
                            self.available_themes = themes;
                        }
                        
                        // If the edited/created theme is now active, update the active theme
                        if self.is_editing_theme && self.editing_theme_original_name == self.theme_name {
                            self.theme_name = self.creating_theme_name.clone();
                            self.theme = theme.base.clone();
                            self.calendar_theme = theme.clone();
                            
                            drop(db);
                            self.save_settings();
                        } else {
                            drop(db);
                        }
                        
                        // Close create dialog and reopen theme manager
                        self.show_create_theme = false;
                        self.show_theme_manager = true;
                        self.is_editing_theme = false;
                        self.creating_theme = None;
                    }
                }
            }
            Message::Exit => {
                std::process::exit(0);
            }
            Message::PreviousMonth => {
                self.current_date = self.current_date
                    .with_day(1)
                    .unwrap()
                    .checked_sub_signed(Duration::days(1))
                    .unwrap()
                    .with_day(1)
                    .unwrap();
            }
            Message::NextMonth => {
                // Go to first day of next month
                let next_month = if self.current_date.month() == 12 {
                    NaiveDate::from_ymd_opt(self.current_date.year() + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(self.current_date.year(), self.current_date.month() + 1, 1).unwrap()
                };
                self.current_date = next_month;
            }
            Message::PreviousDay => {
                if let Some(prev_day) = self.current_date.checked_sub_signed(Duration::days(1)) {
                    self.current_date = prev_day;
                }
            }
            Message::NextDay => {
                if let Some(next_day) = self.current_date.checked_add_signed(Duration::days(1)) {
                    self.current_date = next_day;
                }
            }
            Message::PreviousWeek => {
                if let Some(prev_week) = self.current_date.checked_sub_signed(Duration::days(7)) {
                    self.current_date = prev_week;
                }
            }
            Message::NextWeek => {
                if let Some(next_week) = self.current_date.checked_add_signed(Duration::days(7)) {
                    self.current_date = next_week;
                }
            }
            Message::PreviousQuarter => {
                // Go back 3 months
                let current_month = self.current_date.month();
                let current_year = self.current_date.year();
                
                let new_month = if current_month <= 3 {
                    current_month + 9
                } else {
                    current_month - 3
                };
                
                let new_year = if current_month <= 3 {
                    current_year - 1
                } else {
                    current_year
                };
                
                if let Some(new_date) = NaiveDate::from_ymd_opt(new_year, new_month, 1) {
                    self.current_date = new_date;
                }
            }
            Message::NextQuarter => {
                // Go forward 3 months
                let current_month = self.current_date.month();
                let current_year = self.current_date.year();
                
                let new_month = if current_month >= 10 {
                    current_month - 9
                } else {
                    current_month + 3
                };
                
                let new_year = if current_month >= 10 {
                    current_year + 1
                } else {
                    current_year
                };
                
                if let Some(new_date) = NaiveDate::from_ymd_opt(new_year, new_month, 1) {
                    self.current_date = new_date;
                }
            }
            Message::GoToToday => {
                self.current_date = Local::now().naive_local().date();
            }
            Message::ToggleDatePicker => {
                self.show_date_picker = !self.show_date_picker;
            }
            Message::ChangeMonth(month) => {
                if let Some(new_date) = NaiveDate::from_ymd_opt(self.current_date.year(), month, 1) {
                    self.current_date = new_date;
                }
                self.show_date_picker = false;
            }
            Message::ChangeYear(year) => {
                if let Some(new_date) = NaiveDate::from_ymd_opt(year, self.current_date.month(), 1) {
                    self.current_date = new_date;
                }
                self.show_date_picker = false;
            }
            Message::GoToDateInWeekView(year, month, day) => {
                // Navigate to the specified date and switch to Week view
                if let Some(new_date) = NaiveDate::from_ymd_opt(year, month, day) {
                    self.current_date = new_date;
                    self.current_view = ViewType::Week;
                }
            }
            Message::UpdateTimeSlotInterval(interval) => {
                // Validate interval
                if [15, 30, 45, 60].contains(&interval) {
                    self.time_slot_interval = interval;
                    self.save_settings();
                }
            }
            
            // Event dialog messages
            Message::OpenEventDialog => {
                self.event_dialog_state = Some(crate::ui::dialogs::EventDialogState::new());
                self.show_event_dialog = true;
            }
            Message::EditEvent(event_id) => {
                // Load event from database and populate dialog
                if let Ok(db) = self.db.lock() {
                    let service = crate::services::event::EventService::new(db.connection());
                    if let Ok(Some(event)) = service.get(event_id) {
                        self.event_dialog_state = Some(crate::ui::dialogs::EventDialogState::from_event(&event));
                        self.show_event_dialog = true;
                    }
                }
            }
            Message::CloseEventDialog => {
                self.show_event_dialog = false;
                self.event_dialog_state = None;
            }
            Message::UpdateEventTitle(title) => {
                if let Some(state) = &mut self.event_dialog_state {
                    state.title = title;
                    state.validation_error = None;
                }
            }
            Message::UpdateEventDescription(desc) => {
                if let Some(state) = &mut self.event_dialog_state {
                    state.description = desc;
                }
            }
            Message::UpdateEventLocation(location) => {
                if let Some(state) = &mut self.event_dialog_state {
                    state.location = location;
                }
            }
            Message::UpdateEventStartDate(date) => {
                if let Some(state) = &mut self.event_dialog_state {
                    state.start_date = date;
                    state.validation_error = None;
                }
            }
            Message::UpdateEventStartTime(time) => {
                if let Some(state) = &mut self.event_dialog_state {
                    state.start_time = time;
                    state.validation_error = None;
                }
            }
            Message::UpdateEventEndDate(date) => {
                if let Some(state) = &mut self.event_dialog_state {
                    state.end_date = date;
                    state.validation_error = None;
                }
            }
            Message::UpdateEventEndTime(time) => {
                if let Some(state) = &mut self.event_dialog_state {
                    state.end_time = time;
                    state.validation_error = None;
                }
            }
            Message::ToggleEventAllDay(all_day) => {
                if let Some(state) = &mut self.event_dialog_state {
                    state.all_day = all_day;
                }
            }
            Message::UpdateEventCategory(category) => {
                if let Some(state) = &mut self.event_dialog_state {
                    state.category = category;
                }
            }
            Message::UpdateEventColor(color) => {
                if let Some(state) = &mut self.event_dialog_state {
                    state.color = color;
                }
            }
            Message::UpdateEventRecurrence(recurrence) => {
                if let Some(state) = &mut self.event_dialog_state {
                    state.recurrence = recurrence;
                }
            }
            Message::SaveEvent => {
                if let Some(state) = &self.event_dialog_state {
                    match state.to_event() {
                        Ok(event) => {
                            // Save to database
                            if let Ok(db) = self.db.lock() {
                                let service = crate::services::event::EventService::new(db.connection());
                                
                                let result = if state.is_editing {
                                    service.update(&event)
                                } else {
                                    service.create(event).map(|_| ())
                                };
                                
                                match result {
                                    Ok(_) => {
                                        // Close dialog on success
                                        self.show_event_dialog = false;
                                        self.event_dialog_state = None;
                                    }
                                    Err(e) => {
                                        // Show error
                                        if let Some(dialog_state) = &mut self.event_dialog_state {
                                            dialog_state.validation_error = Some(format!("Failed to save event: {}", e));
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // Show validation error
                            if let Some(dialog_state) = &mut self.event_dialog_state {
                                dialog_state.validation_error = Some(e);
                            }
                        }
                    }
                }
            }
            Message::DeleteEvent(event_id) => {
                // Direct delete without confirmation (if we want confirmation, we'd use ConfirmDeleteEvent)
                if let Ok(db) = self.db.lock() {
                    let service = crate::services::event::EventService::new(db.connection());
                    let _ = service.delete(event_id);
                }
                self.show_event_dialog = false;
                self.event_dialog_state = None;
            }
            Message::ConfirmDeleteEvent(event_id) => {
                // For now, just delete directly. Could add a confirmation dialog later
                if let Ok(db) = self.db.lock() {
                    let service = crate::services::event::EventService::new(db.connection());
                    let _ = service.delete(event_id);
                }
                self.show_event_dialog = false;
                self.event_dialog_state = None;
            }
            Message::CancelDeleteEvent => {
                // Just dismiss any delete confirmation
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
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
            self.time_slot_interval,
            self.show_color_picker,
            self.color_picker_color,
            &self.color_picker_field,
            self.show_event_dialog,
            self.event_dialog_state.as_ref(),
        )
    }

    fn theme(&self) -> Self::Theme {
        self.theme.clone()
    }
}

impl CalendarApp {
    /// Save current settings to database
    fn save_settings(&self) {
        utils::save_settings(
            &self.db,
            &self.theme_name,
            self.show_my_day,
            self.my_day_position_right,
            self.show_ribbon,
            self.current_view,
            &self.time_format,
            self.first_day_of_week,
            &self.date_format,
            self.time_slot_interval,
        );
    }

    /// Create the main calendar view
    fn create_calendar_view(&self) -> Element<Message> {
        match self.current_view {
            ViewType::Month => views::create_month_view(self.current_date, &self.calendar_theme),
            ViewType::Day => views::create_day_view(
                self.current_date,
                &self.calendar_theme,
                &self.time_format,
                self.time_slot_interval,
            ),
            ViewType::Week => views::create_week_view(
                self.current_date,
                &self.calendar_theme,
                &self.time_format,
                self.time_slot_interval,
                self.first_day_of_week,
            ),
            ViewType::WorkWeek => views::create_workweek_view(
                self.current_date,
                &self.calendar_theme,
                &self.time_format,
                self.time_slot_interval,
                self.first_day_of_week,
            ),
            ViewType::Quarter => views::create_quarter_view(self.current_date, &self.calendar_theme),
        }
    }
}
