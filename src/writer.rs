//! Writer screen: the distraction-free editor, its timer and its autosave.

use std::time::Duration;

use iced::keyboard::{key, Key};
use iced::theme::Palette;
use iced::widget::{container, horizontal_space, row, text, text_editor, Column, Space};
use iced::{Alignment, Border, Color, Length, Subscription, Task};

use crate::focus::{self, FocusHighlighter, FocusSettings};
use crate::toast;
use crate::types::{
    blend, count_words, muted, Message, SessionTarget, WriterState, AUTO_SAVE_IDLE_SECONDS,
    AUTO_SAVE_WORD_DELTA,
};

/// Convenience alias for this screen's widgets.
type Element<'a> = iced::Element<'a, Message>;

/// Handles every message that belongs to the writer screen.
pub fn update(state: &mut WriterState, message: Message) -> Task<Message> {
    match message {
        Message::EditorAction(action) => {
            let is_edit = action.is_edit();
            state.content.perform(action);

            if is_edit {
                state.dirty = true;
                state.idle_seconds = 0;
                track_session(state)
            } else {
                Task::none()
            }
        }
        Message::Tick => {
            advance_toast(state);
            advance_timer(state);
            advance_idle_autosave(state)
        }
        Message::SaveFinished(result) => {
            state.pending_saves = state.pending_saves.saturating_sub(1);

            match result {
                Ok(()) => state.save_error = None,
                Err(error) => {
                    state.save_error = Some(error.clone());
                    state.dirty = true;
                    state.show_toast(format!("Could not save: {error}"));
                }
            }

            Task::none()
        }
        _ => Task::none(),
    }
}

/// Re-counts the document, tracks progress and triggers the word autosave.
fn track_session(state: &mut WriterState) -> Task<Message> {
    let words = count_words(&state.content.text());
    state.words_in_document = words;

    if words > state.last_word_count {
        let new_words = words - state.last_word_count;
        state.words_written = state.words_written.saturating_add(new_words);
        state.words_since_last_save = state.words_since_last_save.saturating_add(new_words);
    }

    state.last_word_count = words;

    if let SessionTarget::WordCount(target) = state.goal {
        if !state.session_complete && state.words_written >= target {
            state.session_complete = true;
            state.show_toast(format!("Word goal reached — {target} new words written."));
        }
    }

    if state.words_since_last_save >= AUTO_SAVE_WORD_DELTA {
        state.words_since_last_save = 0;
        state.idle_seconds = 0;
        state.dirty = false;
        return save_now(state);
    }

    Task::none()
}

/// Counts down the toast lifetime by one tick.
fn advance_toast(state: &mut WriterState) {
    if !state.show_timer_toast {
        return;
    }

    state.toast_ticks_left = state.toast_ticks_left.saturating_sub(1);

    if state.toast_ticks_left == 0 {
        state.show_timer_toast = false;
    }
}

/// Counts down the session timer and shows the expiry toast.
fn advance_timer(state: &mut WriterState) {
    if !matches!(state.goal, SessionTarget::Duration(_)) || state.session_complete {
        return;
    }

    state.remaining_seconds = state.remaining_seconds.saturating_sub(1);

    if state.remaining_seconds == 0 {
        state.session_complete = true;
        state.show_toast("Time is up! Your session is complete.");
    }
}

/// Saves the document once it has been left untouched for a while.
fn advance_idle_autosave(state: &mut WriterState) -> Task<Message> {
    if !state.dirty {
        state.idle_seconds = 0;
        return Task::none();
    }

    state.idle_seconds = state.idle_seconds.saturating_add(1);

    if state.idle_seconds >= AUTO_SAVE_IDLE_SECONDS {
        state.idle_seconds = 0;
        state.dirty = false;
        return save_now(state);
    }

    Task::none()
}

/// Saves the current document in the background.
pub fn save_now(state: &mut WriterState) -> Task<Message> {
    state.pending_saves = state.pending_saves.saturating_add(1);
    save_task(state)
}

/// Builds a background save task for the current document.
pub fn save_task(state: &WriterState) -> Task<Message> {
    let path = state.target_path.clone();
    let contents = state.content.text();

    Task::perform(
        async move {
            tokio::fs::write(&path, contents.as_bytes())
                .await
                .map_err(|error| format!("{error}"))
        },
        Message::SaveFinished,
    )
}

/// Returns `true` when the given runtime event is an Escape key press.
pub fn is_escape_event(event: &iced::event::Event) -> bool {
    if let iced::event::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) = event {
        return matches!(key, Key::Named(key::Named::Escape));
    }

    false
}

/// The subscriptions of the writer screen.
pub fn subscription(_state: &WriterState) -> Subscription<Message> {
    Subscription::batch([
        iced::event::listen().map(Message::RuntimeEvent),
        iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick),
    ])
}

