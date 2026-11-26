/// One of the 16 colors of the classic VGA text mode palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Color {
    Black,
    Blue,
    Green,
    Cyan,
    Red,
    Magenta,
    Brown,
    LightGray,
    DarkGray,
    LightBlue,
    LightGreen,
    LightCyan,
    LightRed,
    LightMagenta,
    Yellow,
    White,
}

impl Color {
    /// Returns the red, green and blue components of the color, using the
    /// values of the standard VGA palette.
    #[must_use]
    pub const fn rgb(self) -> (u8, u8, u8) {
        match self {
            Color::Black => (0x00, 0x00, 0x00),
            Color::Blue => (0x00, 0x00, 0xAA),
            Color::Green => (0x00, 0xAA, 0x00),
            Color::Cyan => (0x00, 0xAA, 0xAA),
            Color::Red => (0xAA, 0x00, 0x00),
            Color::Magenta => (0xAA, 0x00, 0xAA),
            Color::Brown => (0xAA, 0x55, 0x00),
            Color::LightGray => (0xAA, 0xAA, 0xAA),
            Color::DarkGray => (0x55, 0x55, 0x55),
            Color::LightBlue => (0x55, 0x55, 0xFF),
            Color::LightGreen => (0x55, 0xFF, 0x55),
            Color::LightCyan => (0x55, 0xFF, 0xFF),
            Color::LightRed => (0xFF, 0x55, 0x55),
            Color::LightMagenta => (0xFF, 0x55, 0xFF),
            Color::Yellow => (0xFF, 0xFF, 0x55),
            Color::White => (0xFF, 0xFF, 0xFF),
        }
    }

    /// Returns the color used to display a load or usage ratio, expressed in
    /// per mille: green when low, yellow when high, and red when critical.
    #[must_use]
    pub const fn for_usage(permille: u32) -> Self {
        if permille < 600 {
            Color::LightGreen
        } else if permille < 850 {
            Color::Yellow
        } else {
            Color::LightRed
        }
    }
}
