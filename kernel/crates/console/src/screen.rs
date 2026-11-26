//! A text screen made of character cells, and the drawing primitives used by
//! the console to compose its interface.
use alloc::vec::Vec;

use crate::color::Color;

/// A character cell: a character with its foreground and background colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

impl Cell {
    /// Creates a new cell.
    #[must_use]
    pub const fn new(ch: char, fg: Color, bg: Color) -> Self {
        Self { ch, fg, bg }
    }

    /// Returns the same cell with its foreground and background colors
    /// swapped. This is used to draw the cursor.
    #[must_use]
    pub const fn inverted(self) -> Self {
        Self {
            ch: self.ch,
            fg: self.bg,
            bg: self.fg,
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::new(' ', Color::LightGray, Color::Black)
    }
}

/// A device able to display a grid of character cells. This is the only
/// abstraction the console needs to draw itself: it can be implemented on top
/// of a framebuffer, a VGA text mode buffer, a terminal emulator or a window.
pub trait Display {
    /// Returns the size of the display, in cells (columns, rows).
    fn size(&self) -> (u16, u16);

    /// Draws a cell at the given position. The position is guaranteed to be
    /// within the bounds returned by [`Display::size`].
    fn draw(&mut self, x: u16, y: u16, cell: Cell);

    /// Called once all the cells that changed since the previous frame have
    /// been drawn. Displays that use a back buffer can copy it to the screen
    /// here.
    fn flush(&mut self) {}
}

/// A rectangular area of a grid, in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    /// Creates a new rectangle.
    #[must_use]
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// A two-dimensional array of cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
}

impl Grid {
    /// Creates a new grid of the given size, filled with the given cell.
    #[must_use]
    pub fn new(width: u16, height: u16, fill: Cell) -> Self {
        Self {
            width,
            height,
            cells: alloc::vec![fill; usize::from(width) * usize::from(height)],
        }
    }

    /// Returns the width of the grid, in cells.
    #[must_use]
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Returns the height of the grid, in cells.
    #[must_use]
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Returns the cell at the given position, or `None` if the position is
    /// out of the bounds of the grid.
    #[must_use]
    pub fn get(&self, x: u16, y: u16) -> Option<Cell> {
        self.index(x, y).map(|i| self.cells[i])
    }

    /// Changes the cell at the given position. Positions out of the bounds of
    /// the grid are silently ignored.
    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        if let Some(i) = self.index(x, y) {
            self.cells[i] = cell;
        }
    }

    /// Returns a canvas covering the whole grid.
    #[must_use]
    pub fn canvas(&mut self) -> Canvas<'_> {
        let area = Rect::new(0, 0, self.width, self.height);
        Canvas {
            grid: self,
            area,
            fg: Color::LightGray,
            bg: Color::Black,
        }
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        (x < self.width && y < self.height)
            .then(|| usize::from(y) * usize::from(self.width) + usize::from(x))
    }
}

/// The characters used to draw a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorderStyle {
    Single,
    Double,
}

impl BorderStyle {
    /// Returns the characters of the border: horizontal, vertical, top left,
    /// top right, bottom left and bottom right.
    const fn chars(self) -> [char; 6] {
        match self {
            BorderStyle::Single => ['─', '│', '┌', '┐', '└', '┘'],
            BorderStyle::Double => ['═', '║', '╔', '╗', '╚', '╝'],
        }
    }
}

/// A view on a rectangular area of a grid, with the colors used to draw in it.
/// All coordinates are relative to the area, and everything drawn outside of
/// the area is clipped.
pub struct Canvas<'a> {
    grid: &'a mut Grid,
    area: Rect,
    fg: Color,
    bg: Color,
}

