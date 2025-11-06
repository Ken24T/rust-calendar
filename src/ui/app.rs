// Main Calendar Application
// Core iced Application implementation

use iced::{
    widget::{button, column, container, row, text},
    Application, Command, Element, Length, Theme,
};
use iced_aw::menu::{Item, Menu, MenuBar};
use crate::services::database::Database;
use crate::services::settings::SettingsService;
use std::sync::{Arc, Mutex};

/// Main Calendar Application
pub struct CalendarApp {
    /// Current theme (Light or Dark)
    theme: Theme,
    /// Show/hide My Day panel
    show_my_day: bool,
    /// Show/hide multi-day ribbon
    show_ribbon: bool,
    /// Current view type
    current_view: ViewType,
    /// Database connection (wrapped in Arc<Mutex<>> for thread safety)
    db: Arc<Mutex<Database>>,
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
                show_ribbon: settings.show_ribbon,
                current_view,
                db: Arc::new(Mutex::new(db)),
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
                // TODO: Open settings dialog
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
            row![
                self.create_my_day_panel(),
                self.create_calendar_view(),
            ]
            .spacing(2)
        } else {
            row![self.create_calendar_view()]
        };
        
        content = content.push(main_content);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
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
        settings.show_ribbon = self.show_ribbon;
        settings.current_view = view_str.to_string();

        // Save to database
        if let Err(e) = settings_service.update(&settings) {
            eprintln!("Error: Failed to save settings: {}", e);
        }
    }
}
