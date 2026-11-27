use core::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use console::input::{Input, Key, KeyEvent, Modifiers, Ticker};

use crate::{
    driver::keyboard::{
        self, DecodedKey, EventDecoder, HandleControl, KeyCode,
        layout::{self, Layout},
    },
    future::{self, sleep::Sleep},
    time::{duration::Duration, instant::Instant},
};

/// The key presses of the keyboard, translated with the current layout.
pub struct KeyboardInput {
    /// Tracks the modifier keys and translates the key events into
    /// characters. Control combinations are translated into the ASCII control
    /// characters (Ctrl+C gives U+0003), which tells the console that Ctrl
    /// was held.
    decoder: EventDecoder<pc_keyboard::layouts::AnyLayout>,

    /// The layout used by the decoder. It is compared with the current layout
    /// at each event, to follow the changes made with the `keyb` command.
    layout: Layout,
}

impl KeyboardInput {
    /// Creates a new input using the current keyboard layout.
    pub fn new() -> Self {
        let layout = layout::current();
        Self {
            decoder: EventDecoder::new(layout.to_any(), HandleControl::MapLettersToUnicode),
            layout,
        }
    }

    /// Converts a keyboard event into a console key press, or returns `None`
    /// if the event is a key release or a key the console does not use.
    fn convert(&mut self, event: keyboard::KeyEvent) -> Option<KeyEvent> {
        // Follow the changes of the layout (e.g. with the `keyb` command).
        let current = layout::current();
        if current != self.layout {
            self.decoder.change_layout(current.to_any());
            self.layout = current;
        }

        let key = match self.decoder.process_keyevent(event)? {
            DecodedKey::Unicode(ch) => return Some(convert_char(ch)),
            DecodedKey::RawKey(code) => match code {
                KeyCode::ArrowUp => Key::Up,
                KeyCode::ArrowDown => Key::Down,
                KeyCode::ArrowLeft => Key::Left,
                KeyCode::ArrowRight => Key::Right,
                KeyCode::Home => Key::Home,
                KeyCode::End => Key::End,
                KeyCode::PageUp => Key::PageUp,
                KeyCode::PageDown => Key::PageDown,
                KeyCode::Insert => Key::Insert,
                KeyCode::F1 => Key::Function(1),
                KeyCode::F2 => Key::Function(2),
                KeyCode::F3 => Key::Function(3),
                KeyCode::F4 => Key::Function(4),
                KeyCode::F5 => Key::Function(5),
                KeyCode::F6 => Key::Function(6),
                KeyCode::F7 => Key::Function(7),
                KeyCode::F8 => Key::Function(8),
                KeyCode::F9 => Key::Function(9),
                KeyCode::F10 => Key::Function(10),
                KeyCode::F11 => Key::Function(11),
                KeyCode::F12 => Key::Function(12),
                _ => return None,
            },
        };
        Some(KeyEvent::new(key))
    }
}

/// Converts a character produced by the layout into a console key press. The
/// layouts produce ASCII control characters for some keys (Enter, Escape...)
/// and for the Ctrl+letter combinations.
fn convert_char(ch: char) -> KeyEvent {
    let key = match ch {
        '\n' | '\r' => Key::Enter,
        '\t' => Key::Tab,
        '\u{8}' => Key::Backspace,
        '\u{1b}' => Key::Escape,
        '\u{7f}' => Key::Delete,
        '\u{1}'..='\u{1a}' => {
            // Ctrl+A gives U+0001, Ctrl+B gives U+0002, and so on.
            let letter = char::from(b'a' + (ch as u8) - 1);
            return KeyEvent {
                key: Key::Char(letter),
                modifiers: Modifiers {
                    ctrl: true,
                    ..Modifiers::default()
                },
            };
        }
        ch => Key::Char(ch),
    };
    KeyEvent::new(key)
}

impl Input for KeyboardInput {
    /// Returns the next key press, skipping the events that do not produce a
    /// key press for the console (key releases, modifier keys...).
    fn poll_key(&mut self, cx: &mut Context<'_>) -> Poll<KeyEvent> {
        loop {
            let event = ready!(keyboard::poll_event(cx));
            if let Some(key) = self.convert(event) {
                return Poll::Ready(key);
            }
        }
    }
}

/// A ticker firing at a fixed period, based on the timers of the kernel. If
/// the console is late, the missed ticks are skipped instead of firing in a
/// burst.
pub struct Interval {
    /// The time between two ticks.
    period: Duration,

    /// The deadline of the next tick.
    next: Instant,

    /// The future waiting for the next tick, created by the first poll after
    /// a tick. It is kept between polls so that the timer is registered only
    /// once per tick.
    sleep: Option<Sleep>,
}

impl Interval {
    /// Creates a ticker firing for the first time after one period.
    pub fn new(period: Duration) -> Self {
        Self {
            period,
            next: Instant::now() + period,
            sleep: None,
        }
    }
}

impl Ticker for Interval {
    /// Returns `Poll::Ready` once the deadline of the next tick is reached.
    fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        // Sleep until the deadline of the next tick, and not for a whole
        // period, so that the time spent by the console between two ticks
        // does not delay the following ticks.
        let remaining = self.next.duration_since(Instant::now());
        let sleep = self.sleep.get_or_insert_with(|| future::sleep(remaining));
        ready!(Pin::new(sleep).poll(cx));

        // Compute the deadline of the next tick. If the console was so late
        // that this deadline is already over, restart from now.
        self.sleep = None;
        let now = Instant::now();
        self.next += self.period;
        if self.next <= now {
            self.next = now + self.period;
        }
        Poll::Ready(())
    }
}
