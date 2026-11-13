//! Focus mode: only the sentence under the cursor stays fully opaque.
//!
//! Iced does not expose a post-processing blur for widget content, so the focus
//! effect is implemented with the text editor's highlighter API: every byte
//! outside the current sentence is recolored into a low-contrast blend of the
//! theme colors. This reads like the rest of the page falling out of focus while
//! the text stays fully editable.

use std::ops::Range;

use iced::advanced::text::highlighter::{self, Highlighter};
use iced::{Font, Theme};

use crate::types::blend;

/// How strongly out-of-focus text is blended into the background.
const DIM_AMOUNT: f32 = 0.20;

/// A sentence, located by line and byte offset within that line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SentenceSpan {
    /// Line of the first character of the sentence.
    pub start_line: usize,
    /// Byte offset of the first character within `start_line`.
    pub start_byte: usize,
    /// Line just past the end of the sentence.
    pub end_line: usize,
    /// Byte offset just past the sentence within `end_line`.
    pub end_byte: usize,
}

/// Settings of the focus highlighter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusSettings {
    /// The sentence that stays fully opaque.
    pub focus: Option<SentenceSpan>,
}

/// Dims every byte outside the focus sentence.
#[derive(Debug)]
pub struct FocusHighlighter {
    focus: Option<SentenceSpan>,
    line: usize,
}

impl Highlighter for FocusHighlighter {
    type Settings = FocusSettings;
    type Highlight = ();

    type Iterator<'a>
        = std::vec::IntoIter<(Range<usize>, ())>
    where
        Self: 'a;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            focus: settings.focus,
            line: if settings.focus.is_some() {
                0
            } else {
                usize::MAX
            },
        }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        let first_changed = match (&self.focus, &new_settings.focus) {
            // Both sentences are known: only the lines from the earliest start
            // of either sentence can have changed their dimming.
            (Some(old), Some(new)) => old.start_line.min(new.start_line),
            // Nothing changed.
            (None, None) => return,
            // Entering or leaving focus mode changes every single line.
            _ => 0,
        };

        self.focus = new_settings.focus;
        self.line = self.line.min(first_changed);
    }

    fn change_line(&mut self, line: usize) {
        if self.focus.is_some() {
            self.line = self.line.min(line);
        }
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let mut spans = Vec::new();

        if let Some(focus) = self.focus {
            let len = line.len();
            let (start, end) = focus_on_line(focus, self.line, len);

            if start > 0 {
                spans.push((0..start, ()));
            }

            if end < len {
                spans.push((end..len, ()));
            }
        }

        self.line = self.line.saturating_add(1);

        spans.into_iter()
    }

    fn current_line(&self) -> usize {
        self.line
    }
}

/// The crisp byte range of the focus sentence on the given line.
fn focus_on_line(focus: SentenceSpan, line: usize, len: usize) -> (usize, usize) {
    if line < focus.start_line || line > focus.end_line {
        return (0, 0);
    }

    let start = if line == focus.start_line {
        focus.start_byte.min(len)
    } else {
        0
    };

    let end = if line == focus.end_line {
        focus.end_byte.min(len)
    } else {
        len
    };

    (start, end.max(start))
}

/// Turns a focus highlight into an iced text format.
pub fn format(_highlight: &(), theme: &Theme) -> highlighter::Format<Font> {
    let palette = theme.palette();

    highlighter::Format {
        color: Some(blend(palette.background, palette.text, DIM_AMOUNT)),
        font: None,
    }
}

/// Finds the sentence that contains the cursor position.
///
/// `cursor_byte` is the first-byte index of the glyph at the cursor, as
/// reported by `text_editor::Content::cursor_position`.
pub fn focus_sentence(text: &str, cursor_line: usize, cursor_byte: usize) -> Option<SentenceSpan> {
    let sentences = sentence_spans(text);

    sentences
        .iter()
        .copied()
        .find(|sentence| contains(sentence, cursor_line, cursor_byte))
        .or_else(|| {
            sentences.iter().copied().rev().find(|sentence| {
                (sentence.end_line, sentence.end_byte) <= (cursor_line, cursor_byte)
            })
        })
        .or_else(|| sentences.first().copied())
}

