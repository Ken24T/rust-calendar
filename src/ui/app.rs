// Main Calendar Application
// Core iced Application implementation

use iced::{
    widget::{button, column, container, row, text, pick_list, checkbox},
    Application, Command, Element, Length, Theme,
};
use iced_aw::menu::{Item, Menu, MenuBar};
use iced_aw::{Modal, Card};
use crate::services::database::Database;
use crate::services::settings::SettingsService;
use std::sync::{Arc, Mutex};

/// Main Calendar Application
pub struct CalendarApp {
    /// Current theme (Light or Dark)
    theme: Theme,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    Day,
    WorkWeek,
    Week,
    Month,
    Quarter,
}

/// Messages for the application
#[derive(Debug, Clone)]
pub enum Message {
    /// Toggle between light and dark theme
    ToggleTheme,
    /// Toggle My Day panel visibility
    ToggleMyDay,
    /// Toggle multi-day ribbon visibility
    ToggleRibbon,
    /// Switch to a different view
    SwitchView(ViewType),
    /// Open settings dialog
    OpenSettings,
    /// Close settings dialog
    CloseSettings,
    /// Update theme setting from dialog
    UpdateTheme(String),
    /// Update view setting from dialog
    UpdateView(String),
    /// Update My Day panel visibility from dialog
    UpdateShowMyDay(bool),
    /// Update My Day panel position from dialog
    UpdateMyDayPosition(String),
    /// Update Ribbon visibility from dialog
    UpdateShowRibbon(bool),
    /// Update time format setting
    UpdateTimeFormat(String),
    /// Update first day of week setting
    UpdateFirstDayOfWeek(String),
    /// Save settings from dialog
    SaveSettings,
    /// Exit the application
    Exit,
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

        // Parse theme
        let theme = match settings.theme.as_str() {
            "dark" => Theme::Dark,
            _ => Theme::Light,
        };

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
                show_my_day: settings.show_my_day,
                my_day_position_right: settings.my_day_position_right,
                show_ribbon: settings.show_ribbon,
                current_view,
                db: Arc::new(Mutex::new(db)),
                show_settings_dialog: false,
                time_format: settings.time_format.clone(),
                first_day_of_week: settings.first_day_of_week,
                date_format: settings.date_format.clone(),
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
                self.theme = match self.theme {
                    Theme::Light => Theme::Dark,
                    Theme::Dark => Theme::Light,
                    _ => Theme::Light,
                };
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
            Message::UpdateTheme(theme_str) => {
                self.theme = match theme_str.as_str() {
                    "dark" => Theme::Dark,
                    "light" => Theme::Light,
                    _ => Theme::Light,
                };
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
            Message::Exit => {
                std::process::exit(0);
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
            let ribbon = self.create_ribbon();
            content = content.push(ribbon);
        }

        // Main content area: My Day panel + Calendar view
        let main_content = if self.show_my_day {
            if self.my_day_position_right {
                row![
                    self.create_calendar_view(),
                    self.create_my_day_panel(),
                ]
                .spacing(2)
            } else {
                row![
                    self.create_my_day_panel(),
                    self.create_calendar_view(),
                ]
                .spacing(2)
            }
        } else {
            row![self.create_calendar_view()]
        };
        
        content = content.push(main_content);

        let base_view = container(content)
            .width(Length::Fill)
            .height(Length::Fill);

        // Show modal dialog if settings is open
        if self.show_settings_dialog {
            Modal::new(base_view, Some(self.create_settings_dialog()))
                .backdrop(Message::CloseSettings)
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
    fn create_ribbon(&self) -> Element<Message> {
        container(
            text("Multi-Day Event Ribbon (Coming Soon)")
                .size(14)
        )
        .padding(10)
        .width(Length::Fill)
        .into()
    }

    /// Create the My Day panel
    fn create_my_day_panel(&self) -> Element<Message> {
        container(
            column![
                text("My Day").size(18),
                text("Thu, Nov 6, 2025").size(14),
                text(""),
                text("No events today").size(12),
            ]
            .spacing(10)
        )
        .padding(15)
        .width(250)
        .height(Length::Fill)
        .into()
    }

    /// Create the main calendar view
    fn create_calendar_view(&self) -> Element<Message> {
        let view_content = match self.current_view {
            ViewType::Day => "Day View - Coming Soon",
            ViewType::WorkWeek => "Work Week View - Coming Soon",
            ViewType::Week => "Week View - Coming Soon",
            ViewType::Month => "Month View - Coming Soon",
            ViewType::Quarter => "Quarter View - Coming Soon",
        };

        container(
            column![
                text(view_content).size(24),
                text(""),
                text("This is where the calendar grid will appear").size(14),
            ]
            .spacing(20)
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
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

        // Create settings from current state
        let theme_str = match self.theme {
            Theme::Dark => "dark",
            _ => "light",
        };

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
        settings.theme = theme_str.to_string();
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
        // Theme setting
        let theme_label = text("Theme:").size(14);
        let current_theme = match self.theme {
            Theme::Dark => "Dark",
            _ => "Light",
        };
        let theme_picker = pick_list(
            vec!["Light", "Dark"],
            Some(current_theme),
            |theme| Message::UpdateTheme(theme.to_lowercase())
        );

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

        Card::new(
            text("Settings").size(24),
            column![
                text("Display Settings:").size(16),
                row![theme_label, theme_picker].spacing(10),
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
        .max_width(550.0)
        .on_close(Message::CloseSettings)
        .into()
    }
}