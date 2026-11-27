//! The asynchronous event sources used by the console.
use core::task::{Context, Poll};

/// A key, after the keyboard layout has been applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A key that produces a character.
    Char(char),
    Enter,
    Backspace,
    Delete,
    Insert,
    Tab,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    /// A function key, from F1 to F12.
    Function(u8),
}

/// The state of the modifier keys when a key was pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,

    /// The left alt key is held down (`AltGr` is not a modifier for the
    /// console, since it is used to type characters).
    pub alt: bool,
}

/// A key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub key: Key,

    /// The modifier keys held down when the key was pressed.
    pub modifiers: Modifiers,
}

impl KeyEvent {
    /// Creates a new key press without any modifier.
    #[must_use]
    pub const fn new(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers {
                shift: false,
                ctrl: false,
                alt: false,
            },
        }
    }

    /// Returns true if the given character key was pressed with the control
    /// key held down, ignoring the case of the character.
    #[must_use]
    pub fn is_ctrl(&self, ch: char) -> bool {
        self.modifiers.ctrl && matches!(self.key, Key::Char(c) if c.eq_ignore_ascii_case(&ch))
    }
}

/// An asynchronous source of key presses.
pub trait Input {
    /// Attempts to get the next key press. If no key press is available, the
    /// current task is woken up when a key is pressed.
    ///
    /// Implementations must keep the key presses that occur while nobody is
    /// polling them, so that no key press is lost between two polls.
    fn poll_key(&mut self, cx: &mut Context<'_>) -> Poll<KeyEvent>;
}

/// An asynchronous source of periodic events.
pub trait Ticker {
    /// Returns `Poll::Ready` once per period. If the current period has not
    /// elapsed yet, the current task is woken up when it does.
    fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<()>;
}
