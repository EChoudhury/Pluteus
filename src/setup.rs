//! Setup screen: bind a document, pick a goal, tune typography and theme.

use std::path::{Path, PathBuf};

use iced::theme::Palette;
use iced::widget::{
    button, column, container, horizontal_space, radio, row, scrollable, slider, text, text_input,
    toggler,
};
use iced::{Alignment, Border, Color, Length, Shadow, Task, Vector};

use crate::types::{
    blend, count_words, muted, FontChoice, GoalMode, Message, SessionTarget, SetupState,
    ThemeChoice,
};

/// Convenience alias for this screen's widgets.
type Element<'a> = iced::Element<'a, Message>;

/// Handles every message that belongs to the setup screen.
pub fn update(state: &mut SetupState, message: Message) -> Task<Message> {
    match message {
        Message::PickFileRequested => pick_file(),
        Message::FilePicked(Some(path)) => {
            state.target_path = Some(path.clone());
            state.document_text = None;
            state.is_loading = true;
            state.error = None;
            load_document(path)
        }
        Message::FilePicked(None) => Task::none(),
        Message::NewFileRequested => new_file(),
        Message::NewFilePicked(Some(path)) => {
            let path = with_default_extension(path);
            state.target_path = Some(path.clone());
            state.document_text = None;
            state.is_loading = true;
            state.error = None;
            create_document(path)
        }
        Message::NewFilePicked(None) => Task::none(),
        Message::DocumentLoaded(path, result) => {
            if state.target_path.as_deref() == Some(path.as_path()) {
                state.is_loading = false;

                match result {
                    Ok(contents) => {
                        state.document_text = Some(contents);
                        state.error = None;
                    }
                    Err(error) => {
                        state.document_text = None;
                        state.error = Some(error);
                    }
                }
            }

            Task::none()
        }
        Message::GoalModeSelected(mode) => {
            if state.goal_mode != mode {
                state.goal_mode = mode;
                state.goal_input = String::from(mode.default_input());
                state.goal = SessionTarget::parse(mode, &state.goal_input);
            }

            Task::none()
        }
        Message::GoalInputChanged(input) => {
            state.goal = SessionTarget::parse(state.goal_mode, &input);
            state.goal_input = input;
            Task::none()
        }
        Message::FontSizeChanged(size) => {
            state.typography.font_size = size;
            Task::none()
        }
        Message::LineHeightChanged(height) => {
            state.typography.line_height = height;
            Task::none()
        }
        Message::FontSelected(font) => {
            state.font = font;
            Task::none()
        }
        Message::ThemeSelected(theme) => {
            state.theme = theme;
            Task::none()
        }
        Message::FullscreenToggled(fullscreen) => {
            state.fullscreen = fullscreen;
            Task::none()
        }
        Message::ShowWriterInfoToggled(show_info) => {
            state.show_info = show_info;
            Task::none()
        }
        Message::FocusBlurToggled(focus_blur) => {
            state.focus_blur = focus_blur;
            Task::none()
        }
        _ => Task::none(),
    }
}

/// Opens the native file picker on a background task.
fn pick_file() -> Task<Message> {
    Task::perform(
        async {
            rfd::AsyncFileDialog::new()
                .set_title("Choose a document")
                .add_filter("Plain text", &["txt"])
                .add_filter("Markdown", &["md", "markdown"])
                .add_filter("All files", &["*"])
                .pick_file()
                .await
                .map(|handle| handle.path().to_path_buf())
        },
        Message::FilePicked,
    )
}

/// Opens the native "save as" dialog to create a blank document.
fn new_file() -> Task<Message> {
    Task::perform(
        async {
            rfd::AsyncFileDialog::new()
                .set_title("Create a new document")
                .set_file_name("Untitled.txt")
                .add_filter("Plain text", &["txt"])
                .add_filter("Markdown", &["md", "markdown"])
                .save_file()
                .await
                .map(|handle| handle.path().to_path_buf())
        },
        Message::NewFilePicked,
    )
}

/// Creates a blank document on disk and loads it into the setup screen.
fn create_document(path: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let result = match tokio::fs::write(&path, b"").await {
                Ok(()) => Ok(String::new()),
                Err(error) => Err(format!("Could not create {}: {error}", path.display())),
            };

            (path, result)
        },
        |(path, result)| Message::DocumentLoaded(path, result),
    )
}

/// Reads the chosen document without blocking the UI thread.
fn load_document(path: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let result = match tokio::fs::read_to_string(&path).await {
                Ok(contents) => Ok(contents),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
                Err(error) => Err(format!("Could not read {}: {error}", path.display())),
            };

            (path, result)
        },
        |(path, result)| Message::DocumentLoaded(path, result),
    )
}

