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
        let menu_bar = self.create_menu_bar();
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
            Modal::new(base_view, Some(self.create_settings_dialog()))
                .backdrop(Message::CloseSettings)
                .into()
        } else if self.show_theme_manager {
            Modal::new(base_view, Some(self.create_theme_manager_dialog()))
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
    /// Create the top menu bar with standard Windows menu structure
    fn create_menu_bar(&self) -> Element<Message> {
        let file_menu = Item::with_menu(
            button(text("File").size(14)).padding([5, 10]),
            Menu::new(vec![
                Item::new(
                    button(text("Exit").size(13))
                        .on_press(Message::Exit)
                        .padding([8, 20])
                        .width(Length::Fill)
                ),
            ])
            .max_width(180.0)
            .offset(0.0)
            .spacing(0.0)
        );

        let edit_menu = Item::with_menu(
            button(text("Edit").size(14)).padding([5, 10]),
            Menu::new(vec![
                Item::new(
                    button(text("Settings").size(13))
                        .on_press(Message::OpenSettings)
                        .padding([8, 20])
                        .width(Length::Fill)
                ),
            ])
            .max_width(180.0)
            .offset(0.0)
            .spacing(0.0)
        );

        // Views submenu
        let views_submenu = Menu::new(vec![
            Item::new(
                button(
                    text(if self.current_view == ViewType::Day { "✓ Day" } else { "  Day" }).size(13)
                )
                .on_press(Message::SwitchView(ViewType::Day))
                .padding([8, 20])
                .width(Length::Fill)
            ),
            Item::new(
                button(
                    text(if self.current_view == ViewType::WorkWeek { "✓ Work Week" } else { "  Work Week" }).size(13)
                )
                .on_press(Message::SwitchView(ViewType::WorkWeek))
                .padding([8, 20])
                .width(Length::Fill)
            ),
            Item::new(
                button(
                    text(if self.current_view == ViewType::Week { "✓ Week" } else { "  Week" }).size(13)
                )
                .on_press(Message::SwitchView(ViewType::Week))
                .padding([8, 20])
                .width(Length::Fill)
            ),
            Item::new(
                button(
                    text(if self.current_view == ViewType::Month { "✓ Month" } else { "  Month" }).size(13)
                )
                .on_press(Message::SwitchView(ViewType::Month))
                .padding([8, 20])
                .width(Length::Fill)
            ),
            Item::new(
                button(
                    text(if self.current_view == ViewType::Quarter { "✓ Quarter" } else { "  Quarter" }).size(13)
                )
                .on_press(Message::SwitchView(ViewType::Quarter))
                .padding([8, 20])
                .width(Length::Fill)
            ),
        ])
        .max_width(180.0)
        .offset(0.0)
        .spacing(0.0);

        let view_menu = Item::with_menu(
            button(text("View").size(14)).padding([5, 10]),
            Menu::new(vec![
                Item::new(
                    button(
                        text(if self.show_my_day { "✓ My Day" } else { "  My Day" }).size(13)
                    )
                    .on_press(Message::ToggleMyDay)
                    .padding([8, 20])
                    .width(Length::Fill)
                ),
                Item::new(
                    button(
                        text(if self.show_ribbon { "✓ Ribbon" } else { "  Ribbon" }).size(13)
                    )
                    .on_press(Message::ToggleRibbon)
                    .padding([8, 20])
                    .width(Length::Fill)
                ),
                Item::with_menu(
                    button(text("Views ▶").size(13))
                        .padding([8, 20])
                        .width(Length::Fill),
                    views_submenu
                ),
            ])
            .max_width(180.0)
            .offset(0.0)
            .spacing(0.0)
        );

        let menu_bar = MenuBar::new(vec![file_menu, edit_menu, view_menu]);

        let toolbar = row![
            menu_bar,
            // Spacer
            text("").width(Length::Fill),
            // Theme toggle with fixed width
            button(text(if matches!(self.theme, Theme::Light) { "🌙" } else { "☀️" }).size(16))
                .on_press(Message::ToggleTheme)
                .padding(8)
                .width(45),
        ]
        .padding(5)
        .spacing(5);

        toolbar.into()
    }

    /// Create the multi-day event ribbon
    /// Create the main calendar view
    fn create_calendar_view(&self) -> Element<Message> {
        match self.current_view {
            ViewType::Month => self.create_month_view(),
            ViewType::Day => helpers::create_placeholder_view("Day View - Coming Soon"),
            ViewType::WorkWeek => helpers::create_placeholder_view("Work Week View - Coming Soon"),
            ViewType::Week => helpers::create_placeholder_view("Week View - Coming Soon"),
            ViewType::Quarter => helpers::create_placeholder_view("Quarter View - Coming Soon"),
        }
    }

    /// Create the Month view with calendar grid
    fn create_month_view(&self) -> Element<Message> {
        let today = Local::now().naive_local().date();
        
        // Month header with navigation
        let month_name = self.current_date.format("%B %Y").to_string();
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
        let first_of_month = self.current_date.with_day(1).unwrap();
        let first_weekday = first_of_month.weekday().num_days_from_sunday() as i32;
        let days_in_month = self.get_days_in_month(self.current_date.year(), self.current_date.month());
        
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
                        self.current_date.year(),
                        self.current_date.month(),
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
                    let theme_colors = self.calendar_theme.clone();
                    if is_today {
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

        let theme_bg = self.calendar_theme.calendar_background;
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
    fn get_days_in_month(&self, year: i32, month: u32) -> i32 {
        if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .unwrap()
        .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
        .num_days() as i32
    }

    /// Create theme manager dialog
    fn create_theme_manager_dialog(&self) -> Element<Message> {
        let mut theme_list: Vec<Element<Message>> = vec![
            text("Available Themes:").size(16).into(),
        ];
        
        // List all themes with delete buttons for custom themes
        for theme_name in &self.available_themes {
            let is_builtin = theme_name == "Light" || theme_name == "Dark";
            let is_current = theme_name == &self.theme_name;
            
            let theme_text = if is_current {
                format!("✓ {}", theme_name)
            } else {
                theme_name.clone()
            };
            
            if is_builtin {
                // Built-in themes - just show the name
                theme_list.push(
                    row![
                        text(theme_text).size(14).width(Length::Fill),
                        text("(Built-in)").size(12),
                    ]
                    .spacing(10)
                    .padding(5)
                    .into()
                );
            } else {
                // Custom themes - show with delete button
                let delete_button = button(text("Delete").size(12))
                    .on_press(Message::DeleteTheme(theme_name.clone()))
                    .padding([5, 10]);
                
                theme_list.push(
                    row![
                        text(theme_text).size(14).width(Length::Fill),
                        delete_button,
                    ]
                    .spacing(10)
                    .padding(5)
                    .into()
                );
            }
        }
        
        let close_button = button(text("Close").size(14))
            .on_press(Message::CloseThemeManager)
            .padding([10, 30]);

        // Custom header with close button
        let close_btn = button(text("×").size(24))
            .on_press(Message::CloseThemeManager)
            .padding(5);
        
        let header = row![
            text("Manage Themes").size(20),
            text("").width(Length::Fill), // Spacer
            close_btn
        ]
        .align_items(iced::Alignment::Center);

        Card::new(
            header,
            column(theme_list).spacing(5)
        )
        .foot(
            column![
                text("Note: Create custom themes via the database for now.").size(11),
                row![close_button]
                    .spacing(10)
                    .padding([10, 10, 10, 10])
            ]
        )
        .max_width(400.0)
        .into()
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

    /// Create the settings dialog
    fn create_settings_dialog(&self) -> Element<Message> {
        // Theme setting - use available themes from database
        let theme_label = text("Theme:").size(14);
        let theme_picker = pick_list(
            self.available_themes.clone(),
            Some(self.theme_name.clone()),
            Message::UpdateTheme
        );
        
        let manage_themes_button = button(text("Manage Themes...").size(12))
            .on_press(Message::OpenThemeManager)
            .padding([5, 10]);

        // View setting
        let view_label = text("Default View:").size(14);
        let current_view_str = match self.current_view {
            ViewType::Day => "Day",
            ViewType::WorkWeek => "Work Week",
            ViewType::Week => "Week",
            ViewType::Month => "Month",
            ViewType::Quarter => "Quarter",
        };
        let view_picker = pick_list(
            vec!["Day", "Work Week", "Week", "Month", "Quarter"],
            Some(current_view_str),
            |view| {
                let view_enum = match view {
                    "Work Week" => "WorkWeek",
                    _ => view,
                };
                Message::UpdateView(view_enum.to_string())
            }
        );

        // My Day panel checkbox
        let my_day_checkbox = checkbox("Show My Day Panel", self.show_my_day)
            .on_toggle(Message::UpdateShowMyDay);

        // My Day position setting
        let my_day_position_label = text("My Day Position:").size(14);
        let current_position = if self.my_day_position_right { "Right" } else { "Left" };
        let my_day_position_picker = pick_list(
            vec!["Left", "Right"],
            Some(current_position),
            |position| Message::UpdateMyDayPosition(position.to_string())
        );

        // Ribbon checkbox
        let ribbon_checkbox = checkbox("Show Multi-Day Ribbon", self.show_ribbon)
            .on_toggle(Message::UpdateShowRibbon);

        // Time format setting
        let time_format_label = text("Time Format:").size(14);
        let time_format_picker = pick_list(
            vec!["12h", "24h"],
            Some(self.time_format.as_str()),
            |format| Message::UpdateTimeFormat(format.to_string())
        );

        // First day of week setting
        let first_day_label = text("First Day of Week:").size(14);
        let current_day_idx = self.first_day_of_week as usize;
        let day_names = vec!["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
        let current_day_name = if current_day_idx < day_names.len() {
            day_names[current_day_idx]
        } else {
            "Sunday"
        };
        
        let first_day_picker = pick_list(
            day_names,
            Some(current_day_name),
            |selected| {
                let day_num = match selected {
                    "Sunday" => "0",
                    "Monday" => "1",
                    "Tuesday" => "2",
                    "Wednesday" => "3",
                    "Thursday" => "4",
                    "Friday" => "5",
                    "Saturday" => "6",
                    _ => "0",
                };
                Message::UpdateFirstDayOfWeek(day_num.to_string())
            }
        );

        let save_button = button(text("Save").size(14))
            .on_press(Message::SaveSettings)
            .padding([10, 30]);

        let cancel_button = button(text("Cancel").size(14))
            .on_press(Message::CloseSettings)
            .padding([10, 30]);

        // Custom header with close button
        let close_btn = button(text("×").size(24))
            .on_press(Message::CloseSettings)
            .padding(5);
        
        let header = row![
            text("Settings").size(24),
            text("").width(Length::Fill), // Spacer
            close_btn
        ]
        .align_items(iced::Alignment::Center);

        Card::new(
            header,
            column![
                text("Display Settings:").size(16),
                row![theme_label, theme_picker, manage_themes_button].spacing(10),
                row![view_label, view_picker].spacing(10),
                my_day_checkbox,
                row![my_day_position_label, my_day_position_picker].spacing(10),
                ribbon_checkbox,
                text("").size(15),
                text("General Settings:").size(16),
                row![time_format_label, time_format_picker].spacing(10),
                row![first_day_label, first_day_picker].spacing(10),
            ]
            .spacing(8)
        )
        .foot(
            row![save_button, cancel_button]
                .spacing(10)
                .padding([0, 10, 10, 10])
        )
        .max_width(600.0)
        .into()
    }
}