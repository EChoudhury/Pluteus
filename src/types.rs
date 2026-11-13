//! Shared state, messages, themes and small helpers used across the app.

use std::path::PathBuf;

use iced::theme::Palette;
use iced::widget::text_editor;
use iced::{Color, Font, Theme};

/// How many newly written words may pile up before an automatic save happens.
pub const AUTO_SAVE_WORD_DELTA: usize = 10;

/// How many seconds of inactivity trigger an automatic save.
pub const AUTO_SAVE_IDLE_SECONDS: u64 = 10;

/// How many one-second ticks a toast stays on screen before it fades away.
pub const TOAST_TICKS: u8 = 7;

/// Every interaction the application can react to.
#[derive(Debug, Clone)]
pub enum Message {
    // -- Setup screen -------------------------------------------------------
    /// The user asked for the native file picker.
    PickFileRequested,
    /// The native file picker returned (or was cancelled).
    FilePicked(Option<PathBuf>),
    /// The user asked for the native "save as" dialog to create a document.
    NewFileRequested,
    /// The native save dialog returned a path (or was cancelled).
    NewFilePicked(Option<PathBuf>),
    /// The background read (or creation) of the chosen document finished.
    DocumentLoaded(PathBuf, Result<String, String>),
    /// A session goal unit (`Minutes` or `Words`) was selected.
    GoalModeSelected(GoalMode),
    /// The raw text of the session goal input changed.
    GoalInputChanged(String),
    /// The preferred text size changed.
    FontSizeChanged(f32),
    /// The preferred line height changed.
    LineHeightChanged(f32),
    /// A typeface was selected.
    FontSelected(FontChoice),
    /// A color theme was selected.
    ThemeSelected(ThemeChoice),
    /// The "full screen when writing" preference changed.
    FullscreenToggled(bool),
    /// The "show writer information" preference changed.
    ShowWriterInfoToggled(bool),
    /// The "focus blur" preference changed.
    FocusBlurToggled(bool),
    /// The user wants to begin a writing session.
    StartSession,

    // -- Writer screen ------------------------------------------------------
    /// The text editor produced an action (edit, move, selection, ...).
    EditorAction(text_editor::Action),
    /// Escape was pressed while the editor had keyboard focus.
    EscapeRequested,
    /// A one-second interval elapsed.
    Tick,
    /// A global keyboard/runtime event was observed.
    RuntimeEvent(iced::event::Event),
    /// An asynchronous save finished.
    SaveFinished(Result<(), String>),
}

/// The two top-level screens of the application.
pub enum AppScreen {
    Setup(SetupState),
    Writer(WriterState),
}

/// How the session goal is expressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalMode {
    Minutes,
    Words,
}

impl GoalMode {
    /// The label shown on the matching radio button.
    pub fn label(self) -> &'static str {
        match self {
            Self::Minutes => "Minutes",
            Self::Words => "Words",
        }
    }

    /// A sensible starting value for the matching input.
    pub fn default_input(self) -> &'static str {
        match self {
            Self::Minutes => "25",
            Self::Words => "500",
        }
    }
}

/// The goal of a writing session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTarget {
    /// Write for a number of minutes.
    Duration(u32),
    /// Write a number of new words.
    WordCount(usize),
}

impl SessionTarget {
    /// Parses (and clamps) a raw goal input into a valid session target.
    pub fn parse(mode: GoalMode, input: &str) -> Self {
        match mode {
            GoalMode::Minutes => {
                let minutes = input.trim().parse::<u32>().unwrap_or(0).clamp(1, 600);
                Self::Duration(minutes)
            }
            GoalMode::Words => {
                let words = input.trim().parse::<usize>().unwrap_or(0).max(1);
                Self::WordCount(words)
            }
        }
    }
}

/// The available typefaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontChoice {
    Courier,
    #[default]
    SansSerif,
    TimesNewRoman,
    Montserrat,
}

impl FontChoice {
    /// All typefaces, in display order.
    pub const ALL: [Self; 4] = [
        Self::Courier,
        Self::SansSerif,
        Self::TimesNewRoman,
        Self::Montserrat,
    ];

