use core::convert::Infallible;

use console::ratatui::{
    backend::{Backend, ClearType, WindowSize},
    buffer::Cell,
    layout::{Position, Size},
    style::{Color, Modifier},
};

use crate::video::{Font, Framebuffer, Rgb};

/// The foreground color used for [`Color::Reset`].
const DEFAULT_FG: Rgb = Rgb::new(0xC0, 0xC0, 0xC0);

/// The background color used for [`Color::Reset`].
const DEFAULT_BG: Rgb = Rgb::BLACK;

/// The 16 colors of the ANSI palette, used for the named colors and the first
/// indexed colors (the values of the xterm palette).
const ANSI: [Rgb; 16] = [
    Rgb::new(0x00, 0x00, 0x00),
    Rgb::new(0xCD, 0x00, 0x00),
    Rgb::new(0x00, 0xCD, 0x00),
    Rgb::new(0xCD, 0xCD, 0x00),
    Rgb::new(0x00, 0x00, 0xEE),
    Rgb::new(0xCD, 0x00, 0xCD),
    Rgb::new(0x00, 0xCD, 0xCD),
    Rgb::new(0xE5, 0xE5, 0xE5),
    Rgb::new(0x7F, 0x7F, 0x7F),
    Rgb::new(0xFF, 0x00, 0x00),
    Rgb::new(0x00, 0xFF, 0x00),
    Rgb::new(0xFF, 0xFF, 0x00),
    Rgb::new(0x5C, 0x5C, 0xFF),
    Rgb::new(0xFF, 0x00, 0xFF),
    Rgb::new(0x00, 0xFF, 0xFF),
    Rgb::new(0xFF, 0xFF, 0xFF),
];

/// A Ratatui backend drawing the cells of the terminal on the framebuffer,
/// with 24-bit colors and a regular and a bold font. The grid of cells is
/// centered on the framebuffer if its size is not a multiple of the size of
/// the glyphs.
///
/// The backend does not draw any cursor: the console draws its own cursor in
/// the cells.
pub struct FramebufferBackend {
    framebuffer: Framebuffer,
    regular: Font,

    /// The font used for bold text, with glyphs of the same size as the
    /// regular font.
    bold: Font,

    /// The size of the grid of cells.
    columns: u16,
    rows: u16,

    /// The position in pixels of the top left corner of the grid.
    origin: (usize, usize),

    /// The position of the terminal cursor. It is only remembered for
    /// Ratatui, since the backend does not draw it.
    cursor: Position,
}

impl FramebufferBackend {
    /// Creates a backend covering the whole framebuffer, and clears it. Both
    /// fonts must have glyphs of the same size.
    pub fn new(mut framebuffer: Framebuffer, regular: Font, bold: Font) -> Self {
        assert!(
            regular.width() == bold.width() && regular.height() == bold.height(),
            "The regular and bold fonts must have the same size"
        );
        let columns = framebuffer.width() / regular.width();
        let rows = framebuffer.height() / regular.height();
        let origin = (
            (framebuffer.width() - columns * regular.width()) / 2,
            (framebuffer.height() - rows * regular.height()) / 2,
        );

        framebuffer.clear(DEFAULT_BG);
        Self {
            columns: u16::try_from(columns).unwrap_or(u16::MAX),
            rows: u16::try_from(rows).unwrap_or(u16::MAX),
            framebuffer,
            regular,
            bold,
            origin,
            cursor: Position::ORIGIN,
        }
    }

    /// Returns the position in pixels of the top left corner of a cell.
    fn pixel_position(&self, x: u16, y: u16) -> (usize, usize) {
        (
            self.origin.0 + usize::from(x) * self.regular.width(),
            self.origin.1 + usize::from(y) * self.regular.height(),
        )
    }

    /// Draws a cell: its symbol with the font matching its style, with its
    /// colors and modifiers. Only the first character of the symbol is drawn,
    /// since the fonts do not support combining characters.
    fn draw_cell(&mut self, x: u16, y: u16, cell: &Cell) {
        if x >= self.columns || y >= self.rows {
            return;
        }

        let modifier = cell.modifier;
        let mut fg = rgb(cell.fg, DEFAULT_FG);
        let mut bg = rgb(cell.bg, DEFAULT_BG);
        if modifier.contains(Modifier::REVERSED) {
            core::mem::swap(&mut fg, &mut bg);
        }
        if modifier.contains(Modifier::DIM) {
            fg = blend(fg, bg);
        }
        if modifier.contains(Modifier::HIDDEN) {
            fg = bg;
        }

        let font = if modifier.contains(Modifier::BOLD) {
            &self.bold
        } else {
            &self.regular
        };
        let ch = cell.symbol().chars().next().unwrap_or(' ');
        let glyph = font.glyph_or_replacement(ch);
        let (px, py) = self.pixel_position(x, y);
        self.framebuffer.draw_glyph(px, py, &glyph, fg, bg);

        if modifier.contains(Modifier::UNDERLINED) {
            let underline = py + self.regular.height() - 2;
            self.framebuffer
                .fill_rect(px, underline, self.regular.width(), 1, fg);
        }
    }

