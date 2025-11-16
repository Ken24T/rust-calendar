mod state;
mod update;
mod view;

pub use state::CalendarApp;

use iced::{Application, Command, Element, Subscription, Theme};

use crate::ui::messages::Message;

impl Application for CalendarApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = String;

    fn new(db_path: Self::Flags) -> (Self, Command<Self::Message>) {
        CalendarApp::create(db_path)
    }

    fn title(&self) -> String {
        String::from("Rust Calendar")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        self.handle_message(message)
    }

    fn view(&self) -> Element<Self::Message> {
        self.render_view()
    }

    fn theme(&self) -> Self::Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        use iced::{event, keyboard, Event};

        event::listen_with(|event, _status| match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                Some(Message::KeyPressed(key, modifiers))
            }
            _ => None,
        })
    }
}
