//! The colors of the console: a dark theme based on the Gruvbox Dark palette,
//! using 24-bit colors.
use ratatui::style::{Color, Modifier, Style};

/// The background of the whole screen.
pub const BACKGROUND: Color = Color::Rgb(0x28, 0x28, 0x28);

/// A slightly darker background, used for the header and the footer.
pub const BACKGROUND_DARK: Color = Color::Rgb(0x1D, 0x20, 0x21);

/// The background of highlighted elements.
pub const SURFACE: Color = Color::Rgb(0x3C, 0x38, 0x36);

/// The borders of the panels.
pub const BORDER: Color = Color::Rgb(0x50, 0x49, 0x45);

/// Regular text.
pub const TEXT: Color = Color::Rgb(0xEB, 0xDB, 0xB2);

/// Secondary text (labels, hints...).
pub const MUTED: Color = Color::Rgb(0x92, 0x83, 0x74);

/// The accent color, used sparingly (header badge and prompt).
pub const ACCENT: Color = Color::Rgb(0xFE, 0x80, 0x19);

/// The titles of the panels.
pub const TITLE: Color = Color::Rgb(0xD5, 0xC4, 0xA1);

/// Informative elements (names, values...).
pub const INFO: Color = Color::Rgb(0x83, 0xA5, 0x98);

/// Secondary informative elements.
pub const TEAL: Color = Color::Rgb(0x8E, 0xC0, 0x7C);

/// Success and low usage.
pub const SUCCESS: Color = Color::Rgb(0xB8, 0xBB, 0x26);

/// Warnings and high usage.
pub const WARNING: Color = Color::Rgb(0xFA, 0xBD, 0x2F);

/// Errors and critical usage.
pub const ERROR: Color = Color::Rgb(0xFB, 0x49, 0x34);

/// Returns the color used to display a load or usage ratio, expressed in
/// per mille: green when low, yellow when high, and red when critical.
#[must_use]
pub const fn usage(permille: u32) -> Color {
    if permille < 600 {
        SUCCESS
    } else if permille < 850 {
        WARNING
    } else {
        ERROR
    }
}

/// The style of regular text.
#[must_use]
pub const fn text() -> Style {
    Style::new().fg(TEXT).bg(BACKGROUND)
}

/// The style of secondary text.
#[must_use]
pub const fn muted() -> Style {
    Style::new().fg(MUTED)
}

/// The style of the titles of the panels.
#[must_use]
pub const fn title() -> Style {
    Style::new().fg(TITLE).add_modifier(Modifier::BOLD)
}
