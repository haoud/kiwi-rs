//! The colors of the console: a dark theme inspired by the Catppuccin Mocha
//! palette, using 24-bit colors.
use ratatui::style::{Color, Modifier, Style};

/// The background of the whole screen.
pub const BACKGROUND: Color = Color::Rgb(0x1E, 0x1E, 0x2E);

/// A slightly darker background, used for the header and the footer.
pub const BACKGROUND_DARK: Color = Color::Rgb(0x18, 0x18, 0x25);

/// The background of highlighted elements.
pub const SURFACE: Color = Color::Rgb(0x31, 0x32, 0x44);

/// The borders of the panels.
pub const BORDER: Color = Color::Rgb(0x45, 0x47, 0x5A);

/// Regular text.
pub const TEXT: Color = Color::Rgb(0xCD, 0xD6, 0xF4);

/// Secondary text (labels, hints...).
pub const MUTED: Color = Color::Rgb(0x7F, 0x84, 0x9C);

/// The main accent color (titles, prompt...).
pub const ACCENT: Color = Color::Rgb(0xCB, 0xA6, 0xF7);

/// Informative elements (names, values...).
pub const INFO: Color = Color::Rgb(0x89, 0xB4, 0xFA);

/// Secondary informative elements.
pub const TEAL: Color = Color::Rgb(0x94, 0xE2, 0xD5);

/// Success and low usage.
pub const SUCCESS: Color = Color::Rgb(0xA6, 0xE3, 0xA1);

/// Warnings and high usage.
pub const WARNING: Color = Color::Rgb(0xF9, 0xE2, 0xAF);

/// Errors and critical usage.
pub const ERROR: Color = Color::Rgb(0xF3, 0x8B, 0xA8);

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
    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
}