    /// The human readable name of the typeface.
    pub fn label(self) -> &'static str {
        match self {
            Self::Courier => "Courier",
            Self::SansSerif => "Sans serif",
            Self::TimesNewRoman => "Times New Roman",
            Self::Montserrat => "Montserrat",
        }
    }

    /// The `iced` font used to render a document with this typeface.
    ///
    /// `Courier New` and `Times New Roman` are resolved from the system font
    /// database; `Montserrat` ships with the application (see `assets/`).
    /// `Sans serif` uses the platform's default sans-serif family.
    pub fn font(self) -> Font {
        match self {
            Self::Courier => Font::with_name("Courier New"),
            Self::SansSerif => Font::DEFAULT,
            Self::TimesNewRoman => Font::with_name("Times New Roman"),
            Self::Montserrat => Font::with_name("Montserrat"),
        }
    }
}

/// The available color themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChoice {
    Dark,
    Light,
    Sepia,
}

impl ThemeChoice {
    /// All themes, in display order.
    pub const ALL: [Self; 3] = [Self::Dark, Self::Light, Self::Sepia];

    /// The human readable name of the theme.
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::Sepia => "Sepia",
        }
    }

    /// The color palette of the theme.
    ///
    /// This is allocation free and mirrors the palette that `iced` uses when
    /// the same theme is installed on the application.
    pub fn palette(self) -> Palette {
        match self {
            Self::Dark => Palette::DARK,
            Self::Light => Palette::LIGHT,
            Self::Sepia => Palette {
                background: Color::from_rgb8(0xF4, 0xEC, 0xD8),
                text: Color::from_rgb8(0x3B, 0x31, 0x24),
                primary: Color::from_rgb8(0x8A, 0x63, 0x3A),
                success: Color::from_rgb8(0x4E, 0x7A, 0x46),
                danger: Color::from_rgb8(0xA3, 0x3B, 0x2E),
            },
        }
    }

    /// Builds the `iced` theme that the application should render with.
    pub fn theme(self) -> Theme {
        match self {
            Self::Dark => Theme::Dark,
            Self::Light => Theme::Light,
            Self::Sepia => Theme::custom(String::from("Sepia"), self.palette()),
        }
    }
}

/// Typography preferences shared by the setup and writer screens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Typography {
    pub font_size: f32,
    pub line_height: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            font_size: 20.0,
            line_height: 1.6,
        }
    }
}

/// Everything the writer screen needs to know about how a session is run.
#[derive(Debug, Clone, Copy)]
pub struct SessionSettings {
    pub goal: SessionTarget,
    pub typography: Typography,
    pub theme: ThemeChoice,
    pub font: FontChoice,
    /// Whether the document, goal, autosave and shortcut readouts are shown.
    pub show_info: bool,
    /// Whether only the sentence under the cursor stays fully opaque.
    pub focus_blur: bool,
    /// Whether the window should go full screen for the session.
    pub fullscreen: bool,
}

/// State of the setup screen.
#[derive(Debug, Clone)]
pub struct SetupState {
    /// The document that will receive the writing.
    pub target_path: Option<PathBuf>,
    /// The document contents, once they have been loaded.
    pub document_text: Option<String>,
    /// Whether the document is currently being read from disk.
    pub is_loading: bool,
    /// Whether the goal is expressed in minutes or words.
    pub goal_mode: GoalMode,
    /// The raw text inside the goal input.
    pub goal_input: String,
    /// The parsed goal of the upcoming session.
    pub goal: SessionTarget,
    /// Typography preferences for the session.
    pub typography: Typography,
    /// Typeface preference for the session.
    pub font: FontChoice,
    /// Theme preference for the session.
    pub theme: ThemeChoice,
    /// Whether the window should go full screen when the session starts.
    pub fullscreen: bool,
    /// Whether writer information should be visible during the session.
    pub show_info: bool,
    /// Whether everything except the sentence under the cursor is dimmed.
    pub focus_blur: bool,
    /// A validation or I/O error to show beneath the form.
    pub error: Option<String>,
}

impl Default for SetupState {
    fn default() -> Self {
        Self {
            target_path: None,
            document_text: None,
            is_loading: false,
            goal_mode: GoalMode::Minutes,
            goal_input: String::from(GoalMode::Minutes.default_input()),
            goal: SessionTarget::Duration(25),
            typography: Typography::default(),
            font: FontChoice::default(),
            theme: ThemeChoice::Dark,
            fullscreen: false,
            show_info: true,
            focus_blur: true,
            error: None,
        }
    }
}

impl SetupState {
    /// The settings that the next writing session should use.
    pub fn settings(&self) -> SessionSettings {
        SessionSettings {
            goal: self.goal,
            typography: self.typography,
            theme: self.theme,
            font: self.font,
            show_info: self.show_info,
            focus_blur: self.focus_blur,
            fullscreen: self.fullscreen,
        }
    }

