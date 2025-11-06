// Main Calendar Application
// Core iced Application implementation

use iced::{
    widget::{button, column, container, row, text, pick_list, checkbox},
    Application, Command, Element, Length, Theme, Border,
    alignment::Horizontal,
};
use iced_aw::menu::{Item, Menu, MenuBar};
use iced_aw::{Modal, Card};
use crate::services::database::Database;
use crate::services::settings::SettingsService;
use crate::services::theme::ThemeService;
use crate::ui::theme::CalendarTheme;
use crate::ui::messages::Message;
use crate::ui::view_type::ViewType;
use crate::ui::helpers;
use crate::ui::dialogs;
use crate::ui::views;
use crate::ui::components;
use std::sync::{Arc, Mutex};
use chrono::{Local, Datelike, NaiveDate, Duration};

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
}

impl Application for CalendarApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = String;

    fn new(db_path: Self::Flags) -> (Self, Command<Self::Message>) {
        // Initialize database
        let db = match Database::new(&db_path) {
            Ok(db) => {
                if let Err(e) = db.initialize_schema() {
                    eprintln!("Warning: Failed to initialize database schema: {}", e);
                }
                db
            }
            Err(e) => {
                eprintln!("Warning: Failed to open database, using defaults: {}", e);
                // Create in-memory database as fallback
                Database::new(":memory:").expect("Failed to create fallback in-memory database")
            }
        };

        // Load settings from database
        let settings_service = SettingsService::new(&db);
        let settings = settings_service.get().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load settings, using defaults: {}", e);
            crate::models::settings::Settings::default()
        });

        // Load available themes
        let theme_service = ThemeService::new(&db);
        let available_themes = theme_service.list_themes().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load themes: {}", e);
            vec!["Light".to_string(), "Dark".to_string()]
        });
        
        // Load the selected theme from database or use default
        let theme_name = settings.theme.clone();
        let calendar_theme = theme_service.get_theme(&theme_name)
            .unwrap_or_else(|_| CalendarTheme::light());
        
        let theme = calendar_theme.base.clone();

        // Parse current view
        let current_view = match settings.current_view.as_str() {
            "Day" => ViewType::Day,
            "WorkWeek" => ViewType::WorkWeek,
            "Week" => ViewType::Week,
            "Quarter" => ViewType::Quarter,
            _ => ViewType::Month,
        };

        (
            Self {
                theme,
                calendar_theme,
                theme_name,
                available_themes,
                show_my_day: settings.show_my_day,
                my_day_position_right: settings.my_day_position_right,
                show_ribbon: settings.show_ribbon,
                current_view,
                db: Arc::new(Mutex::new(db)),
                show_settings_dialog: false,
                time_format: settings.time_format.clone(),
                first_day_of_week: settings.first_day_of_week,
                date_format: settings.date_format.clone(),
                current_date: Local::now().naive_local().date(),
                show_date_picker: false,
                show_theme_picker: false,
                show_theme_manager: false,
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
                
                // Delete the theme from database
                {
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
                }
                
                // If deleted theme was active, switch to Light
                if self.theme_name == theme_name {
                    let db = match self.db.lock() {
                        Ok(db) => db,
                        Err(_) => return Command::none(),
                    };
                    
                    let theme_service = ThemeService::new(&db);
                    if let Ok(calendar_theme) = theme_service.get_theme("Light") {
                        self.theme_name = "Light".to_string();
                        self.theme = calendar_theme.base.clone();
                        self.calendar_theme = calendar_theme;
                        self.save_settings();
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
        }
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        // Main layout structure
        let mut content = column![].spacing(0);

        // Top menu bar
        let menu_bar = components::create_menu_bar(
            self.current_view,
            self.show_my_day,
            self.show_ribbon,
            &self.theme,
        );
        content = content.push(menu_bar);

        // Multi-day ribbon (if visible)
        if self.show_ribbon {
            let ribbon = helpers::create_ribbon();
            content = content.push(ribbon);
        }

        // Main content area: My Day panel + Calendar view
        let main_content = if self.show_my_day {
            if self.my_day_position_right {
                row![
                    self.create_calendar_view(),
                    helpers::create_my_day_panel(),
                ]
                .spacing(2)
            } else {
                row![
                    helpers::create_my_day_panel(),
                    self.create_calendar_view(),
                ]
                .spacing(2)
            }
        } else {
            row![self.create_calendar_view()]
        };
        
        content = content.push(main_content);

        let app_bg = self.calendar_theme.app_background;
        let base_view = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme: &Theme| {
                container::Appearance {
                    background: Some(iced::Background::Color(app_bg)),
                    ..Default::default()
                }
            });

        // Show modal dialog if settings or date picker is open
        if self.show_settings_dialog {
            Modal::new(base_view, Some(dialogs::create_settings_dialog(
                &self.available_themes,
                &self.theme_name,
                self.current_view,
                self.show_my_day,
                self.my_day_position_right,
                self.show_ribbon,
                &self.time_format,
                self.first_day_of_week
            )))
                .backdrop(Message::CloseSettings)
                .into()
        } else if self.show_theme_manager {
            Modal::new(base_view, Some(dialogs::create_theme_manager_dialog(&self.available_themes, &self.theme_name)))
                .backdrop(Message::CloseThemeManager)
                .into()
        } else if self.show_date_picker {
            Modal::new(base_view, Some(dialogs::create_date_picker_dialog(
                self.current_date.year(),
                self.current_date.month()
            )))
                .backdrop(Message::ToggleDatePicker)
                .into()
        } else if self.show_theme_picker {
            Modal::new(base_view, Some(dialogs::create_theme_picker_dialog(&self.available_themes, &self.theme_name)))
                .backdrop(Message::CloseThemePicker)
                .into()
        } else {
            base_view.into()
        }
    }

    fn theme(&self) -> Self::Theme {
        self.theme.clone()
    }
}

