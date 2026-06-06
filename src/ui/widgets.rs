//! Small shared UI building blocks. Currently a hand-rolled single-line text input
//! (avoids pulling in a text-widget dependency for simple fields).

use ratatui::crossterm::event::KeyCode;

/// A single-line editable text field with a character cursor.
#[derive(Debug, Default, Clone)]
pub struct TextField {
    pub value: String,
    /// Cursor position as a character index (0..=char_count).
    pub cursor: usize,
}

impl TextField {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-fill with `value`, cursor at the end.
    pub fn with(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self { value, cursor }
    }

    fn byte_at(&self, char_idx: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.value.len())
    }

    fn char_len(&self) -> usize {
        self.value.chars().count()
    }

    pub fn insert(&mut self, ch: char) {
        let at = self.byte_at(self.cursor);
        self.value.insert(at, ch);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let start = self.byte_at(self.cursor - 1);
            let end = self.byte_at(self.cursor);
            self.value.replace_range(start..end, "");
            self.cursor -= 1;
        }
    }

    /// Handle an editing key. Returns true if the key was consumed.
    pub fn handle(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char(c) => {
                self.insert(c);
                true
            }
            KeyCode::Backspace => {
                self.backspace();
                true
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                true
            }
            KeyCode::Right => {
                if self.cursor < self.char_len() {
                    self.cursor += 1;
                }
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::End => {
                self.cursor = self.char_len();
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_backspace() {
        let mut f = TextField::new();
        for c in "abc".chars() {
            f.insert(c);
        }
        assert_eq!(f.value, "abc");
        assert_eq!(f.cursor, 3);
        f.backspace();
        assert_eq!(f.value, "ab");
        assert_eq!(f.cursor, 2);
    }

    #[test]
    fn insert_at_cursor() {
        let mut f = TextField::with("ac");
        f.cursor = 1;
        f.insert('b');
        assert_eq!(f.value, "abc");
        assert_eq!(f.cursor, 2);
    }

    #[test]
    fn cursor_movement_is_bounded() {
        let mut f = TextField::with("hi");
        f.handle(KeyCode::Right);
        f.handle(KeyCode::Right);
        f.handle(KeyCode::Right); // clamped
        assert_eq!(f.cursor, 2);
        f.handle(KeyCode::Home);
        assert_eq!(f.cursor, 0);
        f.handle(KeyCode::Left); // clamped
        assert_eq!(f.cursor, 0);
    }
}