/// Whether the sentence contains the given cursor position.
fn contains(sentence: &SentenceSpan, line: usize, byte: usize) -> bool {
    if line < sentence.start_line || line > sentence.end_line {
        return false;
    }

    if line == sentence.start_line && byte < sentence.start_byte {
        return false;
    }

    if line == sentence.end_line && byte > sentence.end_byte {
        return false;
    }

    true
}

/// Splits text into sentences, tracking line and byte offsets.
///
/// A sentence ends at terminal punctuation (`!`, `?`, `…`) or a period, and
/// also at a blank line, which is treated as a paragraph break.
fn sentence_spans(text: &str) -> Vec<SentenceSpan> {
    let mut spans = Vec::new();
    let mut line = 0usize;
    let mut byte = 0usize;
    let mut start: Option<(usize, usize)> = None;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\n' => {
                let blank_line = matches!(chars.peek(), Some(&'\n'));

                if blank_line {
                    if let Some((start_line, start_byte)) = start.take() {
                        spans.push(SentenceSpan {
                            start_line,
                            start_byte,
                            end_line: line,
                            end_byte: byte,
                        });
                    }
                }

                line += 1;
                byte = 0;
            }
            '.' | '!' | '?' | '…' => {
                if let Some((start_line, start_byte)) = start.take() {
                    spans.push(SentenceSpan {
                        start_line,
                        start_byte,
                        end_line: line,
                        end_byte: byte + c.len_utf8(),
                    });
                }

                byte += c.len_utf8();
            }
            c => {
                if !c.is_whitespace() && start.is_none() {
                    start = Some((line, byte));
                }

                byte += c.len_utf8();
            }
        }
    }

    if let Some((start_line, start_byte)) = start {
        spans.push(SentenceSpan {
            start_line,
            start_byte,
            end_line: line,
            end_byte: byte,
        });
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(
        start_line: usize,
        start_byte: usize,
        end_line: usize,
        end_byte: usize,
    ) -> SentenceSpan {
        SentenceSpan {
            start_line,
            start_byte,
            end_line,
            end_byte,
        }
    }

    #[test]
    fn sentences_are_split_on_terminators_and_paragraph_breaks() {
        let spans = sentence_spans("One. Two! Three?\nFour.");

        assert_eq!(
            spans,
            vec![
                span(0, 0, 0, 4),
                span(0, 5, 0, 9),
                span(0, 10, 0, 16),
                span(1, 0, 1, 5),
            ]
        );
    }

    #[test]
    fn the_cursor_selects_the_sentence_it_sits_in() {
        let text = "One. Two! Three?\nFour.";

        assert_eq!(focus_sentence(text, 0, 0), Some(span(0, 0, 0, 4)));
        assert_eq!(focus_sentence(text, 0, 4), Some(span(0, 0, 0, 4)));
        assert_eq!(focus_sentence(text, 0, 5), Some(span(0, 5, 0, 9)));
        assert_eq!(focus_sentence(text, 0, 7), Some(span(0, 5, 0, 9)));
        assert_eq!(focus_sentence(text, 0, 12), Some(span(0, 10, 0, 16)));
        assert_eq!(focus_sentence(text, 1, 2), Some(span(1, 0, 1, 5)));
    }

    #[test]
    fn whitespace_between_sentences_keeps_the_previous_sentence() {
        let text = "First sentence.\n\nSecond paragraph.";

        assert_eq!(focus_sentence(text, 1, 0), Some(span(0, 0, 0, 15)));
    }

    #[test]
    fn the_highlighter_dims_only_outside_the_sentence() {
        let mut highlighter = FocusHighlighter::new(&FocusSettings {
            focus: Some(span(0, 5, 0, 9)),
        });

        let spans: Vec<_> = highlighter.highlight_line("One. Two! Three?").collect();

        assert_eq!(spans, vec![(0..5, ()), (9..16, ())]);
        assert_eq!(highlighter.current_line(), 1);
    }

    #[test]
    fn multi_line_sentences_keep_the_middle_lines_clear() {
        let focus = span(0, 4, 2, 3);

        assert_eq!(focus_on_line(focus, 0, 10), (4, 10));
        assert_eq!(focus_on_line(focus, 1, 10), (0, 10));
        assert_eq!(focus_on_line(focus, 2, 10), (0, 3));
        assert_eq!(focus_on_line(focus, 3, 10), (0, 0));
    }

    #[test]
    fn empty_documents_have_no_focus() {
        assert_eq!(focus_sentence("", 0, 0), None);
    }
}