/// Renders the writer screen, including the floating toast layer.
pub fn view(state: &WriterState) -> Element<'_> {
    let palette = state.theme.palette();

    let focus = if state.focus_blur {
        let (line, byte) = state.content.cursor_position();

        focus::focus_sentence(&state.content.text(), line, byte)
    } else {
        None
    };

    let editor = text_editor(&state.content)
        .placeholder("Click the page and start writing…")
        .on_action(Message::EditorAction)
        .font(state.font.font())
        .key_binding(|press: text_editor::KeyPress| {
            if matches!(press.key, Key::Named(key::Named::Escape)) {
                Some(text_editor::Binding::Custom(Message::EscapeRequested))
            } else {
                text_editor::Binding::from_key_press(press)
            }
        })
        .padding([28, 30])
        .size(state.typography.font_size)
        .line_height(state.typography.line_height)
        .height(Length::Fill)
        .style(move |_theme, status| editor_style(palette, status))
        .highlight_with::<FocusHighlighter>(FocusSettings { focus }, focus::format);

    let mut page: Column<'_, Message> = Column::new()
        .spacing(12)
        .width(Length::FillPortion(70))
        .height(Length::Fill);

    if state.show_info {
        let (status_label, status_color) = session_status(state, palette);

        let header = row![
            text(state.file_name()).size(15).color(muted(palette)),
            horizontal_space(),
            text(status_label).size(15).color(status_color),
        ]
        .align_y(Alignment::Center);

        page = page.push(header);
    }

    page = page.push(editor);

    if state.show_info {
        let (save_label, save_color) = save_status(state, palette);

        let footer = row![
            text("Esc — back to setup").size(13).color(muted(palette)),
            horizontal_space(),
            text(save_label).size(13).color(save_color),
        ]
        .align_y(Alignment::Center);

        page = page.push(footer);
    }

    let centered = row![
        Space::with_width(Length::FillPortion(15)),
        page,
        Space::with_width(Length::FillPortion(15)),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    let base: Element<'_> = container(centered)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([26, 0])
        .into();

    let notice = state
        .show_timer_toast
        .then_some((state.toast_message.as_str(), palette));

    toast::overlay(base, notice)
}

/// The label and color of the session status readout.
fn session_status(state: &WriterState, palette: Palette) -> (String, Color) {
    match state.goal {
        SessionTarget::Duration(minutes) => {
            let label = format!(
                "{:02}:{:02} left of {minutes} min",
                state.remaining_seconds / 60,
                state.remaining_seconds % 60
            );

            let color = if state.session_complete {
                palette.success
            } else {
                palette.primary
            };

            (label, color)
        }
        SessionTarget::WordCount(target) => {
            let label = format!("{} / {} words", state.words_written, target);

            let color = if state.session_complete {
                palette.success
            } else {
                palette.primary
            };

            (label, color)
        }
    }
}

/// The label and color of the autosave readout.
fn save_status(state: &WriterState, palette: Palette) -> (String, Color) {
    if let Some(error) = &state.save_error {
        (format!("Save failed: {error}"), palette.danger)
    } else if state.pending_saves > 0 {
        (String::from("Saving…"), muted(palette))
    } else if state.dirty {
        (
            format!(
                "{} words  •  autosave: {} new words or {}s idle",
                state.words_in_document, AUTO_SAVE_WORD_DELTA, AUTO_SAVE_IDLE_SECONDS
            ),
            muted(palette),
        )
    } else {
        (
            format!("Saved  •  {} words", state.words_in_document),
            muted(palette),
        )
    }
}

/// Styling of the text editor. It is deliberately borderless: the page has no
/// visible box around the text.
fn editor_style(palette: Palette, _status: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: palette.background.into(),
        border: Border::default(),
        icon: palette.text,
        placeholder: blend(palette.background, palette.text, 0.42),
        value: palette.text,
        selection: blend(palette.background, palette.primary, 0.30),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::types::{FontChoice, SessionSettings, ThemeChoice, Typography};

    fn session() -> WriterState {
        WriterState::new(
            PathBuf::from("notes.txt"),
            String::from("hello"),
            SessionSettings {
                goal: SessionTarget::WordCount(100),
                typography: Typography::default(),
                theme: ThemeChoice::Dark,
                font: FontChoice::default(),
                show_info: true,
                focus_blur: true,
                fullscreen: false,
            },
        )
    }

    fn tick(state: &mut WriterState) {
        let _ = update(state, Message::Tick);
    }

    #[test]
    fn idle_autosave_triggers_after_ten_seconds() {
        let mut state = session();
        state.dirty = true;

        for _ in 0..AUTO_SAVE_IDLE_SECONDS - 1 {
            tick(&mut state);
        }

        assert!(state.dirty);
        assert_eq!(state.pending_saves, 0);

        tick(&mut state);

        assert!(!state.dirty);
        assert_eq!(state.idle_seconds, 0);
        assert_eq!(state.pending_saves, 1);

        // A further tick must not start a second save.
        tick(&mut state);
        assert_eq!(state.pending_saves, 1);
    }

    #[test]
    fn edits_restart_the_idle_countdown() {
        let mut state = session();
        state.dirty = true;

        for _ in 0..5 {
            tick(&mut state);
        }

        assert_eq!(state.idle_seconds, 5);

        let _ = update(
            &mut state,
            Message::EditorAction(text_editor::Action::Edit(text_editor::Edit::Insert('!'))),
        );

        assert!(state.dirty);
        assert_eq!(state.idle_seconds, 0);

        for _ in 0..AUTO_SAVE_IDLE_SECONDS - 1 {
            tick(&mut state);
        }

        assert!(state.dirty);
        assert_eq!(state.pending_saves, 0);
    }
}