impl Canvas<'_> {
    /// Returns the width of the canvas, in cells.
    #[must_use]
    pub fn width(&self) -> u16 {
        self.area.width
    }

    /// Returns the height of the canvas, in cells.
    #[must_use]
    pub fn height(&self) -> u16 {
        self.area.height
    }

    /// Returns a canvas covering the given area of this canvas, clipped to
    /// the bounds of this canvas. The new canvas inherits the colors of this
    /// canvas.
    #[must_use]
    pub fn sub(&mut self, rect: Rect) -> Canvas<'_> {
        let x = rect.x.min(self.area.width);
        let y = rect.y.min(self.area.height);
        let area = Rect {
            x: self.area.x + x,
            y: self.area.y + y,
            width: rect.width.min(self.area.width - x),
            height: rect.height.min(self.area.height - y),
        };
        Canvas {
            grid: self.grid,
            area,
            fg: self.fg,
            bg: self.bg,
        }
    }

    /// Changes the colors used by the next drawing operations.
    pub fn set_colors(&mut self, fg: Color, bg: Color) {
        self.fg = fg;
        self.bg = bg;
    }

    /// Changes the foreground color used by the next drawing operations.
    pub fn set_fg(&mut self, fg: Color) {
        self.fg = fg;
    }

    /// Draws a cell at the given position.
    pub fn put_cell(&mut self, x: u16, y: u16, cell: Cell) {
        if x < self.area.width && y < self.area.height {
            self.grid.set(self.area.x + x, self.area.y + y, cell);
        }
    }

    /// Draws a character at the given position with the current colors.
    pub fn put(&mut self, x: u16, y: u16, ch: char) {
        self.put_cell(x, y, Cell::new(ch, self.fg, self.bg));
    }

    /// Draws a string starting at the given position with the current colors,
    /// and returns the column just after the last character drawn. The string
    /// is not wrapped: characters beyond the right edge are clipped.
    pub fn text(&mut self, x: u16, y: u16, text: &str) -> u16 {
        let mut column = x;
        for ch in text.chars() {
            if column >= self.area.width {
                break;
            }
            self.put(column, y, ch);
            column += 1;
        }
        column
    }

    /// Draws a string so that it ends on the right edge of the canvas.
    pub fn text_right(&mut self, y: u16, text: &str) {
        let len = u16::try_from(text.chars().count()).unwrap_or(u16::MAX);
        self.text(self.area.width.saturating_sub(len), y, text);
    }

    /// Fills the whole canvas with the given character and the current colors.
    pub fn fill(&mut self, ch: char) {
        for y in 0..self.area.height {
            self.hline(0, y, self.area.width, ch);
        }
    }

    /// Draws a horizontal line of `len` characters.
    pub fn hline(&mut self, x: u16, y: u16, len: u16, ch: char) {
        for i in 0..len {
            self.put(x.saturating_add(i), y, ch);
        }
    }

    /// Draws a frame along the edges of the canvas, with an optional title
    /// centered on the top edge.
    pub fn frame(&mut self, style: BorderStyle, title: Option<&str>) {
        let (w, h) = (self.area.width, self.area.height);
        if w < 2 || h < 2 {
            return;
        }

        let [horizontal, vertical, tl, tr, bl, br] = style.chars();
        self.hline(1, 0, w - 2, horizontal);
        self.hline(1, h - 1, w - 2, horizontal);
        for y in 1..h - 1 {
            self.put(0, y, vertical);
            self.put(w - 1, y, vertical);
        }
        self.put(0, 0, tl);
        self.put(w - 1, 0, tr);
        self.put(0, h - 1, bl);
        self.put(w - 1, h - 1, br);

        if let Some(title) = title {
            let len = u16::try_from(title.chars().count()).unwrap_or(u16::MAX);
            if len + 4 <= w {
                self.text((w - len) / 2, 0, title);
            }
        }
    }

    /// Draws a horizontal gauge of `width` cells, filled proportionally to
    /// `permille` (from 0 to 1000) with the given color.
    pub fn gauge(&mut self, x: u16, y: u16, width: u16, permille: u32, color: Color) {
        let permille = permille.min(1000);
        let filled = u16::try_from(u32::from(width) * permille / 1000).unwrap_or(width);
        let fg = self.fg;
        self.fg = color;
        self.hline(x, y, filled, '█');
        self.fg = fg;
        self.hline(x + filled, y, width - filled, '░');
    }
}

/// A screen that only sends to the display the cells that changed since the
/// previous frame. The console composes each frame in a back buffer, and the
/// screen compares it with what is currently displayed. This keeps the cost
/// of a frame proportional to what actually changed, which is important since
/// drawing a cell on a framebuffer means writing hundreds of pixels.
pub struct Screen {
    back: Grid,
    front: Option<Grid>,
}

impl Screen {
    /// Creates a new screen of the given size, in cells.
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            back: Grid::new(width, height, Cell::default()),
            front: None,
        }
    }

    /// Returns a canvas to compose the next frame.
    #[must_use]
    pub fn canvas(&mut self) -> Canvas<'_> {
        self.back.canvas()
    }

    /// Forces the next call to [`Screen::present`] to redraw every cell.
    pub fn invalidate(&mut self) {
        self.front = None;
    }

    /// Draws on the display the cells that changed since the previous frame.
    pub fn present<D: Display + ?Sized>(&mut self, display: &mut D) {
        let width = usize::from(self.back.width).max(1);
        for (i, &cell) in self.back.cells.iter().enumerate() {
            if self
                .front
                .as_ref()
                .is_some_and(|front| front.cells[i] == cell)
            {
                continue;
            }
            // The index is lower than `width * height`, so both coordinates
            // fit in an `u16`.
            #[allow(clippy::cast_possible_truncation)]
            display.draw((i % width) as u16, (i / width) as u16, cell);
        }

        match &mut self.front {
            Some(front) => front.clone_from(&self.back),
            None => self.front = Some(self.back.clone()),
        }
        display.flush();
    }
}