/// Renders the setup screen.
pub fn view(state: &SetupState) -> Element<'_> {
    let palette = state.theme.palette();

    let heading = column![
        text("Pluteus").size(34).color(palette.text),
        text("A quiet desk, a clear goal, and no distractions.")
            .size(13)
            .color(muted(palette)),
    ]
    .spacing(5)
    .align_x(Alignment::Center);

    let mut content = column![
        heading,
        document_card(state, palette),
        session_card(state, palette),
        typography_card(state, palette),
        writing_card(state, palette),
        start_button(palette),
    ]
    .spacing(12)
    .width(Length::Fill)
    .max_width(760.0)
    .align_x(Alignment::Center);

    if let Some(error) = &state.error {
        content = content.push(text(error.as_str()).size(13).color(palette.danger));
    }

    scrollable(
        container(content)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding([22, 24]),
    )
    .height(Length::Fill)
    .into()
}

/// The "Document" card.
fn document_card<'a>(state: &'a SetupState, palette: Palette) -> Element<'a> {
    let label = match (&state.target_path, state.is_loading) {
        (None, _) => String::from("No document selected yet."),
        (Some(path), true) => format!("Opening {}…", short_path(path)),
        (Some(path), false) => short_path(path),
    };

    let details = match &state.document_text {
        Some(document) => format!("{label}  •  {} words", count_words(document)),
        None => label,
    };

    let opener = button(text("Open file…").size(14))
        .padding([8, 16])
        .style(move |_theme, status| secondary_button_style(palette, status))
        .on_press(Message::PickFileRequested);

    let creator = button(text("New file…").size(14))
        .padding([8, 16])
        .style(move |_theme, status| secondary_button_style(palette, status))
        .on_press(Message::NewFileRequested);

    let body = column![
        row![opener, creator, text(details).size(13).color(muted(palette))]
            .spacing(10)
            .align_y(Alignment::Center),
        text("Open an existing .txt or .md document, or create a blank one. Everything is saved back automatically — every 10 new words or 10 seconds of inactivity.")
            .size(12)
            .color(muted(palette)),
    ]
    .spacing(8);

    card(palette, "Document", body)
}

/// The "Session" card.
fn session_card<'a>(state: &'a SetupState, palette: Palette) -> Element<'a> {
    let mode = row![
        radio(
            GoalMode::Minutes.label(),
            GoalMode::Minutes,
            Some(state.goal_mode),
            Message::GoalModeSelected,
        )
        .text_size(14),
        radio(
            GoalMode::Words.label(),
            GoalMode::Words,
            Some(state.goal_mode),
            Message::GoalModeSelected,
        )
        .text_size(14),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let input = text_input(goal_placeholder(state.goal_mode), &state.goal_input)
        .on_input(Message::GoalInputChanged)
        .padding(8)
        .size(14)
        .width(110)
        .align_x(Alignment::Center);

    let summary = match state.goal_mode {
        GoalMode::Minutes => "Ends when the clock runs out.",
        GoalMode::Words => "Ends when you reach the word goal.",
    };

    let body = column![
        row![mode, horizontal_space(), input]
            .spacing(14)
            .align_y(Alignment::Center),
        text(summary).size(12).color(muted(palette)),
    ]
    .spacing(8);

    card(palette, "Session", body)
}

/// The "Typography & theme" card.
fn typography_card<'a>(state: &'a SetupState, palette: Palette) -> Element<'a> {
    let font_size = column![
        row![
            text("Text size").size(13).color(palette.text),
            horizontal_space(),
            text(format!("{:.0} px", state.typography.font_size))
                .size(12)
                .color(muted(palette)),
        ],
        slider(
            14.0..=30.0,
            state.typography.font_size,
            Message::FontSizeChanged,
        )
        .step(0.5_f32),
    ]
    .spacing(2)
    .width(Length::Fill);

    let line_height = column![
        row![
            text("Line spacing").size(13).color(palette.text),
            horizontal_space(),
            text(format!("{:.1}×", state.typography.line_height))
                .size(12)
                .color(muted(palette)),
        ],
        slider(
            1.0..=2.2,
            state.typography.line_height,
            Message::LineHeightChanged,
        )
        .step(0.1_f32),
    ]
    .spacing(2)
    .width(Length::Fill);

    let sliders = row![font_size, line_height].spacing(24);

    let typefaces = row(FontChoice::ALL.iter().map(|choice| -> Element<'a> {
        let selected = *choice == state.font;

        button(text(choice.label()).font(choice.font()).size(13))
            .padding([7, 14])
            .style(move |_theme, status| choice_button_style(palette, selected, status))
            .on_press(Message::FontSelected(*choice))
            .into()
    }))
    .spacing(8);

    let themes = row(ThemeChoice::ALL.iter().map(|choice| -> Element<'a> {
        let selected = *choice == state.theme;

        button(text(choice.label()).size(13))
            .padding([7, 16])
            .style(move |_theme, status| choice_button_style(palette, selected, status))
            .on_press(Message::ThemeSelected(*choice))
            .into()
    }))
    .spacing(8);

    let body = column![
        sliders,
        labeled_row("Typeface", typefaces, palette),
        labeled_row("Theme", themes, palette),
    ]
    .spacing(9);

    card(palette, "Typography & theme", body)
}

