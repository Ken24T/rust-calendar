// Main Calendar Application
// Core iced Application implementation

use iced::{
    widget::{button, column, container, row, text},
    Application, Command, Element, Length, Theme,
};
use iced_aw::menu::{Item, Menu, MenuBar};

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
    /// Open settings dialog
    OpenSettings,
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
                show_my_day: false,
                show_ribbon: false,
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
                Item::new(button(text("Exit").size(13)).padding([5, 15])),
            ])
            .max_width(180.0)
            .offset(0.0)
            .spacing(5.0)
        );

        let edit_menu = Item::with_menu(
            button(text("Edit").size(14)).padding([5, 10]),
            Menu::new(vec![
                Item::new(
                    button(text("Settings").size(13))
                        .on_press(Message::OpenSettings)
                        .padding([5, 15])
                ),
            ])
            .max_width(180.0)
            .offset(0.0)
            .spacing(5.0)
        );

        let view_menu = Item::with_menu(
            button(text("View").size(14)).padding([5, 10]),
            Menu::new(vec![
                Item::new(
                    button(
                        text(if self.show_my_day { "✓ My Day" } else { "  My Day" }).size(13)
                    )
                    .on_press(Message::ToggleMyDay)
                    .padding([5, 15])
                ),
                Item::new(
                    button(
                        text(if self.show_ribbon { "✓ Ribbon" } else { "  Ribbon" }).size(13)
                    )
                    .on_press(Message::ToggleRibbon)
                    .padding([5, 15])
                ),
            ])
            .max_width(180.0)
            .offset(0.0)
            .spacing(5.0)
        );

        let menu_bar = MenuBar::new(vec![file_menu, edit_menu, view_menu]);

        let toolbar = row![
            menu_bar,
            // Spacer
            text("").width(Length::Fill),
            // Theme toggle
            button(text(if matches!(self.theme, Theme::Light) { "🌙" } else { "☀️" }).size(16))
                .on_press(Message::ToggleTheme)
                .padding(5),
            text("  |  ").size(14),
            // View switcher
            button(text("Day").size(13))
                .on_press(Message::SwitchView(ViewType::Day))
                .padding([5, 15]),
            button(text("Week").size(13))
                .on_press(Message::SwitchView(ViewType::Week))
                .padding([5, 15]),
            button(text("Month").size(13))
                .on_press(Message::SwitchView(ViewType::Month))
                .padding([5, 15]),
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