impl CalendarApp {
    /// Create the multi-day event ribbon
    /// Create the main calendar view
    fn create_calendar_view(&self) -> Element<Message> {
        match self.current_view {
            ViewType::Month => views::create_month_view(self.current_date, &self.calendar_theme),
            ViewType::Day => helpers::create_placeholder_view("Day View - Coming Soon"),
            ViewType::WorkWeek => helpers::create_placeholder_view("Work Week View - Coming Soon"),
            ViewType::Week => helpers::create_placeholder_view("Week View - Coming Soon"),
            ViewType::Quarter => helpers::create_placeholder_view("Quarter View - Coming Soon"),
        }
    }

    /// Save current settings to database
    fn save_settings(&self) {
        let db = match self.db.lock() {
            Ok(db) => db,
            Err(e) => {
                eprintln!("Error: Failed to acquire database lock: {}", e);
                return;
            }
        };

        let settings_service = SettingsService::new(&db);

        let view_str = match self.current_view {
            ViewType::Day => "Day",
            ViewType::WorkWeek => "WorkWeek",
            ViewType::Week => "Week",
            ViewType::Month => "Month",
            ViewType::Quarter => "Quarter",
        };

        // Load existing settings to preserve other fields
        let mut settings = match settings_service.get() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Warning: Failed to load settings for update: {}", e);
                crate::models::settings::Settings::default()
            }
        };

        // Update with current UI state
        settings.theme = self.theme_name.clone();
        settings.show_my_day = self.show_my_day;
        settings.my_day_position_right = self.my_day_position_right;
        settings.show_ribbon = self.show_ribbon;
        settings.current_view = view_str.to_string();
        settings.time_format = self.time_format.clone();
        settings.first_day_of_week = self.first_day_of_week;
        settings.date_format = self.date_format.clone();

        // Save to database
        if let Err(e) = settings_service.update(&settings) {
            eprintln!("Error: Failed to save settings: {}", e);
        }
    }
}
