//! Pluteus — a calm, session-based writing application built with iced.

mod focus;
mod setup;
mod toast;
mod types;
mod writer;

use iced::{Subscription, Task, Theme};

use crate::types::{AppScreen, Message, SetupState, WriterState};

/// Convenience alias for the application's widgets.
type Element<'a> = iced::Element<'a, Message>;

fn main() -> iced::Result {
    iced::application("Pluteus", Pluteus::update, Pluteus::view)
        .theme(Pluteus::theme)
        .subscription(Pluteus::subscription)
        .font(include_bytes!("../assets/FiraSans-Regular.ttf").as_slice())
        .font(include_bytes!("../assets/Montserrat-Regular.ttf").as_slice())
        .window_size(iced::Size::new(1100.0, 800.0))
        .centered()
        .antialiasing(true)
        .run()
}

/// The root application state: exactly one screen is active at a time.
struct Pluteus {
    screen: AppScreen,
}

impl Default for Pluteus {
    fn default() -> Self {
        Self {
            screen: AppScreen::Setup(SetupState::default()),
        }
    }
}

impl Pluteus {
    /// Routes every message to the module that owns it.
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::StartSession => self.start_session(),
            Message::EscapeRequested => self.leave_writer(),
            Message::RuntimeEvent(event) => {
                if writer::is_escape_event(&event) {
                    self.leave_writer()
                } else {
                    Task::none()
                }
            }
            other => match &mut self.screen {
                AppScreen::Setup(setup) => setup::update(setup, other),
                AppScreen::Writer(writer) => writer::update(writer, other),
            },
        }
    }

    /// Validates the setup form and enters the writer screen.
    fn start_session(&mut self) -> Task<Message> {
        let AppScreen::Setup(setup) = &mut self.screen else {
            return Task::none();
        };

        let valid_goal = setup
            .goal_input
            .trim()
            .parse::<u32>()
            .map(|value| value > 0)
            .unwrap_or(false);

        if !valid_goal {
            setup.error = Some(String::from(
                "Enter a positive whole number for the session goal.",
            ));
            return Task::none();
        }

        let Some(path) = setup.target_path.clone() else {
            setup.error = Some(String::from("Choose a document file before starting."));
            return Task::none();
        };

        let Some(document) = setup.document_text.clone() else {
            setup.error = Some(String::from(if setup.is_loading {
                "The document is still loading. Try again in a moment."
            } else {
                "The selected document could not be prepared. Choose another file."
            }));
            return Task::none();
        };

        let settings = setup.settings();

        self.screen = AppScreen::Writer(WriterState::new(path, document, settings));

        if settings.fullscreen {
            window_mode(true)
        } else {
            Task::none()
        }
    }

    /// Saves the document and returns to the setup screen.
    fn leave_writer(&mut self) -> Task<Message> {
        let (save, setup, was_fullscreen) = match &self.screen {
            AppScreen::Writer(writer) => (
                writer::save_task(writer),
                SetupState::from_writer(writer),
                writer.fullscreen,
            ),
            AppScreen::Setup(_) => return Task::none(),
        };

        self.screen = AppScreen::Setup(setup);

        let mut tasks = vec![save];

        if was_fullscreen {
            tasks.push(window_mode(false));
        }

        Task::batch(tasks)
    }

    /// Renders the active screen.
    fn view(&self) -> Element<'_> {
        match &self.screen {
            AppScreen::Setup(setup) => setup::view(setup),
            AppScreen::Writer(writer) => writer::view(writer),
        }
    }

    /// The subscriptions of the active screen.
    fn subscription(&self) -> Subscription<Message> {
        match &self.screen {
            AppScreen::Setup(_) => Subscription::none(),
            AppScreen::Writer(writer) => writer::subscription(writer),
        }
    }

    /// The theme of the active screen.
    fn theme(&self) -> Theme {
        match &self.screen {
            AppScreen::Setup(setup) => setup.theme.theme(),
            AppScreen::Writer(writer) => writer.theme.theme(),
        }
    }
}

/// Switches the application's window between full screen and windowed mode.
fn window_mode(fullscreen: bool) -> Task<Message> {
    iced::window::get_latest().and_then(move |id| {
        let mode = if fullscreen {
            iced::window::Mode::Fullscreen
        } else {
            iced::window::Mode::Windowed
        };

        iced::window::change_mode(id, mode)
    })
}