    /// Rebuilds the setup screen from an active writing session, so that
    /// leaving a session never loses the current document or its preferences.
    pub fn from_writer(writer: &WriterState) -> Self {
        let (goal_mode, goal_input) = match writer.goal {
            SessionTarget::Duration(minutes) => (GoalMode::Minutes, minutes.to_string()),
            SessionTarget::WordCount(words) => (GoalMode::Words, words.to_string()),
        };

        Self {
            target_path: Some(writer.target_path.clone()),
            document_text: Some(writer.content.text()),
            is_loading: false,
            goal_mode,
            goal_input,
            goal: writer.goal,
            typography: writer.typography,
            font: writer.font,
            theme: writer.theme,
            fullscreen: writer.fullscreen,
            show_info: writer.show_info,
            focus_blur: writer.focus_blur,
            error: None,
        }
    }
}

/// State of the writer screen.
pub struct WriterState {
    /// The live content of the text editor.
    pub content: text_editor::Content,
    /// The file that receives automatic saves.
    pub target_path: PathBuf,
    /// The goal of this session.
    pub goal: SessionTarget,
    /// Seconds left in a duration based session.
    pub remaining_seconds: u64,
    /// Words currently present in the document.
    pub words_in_document: usize,
    /// New words written since the session started.
    pub words_written: usize,
    /// New words written since the last automatic save.
    pub words_since_last_save: usize,
    /// Word count of the document when it was last measured.
    pub last_word_count: usize,
    /// Whether the document has edits that are not on disk yet.
    pub dirty: bool,
    /// Seconds since the last edit, used for the idle autosave.
    pub idle_seconds: u64,
    /// How many background saves are currently in flight.
    pub pending_saves: usize,
    /// The last save error, if any.
    pub save_error: Option<String>,
    /// Whether the session goal has been reached.
    pub session_complete: bool,
    /// Whether the floating toast is visible.
    pub show_timer_toast: bool,
    /// The text shown inside the floating toast.
    pub toast_message: String,
    /// Remaining toast lifetime in one-second ticks.
    pub toast_ticks_left: u8,
    /// Typography preferences for this session.
    pub typography: Typography,
    /// Typeface used to render the session.
    pub font: FontChoice,
    /// Theme preference for this session.
    pub theme: ThemeChoice,
    /// Whether the information readouts are visible.
    pub show_info: bool,
    /// Whether only the sentence under the cursor stays fully opaque.
    pub focus_blur: bool,
    /// Whether this session started in full screen.
    pub fullscreen: bool,
}

impl WriterState {
    /// Starts a new session for the given document.
    pub fn new(target_path: PathBuf, document_text: String, settings: SessionSettings) -> Self {
        let words_in_document = count_words(&document_text);

        Self {
            content: text_editor::Content::with_text(&document_text),
            target_path,
            goal: settings.goal,
            remaining_seconds: match settings.goal {
                SessionTarget::Duration(minutes) => u64::from(minutes) * 60,
                SessionTarget::WordCount(_) => 0,
            },
            words_in_document,
            words_written: 0,
            words_since_last_save: 0,
            last_word_count: words_in_document,
            dirty: false,
            idle_seconds: 0,
            pending_saves: 0,
            save_error: None,
            session_complete: false,
            show_timer_toast: false,
            toast_message: String::new(),
            toast_ticks_left: 0,
            typography: settings.typography,
            font: settings.font,
            theme: settings.theme,
            show_info: settings.show_info,
            focus_blur: settings.focus_blur,
            fullscreen: settings.fullscreen,
        }
    }

    /// Makes the floating toast visible for a few seconds.
    pub fn show_toast(&mut self, message: impl Into<String>) {
        self.toast_message = message.into();
        self.toast_ticks_left = TOAST_TICKS;
        self.show_timer_toast = true;
    }

    /// The file name of the bound document.
    pub fn file_name(&self) -> String {
        self.target_path
            .file_name()
            .and_then(|name| name.to_str())
            .map_or_else(|| String::from("Untitled"), String::from)
    }
}

/// Counts the words in a piece of text.
pub fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Blends two colors (`amount` of `0.0` yields `from`, `1.0` yields `to`).
pub fn blend(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);

    Color {
        r: from.r + (to.r - from.r) * amount,
        g: from.g + (to.g - from.g) * amount,
        b: from.b + (to.b - from.b) * amount,
        a: from.a + (to.a - from.a) * amount,
    }
}

/// A de-emphasized variant of the palette's text color.
pub fn muted(palette: Palette) -> Color {
    blend(palette.text, palette.background, 0.42)
}
