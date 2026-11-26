//! Line editing and command history.
use alloc::{collections::VecDeque, string::String, vec::Vec};

/// The list of the last command lines executed, from the oldest to the newest.
#[derive(Debug, Clone)]
pub struct History {
    entries: VecDeque<String>,
    capacity: usize,
}

impl History {
    /// Creates an empty history that keeps at most `capacity` lines.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Adds a line at the end of the history. Blank lines and lines identical
    /// to the last one are ignored, and the oldest line is forgotten when the
    /// history is full.
    pub fn push(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() || self.entries.back().is_some_and(|last| last == line) {
            return;
        }
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(String::from(line));
    }

    /// Returns the number of lines in the history.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the history is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the line at the given index, 0 being the oldest line.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(String::as_str)
    }

    /// Returns an iterator over the lines, from the oldest to the newest.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &str> + ExactSizeIterator {
        self.entries.iter().map(String::as_str)
    }

    /// Forgets all the lines.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// A single line editor, with a cursor and the ability to browse the history.
#[derive(Debug, Clone)]
pub struct LineEditor {
    chars: Vec<char>,
    cursor: usize,
    max_len: usize,

    /// The index of the history line being edited, if the user is browsing
    /// the history, and the line that was being typed before starting to
    /// browse it, restored when going past the newest line.
    browsing: Option<usize>,
    draft: Vec<char>,
}

impl LineEditor {
    /// Creates an empty line editor that accepts lines of at most `max_len`
    /// characters.
    #[must_use]
    pub fn new(max_len: usize) -> Self {
        Self {
            chars: Vec::with_capacity(max_len),
            cursor: 0,
            max_len,
            browsing: None,
            draft: Vec::new(),
        }
    }

    /// Returns the characters of the line.
    #[must_use]
    pub fn chars(&self) -> &[char] {
        &self.chars
    }

    /// Returns the position of the cursor, in characters.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the line as a string.
    #[must_use]
    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    /// Returns true if the cursor is at the end of the line.
    #[must_use]
    pub fn at_end(&self) -> bool {
        self.cursor == self.chars.len()
    }

    /// Inserts a character at the cursor position. Returns false if the line
    /// is full.
    pub fn insert(&mut self, ch: char) -> bool {
        if self.chars.len() >= self.max_len {
            return false;
        }
        self.chars.insert(self.cursor, ch);
        self.cursor += 1;
        true
    }

    /// Inserts a string at the cursor position, stopping if the line is full.
    pub fn insert_str(&mut self, text: &str) {
        for ch in text.chars() {
            if !self.insert(ch) {
                break;
            }
        }
    }

    /// Removes the character before the cursor.
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
    }

    /// Removes the character under the cursor.
    pub fn delete(&mut self) {
        if self.cursor < self.chars.len() {
            self.chars.remove(self.cursor);
        }
    }

    /// Moves the cursor one character to the left.
    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Moves the cursor one character to the right.
    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.chars.len());
    }

    /// Moves the cursor to the beginning of the line.
    pub fn home(&mut self) {
        self.cursor = 0;
    }

    /// Moves the cursor to the end of the line.
    pub fn end(&mut self) {
        self.cursor = self.chars.len();
    }

    /// Clears the line.
    pub fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
        self.browsing = None;
    }

    /// Returns the line and clears the editor.
    pub fn take(&mut self) -> String {
        let line = self.text();
        self.clear();
        line
    }

    /// Replaces the line with the previous line of the history.
    pub fn history_previous(&mut self, history: &History) {
        let index = match self.browsing {
            None if history.is_empty() => return,
            None => {
                self.draft.clone_from(&self.chars);
                history.len() - 1
            }
            Some(0) => return,
            Some(index) => index - 1,
        };
        self.browsing = Some(index);
        self.replace(history.get(index).unwrap_or_default().chars());
    }

    /// Replaces the line with the next line of the history, or with the line
    /// that was being typed when going past the newest line.
    pub fn history_next(&mut self, history: &History) {
        let Some(index) = self.browsing else {
            return;
        };
        if index + 1 < history.len() {
            self.browsing = Some(index + 1);
            self.replace(history.get(index + 1).unwrap_or_default().chars());
        } else {
            self.browsing = None;
            let draft = core::mem::take(&mut self.draft);
            self.replace(draft.into_iter());
        }
    }

    fn replace(&mut self, chars: impl Iterator<Item = char>) {
        self.chars.clear();
        self.chars.extend(chars.take(self.max_len));
        self.cursor = self.chars.len();
    }
}