    /// Fills the given rows of cells, from the column `from` (inclusive) to
    /// the column `to` (exclusive), with the default background.
    fn clear_cells(&mut self, rows: core::ops::Range<u16>, from: u16, to: u16) {
        for y in rows {
            let (px, py) = self.pixel_position(from, y);
            let width = usize::from(to.saturating_sub(from)) * self.regular.width();
            self.framebuffer
                .fill_rect(px, py, width, self.regular.height(), DEFAULT_BG);
        }
    }
}

impl Backend for FramebufferBackend {
    type Error = Infallible;

    /// Draws the cells that changed since the previous frame.
    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        for (x, y, cell) in content {
            self.draw_cell(x, y, cell);
        }
        Ok(())
    }

    /// Does nothing: the backend never draws the cursor.
    fn hide_cursor(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Does nothing: the backend never draws the cursor.
    fn show_cursor(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Returns the last position given to [`Self::set_cursor_position`].
    fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
        Ok(self.cursor)
    }

    /// Remembers the position of the cursor, used by [`Self::clear_region`].
    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> Result<(), Self::Error> {
        self.cursor = position.into();
        Ok(())
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        self.framebuffer.clear(DEFAULT_BG);
        Ok(())
    }

    /// Fills a part of the grid with the default background color, relatively
    /// to the position of the cursor.
    fn clear_region(&mut self, clear_type: ClearType) -> Result<(), Self::Error> {
        let Position { x, y } = self.cursor;
        let (columns, rows) = (self.columns, self.rows);
        match clear_type {
            ClearType::All => self.framebuffer.clear(DEFAULT_BG),
            ClearType::AfterCursor => {
                self.clear_cells(y..y + 1, x, columns);
                self.clear_cells(y + 1..rows, 0, columns);
            }
            ClearType::BeforeCursor => {
                self.clear_cells(0..y, 0, columns);
                self.clear_cells(y..y + 1, 0, x + 1);
            }
            ClearType::CurrentLine => self.clear_cells(y..y + 1, 0, columns),
            ClearType::UntilNewLine => self.clear_cells(y..y + 1, x, columns),
        }
        Ok(())
    }

    fn size(&self) -> Result<Size, Self::Error> {
        Ok(Size::new(self.columns, self.rows))
    }

    fn window_size(&mut self) -> Result<WindowSize, Self::Error> {
        let pixels = Size::new(
            u16::try_from(self.framebuffer.width()).unwrap_or(u16::MAX),
            u16::try_from(self.framebuffer.height()).unwrap_or(u16::MAX),
        );
        Ok(WindowSize {
            columns_rows: Size::new(self.columns, self.rows),
            pixels,
        })
    }

    /// Does nothing: the cells are drawn directly on the framebuffer.
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Converts a terminal color into a framebuffer color, using `default` for
/// [`Color::Reset`].
fn rgb(color: Color, default: Rgb) -> Rgb {
    let ansi = |index: usize| ANSI[index];
    match color {
        Color::Reset => default,
        Color::Black => ansi(0),
        Color::Red => ansi(1),
        Color::Green => ansi(2),
        Color::Yellow => ansi(3),
        Color::Blue => ansi(4),
        Color::Magenta => ansi(5),
        Color::Cyan => ansi(6),
        Color::Gray => ansi(7),
        Color::DarkGray => ansi(8),
        Color::LightRed => ansi(9),
        Color::LightGreen => ansi(10),
        Color::LightYellow => ansi(11),
        Color::LightBlue => ansi(12),
        Color::LightMagenta => ansi(13),
        Color::LightCyan => ansi(14),
        Color::White => ansi(15),
        Color::Rgb(red, green, blue) => Rgb::new(red, green, blue),
        Color::Indexed(index) => indexed(index),
    }
}

/// Converts a color of the xterm 256 colors palette: the 16 ANSI colors, a
/// 6x6x6 color cube, and 24 shades of gray.
fn indexed(index: u8) -> Rgb {
    // The levels of each component in the color cube.
    const LEVELS: [u8; 6] = [0x00, 0x5F, 0x87, 0xAF, 0xD7, 0xFF];
    match index {
        0..=15 => ANSI[usize::from(index)],
        16..=231 => {
            let cube = usize::from(index - 16);
            Rgb::new(LEVELS[cube / 36], LEVELS[cube / 6 % 6], LEVELS[cube % 6])
        }
        _ => {
            let level = 8 + (index - 232) * 10;
            Rgb::new(level, level, level)
        }
    }
}

/// Returns the color halfway between two colors.
fn blend(a: Rgb, b: Rgb) -> Rgb {
    let mix = u8::midpoint;
    Rgb::new(
        mix(a.red, b.red),
        mix(a.green, b.green),
        mix(a.blue, b.blue),
    )
}
