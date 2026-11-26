//! The keyboard layouts supported by the kernel, and the layout currently
//! used to translate the key events into characters.
use core::sync::atomic::{AtomicU8, Ordering};

use pc_keyboard::layouts::{AnyLayout, Azerty, Us104Key};

/// A keyboard layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Layout {
    /// The US QWERTY layout.
    Us,

    /// The French AZERTY layout.
    Fr,
}

impl Layout {
    /// All the supported layouts.
    pub const ALL: [Layout; 2] = [Layout::Us, Layout::Fr];

    /// Returns the short name of the layout (e.g. `fr`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Layout::Us => "us",
            Layout::Fr => "fr",
        }
    }

    /// Returns a description of the layout.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Layout::Us => "English (US, QWERTY)",
            Layout::Fr => "French (AZERTY)",
        }
    }

    /// Finds a layout by its short name, ignoring the case.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|layout| layout.name().eq_ignore_ascii_case(name))
    }

    /// Returns the `pc-keyboard` implementation of the layout.
    #[must_use]
    pub const fn to_any(self) -> AnyLayout {
        match self {
            Layout::Us => AnyLayout::Us104Key(Us104Key),
            Layout::Fr => AnyLayout::Azerty(Azerty),
        }
    }
}

/// The layout used by the keyboard consumers.
static CURRENT: AtomicU8 = AtomicU8::new(Layout::Fr as u8);

/// Returns the current keyboard layout.
#[must_use]
pub fn current() -> Layout {
    match CURRENT.load(Ordering::Relaxed) {
        0 => Layout::Us,
        _ => Layout::Fr,
    }
}

/// Changes the current keyboard layout.
pub fn set_current(layout: Layout) {
    CURRENT.store(layout as u8, Ordering::Relaxed);
}
