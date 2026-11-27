//! The output of the console: a bounded list of lines that commands write
//! into, and that the user can scroll back through.
use core::fmt;

use alloc::{collections::VecDeque, vec::Vec};
use ratatui::style::Color;

use crate::theme;

/// A character written to the scrollback, with its color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StyledChar {
    pub ch: char,
    pub fg: Color,
    pub bold: bool,
}

/// The lines written to the console. Lines longer than the width of the
/// console are wrapped when they are written, so each line can be displayed
/// on exactly one row.
#[derive(Debug, Clone)]
pub struct Scrollback {
    /// The lines, from the oldest to the newest. The last line is the one
    /// currently being written: it is never displayed if it is empty.
    lines: VecDeque<Vec<StyledChar>>,

    /// The maximum number of characters of a line: longer lines are wrapped.
    width: usize,

    /// The maximum number of lines kept: the oldest lines are forgotten when
    /// new lines are written.
    capacity: usize,

    /// The style of the text written.
    fg: Color,
    bold: bool,
}

impl Scrollback {
    /// Creates an empty scrollback of the given width, that keeps at most
    /// `capacity` lines.
    #[must_use]
    pub fn new(width: u16, capacity: usize) -> Self {
        let mut lines = VecDeque::with_capacity(capacity + 1);
        lines.push_back(Vec::new());
        Self {
            lines,
            width: usize::from(width.max(1)),
            capacity: capacity.max(1),
            fg: theme::TEXT,
            bold: false,
        }
    }

    /// Returns the number of lines that can be displayed.
    #[must_use]
    pub fn len(&self) -> usize {
        if self.lines.back().is_some_and(Vec::is_empty) {
            self.lines.len() - 1
        } else {
            self.lines.len()
        }
    }

    /// Returns true if there is no line to display.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the line at the given index, 0 being the oldest line.
    #[must_use]
    pub fn line(&self, index: usize) -> Option<&[StyledChar]> {
        if index < self.len() {
            self.lines.get(index).map(Vec::as_slice)
        } else {
            None
        }
    }

    /// Removes all the lines.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.lines.push_back(Vec::new());
    }

    /// Changes the color of the text written after this call.
    pub fn set_color(&mut self, fg: Color) {
        self.fg = fg;
    }

    /// Restores the default style of the text.
    pub fn reset_style(&mut self) {
        self.fg = theme::TEXT;
        self.bold = false;
    }

    /// Writes the formatted arguments with the given color, then restores the
    /// previous color.
    pub fn write_colored(&mut self, fg: Color, args: fmt::Arguments<'_>) {
        let previous = self.fg;
        self.fg = fg;
        _ = fmt::Write::write_fmt(self, args);
        self.fg = previous;
    }

    /// Writes the formatted arguments in bold with the given color, then
    /// restores the previous style.
    pub fn write_bold(&mut self, fg: Color, args: fmt::Arguments<'_>) {
        let previous = self.bold;
        self.bold = true;
        self.write_colored(fg, args);
        self.bold = previous;
    }

    /// Ends the current line, unless it is already empty.
    pub fn finish_line(&mut self) {
        if self.lines.back().is_some_and(|line| !line.is_empty()) {
            self.new_line();
        }
    }

    /// Writes a single character. Line feeds start a new line, tabulations
    /// are expanded to the next multiple of 8 columns, and other control
    /// characters are ignored.
    pub fn put_char(&mut self, ch: char) {
        match ch {
            '\n' => self.new_line(),
            '\t' => {
                let column = self.current().len();
                for _ in column..(column / 8 + 1) * 8 {
                    self.push(' ');
                }
            }
            ch if ch.is_control() => {}
            ch => self.push(ch),
        }
    }

    /// Appends a printable character to the current line with the current
    /// style, starting a new line if the current one is full.
    fn push(&mut self, ch: char) {
        if self.current().len() >= self.width {
            self.new_line();
        }
        let styled = StyledChar {
            ch,
            fg: self.fg,
            bold: self.bold,
        };
        self.current().push(styled);
    }

    fn current(&mut self) -> &mut Vec<StyledChar> {
        // There is always at least one line in the scrollback.
        self.lines.back_mut().expect("Scrollback without lines")
    }

    /// Starts a new line, forgetting the oldest line if the scrollback is
    /// full.
    fn new_line(&mut self) {
        if self.lines.len() > self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(Vec::new());
    }
}

impl fmt::Write for Scrollback {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.chars().for_each(|ch| self.put_char(ch));
        Ok(())
    }
}