/// The "Writing view" card.
fn writing_card<'a>(state: &'a SetupState, palette: Palette) -> Element<'a> {
    let toggles = row![
        toggler(state.fullscreen)
            .label("Full screen")
            .text_size(13)
            .on_toggle(Message::FullscreenToggled),
        toggler(state.show_info)
            .label("Session info")
            .text_size(13)
            .on_toggle(Message::ShowWriterInfoToggled),
        toggler(state.focus_blur)
            .label("Focus blur")
            .text_size(13)
            .on_toggle(Message::FocusBlurToggled),
    ]
    .spacing(26)
    .align_y(Alignment::Center);

    let body = column![
        toggles,
        text("Only the sentence at your cursor stays sharp; every other sentence fades.")
            .size(12)
            .color(muted(palette)),
    ]
    .spacing(8);

    card(palette, "Writing view", body)
}

/// The primary call to action.
fn start_button<'a>(palette: Palette) -> Element<'a> {
    button(text("Start writing").size(16))
        .padding([10, 30])
        .style(move |_theme, status| primary_button_style(palette, status))
        .on_press(Message::StartSession)
        .into()
}

/// Wraps some content in a titled, padded card.
fn card<'a>(palette: Palette, title: &'static str, body: impl Into<Element<'a>>) -> Element<'a> {
    container(column![text(title).size(11).color(muted(palette)), body.into(),].spacing(9))
        .width(Length::Fill)
        .padding([14, 16])
        .style(move |_theme| card_style(palette))
        .into()
}

/// A row with a small label in front of some controls.
fn labeled_row<'a>(
    label: &'static str,
    controls: impl Into<Element<'a>>,
    palette: Palette,
) -> Element<'a> {
    row![
        text(label).size(12).color(muted(palette)).width(70),
        controls.into(),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

/// Gives a newly created document a `.txt` extension when none was typed.
fn with_default_extension(mut path: PathBuf) -> PathBuf {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("");

    if extension.is_empty() {
        path.set_extension("txt");
    }

    path
}

/// Shortens long paths so they fit in a single row.
fn short_path(path: &Path) -> String {
    const LIMIT: usize = 64;

    let display = path.display().to_string();
    let count = display.chars().count();

    if count <= LIMIT {
        return display;
    }

    let head: String = display.chars().take(20).collect();
    let tail: String = display.chars().skip(count - 32).collect();

    format!("{head}…{tail}")
}

/// The placeholder of the goal input for the active mode.
fn goal_placeholder(mode: GoalMode) -> &'static str {
    match mode {
        GoalMode::Minutes => "25",
        GoalMode::Words => "500",
    }
}

/// Styling of the cards.
fn card_style(palette: Palette) -> container::Style {
    container::Style {
        text_color: Some(palette.text),
        background: Some(blend(palette.background, palette.text, 0.05).into()),
        border: Border {
            color: blend(palette.background, palette.text, 0.13),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow::default(),
    }
}

/// Styling of the main call to action.
fn primary_button_style(palette: Palette, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Active | button::Status::Hovered => (palette.primary, Color::WHITE),
        button::Status::Pressed => (blend(palette.primary, Color::BLACK, 0.18), Color::WHITE),
        button::Status::Disabled => (
            blend(palette.background, palette.primary, 0.35),
            muted(palette),
        ),
    };

    button::Style {
        background: Some(background.into()),
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 9.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.16),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 6.0,
        },
    }
}

/// Styling of secondary buttons.
fn secondary_button_style(palette: Palette, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Active => blend(palette.background, palette.text, 0.08),
        button::Status::Hovered => blend(palette.background, palette.text, 0.14),
        button::Status::Pressed => blend(palette.background, palette.text, 0.20),
        button::Status::Disabled => blend(palette.background, palette.text, 0.04),
    };

    button::Style {
        background: Some(background.into()),
        text_color: palette.text,
        border: Border {
            color: blend(palette.background, palette.text, 0.16),
            width: 1.0,
            radius: 9.0.into(),
        },
        shadow: Shadow::default(),
    }
}

/// Styling of the font and theme picker buttons.
fn choice_button_style(palette: Palette, selected: bool, status: button::Status) -> button::Style {
    if !selected {
        return secondary_button_style(palette, status);
    }

    let background = match status {
        button::Status::Active => blend(palette.background, palette.primary, 0.22),
        button::Status::Hovered => blend(palette.background, palette.primary, 0.30),
        button::Status::Pressed => blend(palette.background, palette.primary, 0.38),
        button::Status::Disabled => blend(palette.background, palette.primary, 0.16),
    };

    button::Style {
        background: Some(background.into()),
        text_color: palette.text,
        border: Border {
            color: palette.primary,
            width: 1.0,
            radius: 9.0.into(),
        },
        shadow: Shadow::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_documents_default_to_txt() {
        assert_eq!(
            with_default_extension(PathBuf::from("notes")),
            PathBuf::from("notes.txt")
        );
        assert_eq!(
            with_default_extension(PathBuf::from("notes.md")),
            PathBuf::from("notes.md")
        );
        assert_eq!(
            with_default_extension(PathBuf::from("notes.txt")),
            PathBuf::from("notes.txt")
        );
    }
}
