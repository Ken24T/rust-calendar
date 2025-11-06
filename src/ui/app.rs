// Main Calendar Application
// Core iced Application implementation

use iced::{
    widget::{button, column, container, row, text},
    Application, Command, Element, Length, Theme,
};

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewType {
    Day,
    Week,
    Month,
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
}

impl Application for CalendarApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                theme: Theme::Light,
                show_my_day: true,
                show_ribbon: true,
                current_view: ViewType::Month,
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
            }
            Message::ToggleMyDay => {
                self.show_my_day = !self.show_my_day;
            }
            Message::ToggleRibbon => {
                self.show_ribbon = !self.show_ribbon;
            }
            Message::SwitchView(view_type) => {
                self.current_view = view_type;
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
    /// Create the top menu bar
    fn create_menu_bar(&self) -> Element<Message> {
        let theme_icon = if matches!(self.theme, Theme::Light) {
            "🌙"
        } else {
            "☀️"
        };

        let my_day_label = if self.show_my_day {
            "Hide My Day"
        } else {
            "Show My Day"
        };

        let ribbon_label = if self.show_ribbon {
            "Hide Ribbon"
        } else {
            "Show Ribbon"
        };

        let view_label = match self.current_view {
            ViewType::Day => "Day View",
            ViewType::Week => "Week View",
            ViewType::Month => "Month View",
        };

        row![
            text("Rust Calendar").size(20),
            button(text(theme_icon)).on_press(Message::ToggleTheme),
            button(text(my_day_label)).on_press(Message::ToggleMyDay),
            button(text(ribbon_label)).on_press(Message::ToggleRibbon),
            button(text("Day")).on_press(Message::SwitchView(ViewType::Day)),
            button(text("Week")).on_press(Message::SwitchView(ViewType::Week)),
            button(text("Month")).on_press(Message::SwitchView(ViewType::Month)),
            text(view_label).size(16),
        ]
        .spacing(10)
        .padding(10)
        .into()
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
            ViewType::Week => "Week View - Coming Soon",
            ViewType::Month => "Month View - Coming Soon",
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
}
