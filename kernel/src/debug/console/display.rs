use console::{
    Color,
    screen::{Cell, Display},
};

use crate::video::{Font, Framebuffer, Rgb};

/// A console display drawing the cells on the framebuffer. The grid of cells
/// is centered on the framebuffer if its size is not a multiple of the size of
/// the glyphs.
pub struct FramebufferDisplay {
    framebuffer: Framebuffer,
    font: Font,
    columns: u16,
    rows: u16,
    origin: (usize, usize),
}

impl FramebufferDisplay {
    /// Creates a display covering the whole framebuffer, and clears it.
    pub fn new(mut framebuffer: Framebuffer, font: Font) -> Self {
        let columns = framebuffer.width() / font.width();
        let rows = framebuffer.height() / font.height();
        let origin = (
            (framebuffer.width() - columns * font.width()) / 2,
            (framebuffer.height() - rows * font.height()) / 2,
        );

        framebuffer.clear(Rgb::BLACK);
        Self {
            columns: u16::try_from(columns).unwrap_or(u16::MAX),
            rows: u16::try_from(rows).unwrap_or(u16::MAX),
            framebuffer,
            font,
            origin,
        }
    }
}

impl Display for FramebufferDisplay {
    fn size(&self) -> (u16, u16) {
        (self.columns, self.rows)
    }

    fn draw(&mut self, x: u16, y: u16, cell: Cell) {
        let glyph = self.font.glyph_or_replacement(cell.ch);
        self.framebuffer.draw_glyph(
            self.origin.0 + usize::from(x) * self.font.width(),
            self.origin.1 + usize::from(y) * self.font.height(),
            &glyph,
            rgb(cell.fg),
            rgb(cell.bg),
        );
    }
}

/// Converts a color of the console palette into a framebuffer color.
fn rgb(color: Color) -> Rgb {
    let (red, green, blue) = color.rgb();
    Rgb::new(red, green, blue)
}
