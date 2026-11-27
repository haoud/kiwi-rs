use core::sync::atomic::{AtomicBool, Ordering};

use crate::{
    arch::{self, framebuffer::PixelFormat},
    video::font::{Font, Glyph},
};

/// A color, with 8 bits per component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb::new(0, 0, 0);
    pub const WHITE: Rgb = Rgb::new(0xFF, 0xFF, 0xFF);

    /// Creates a new color from its components.
    #[must_use]
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

/// The reasons why the framebuffer cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    /// The bootloader did not provide any usable framebuffer.
    NotFound,

    /// The framebuffer has already been taken by someone else.
    AlreadyTaken,

    /// The pixel format of the framebuffer is not supported: only 32 bits per
    /// pixel framebuffers with components of at most 8 bits are supported.
    UnsupportedFormat(PixelFormat),
}

/// A linear framebuffer with 32 bits per pixel, and the primitives to draw on
/// it. There is only one `Framebuffer` in the kernel, obtained with
/// [`Framebuffer::take`], so the drawing functions do not need any
/// synchronization.
///
/// All the drawing functions clip what they draw to the bounds of the
/// framebuffer, so they can be given any coordinates.
pub struct Framebuffer {
    /// The pixels of the framebuffer, line by line. Each line contains
    /// `stride` pixels, of which only the first `width` are displayed.
    pixels: &'static mut [u32],

    /// The size of the framebuffer, in pixels.
    width: usize,
    height: usize,

    /// The number of pixels between the start of two consecutive lines. It
    /// may be greater than the width if the lines are padded.
    stride: usize,

    /// The layout of a pixel, used to convert colors into pixel values.
    format: PixelFormat,
}

impl Framebuffer {
    /// Takes the ownership of the framebuffer set up by the bootloader.
    ///
    /// # Errors
    /// Returns an error if there is no usable framebuffer, if its pixel format
    /// is not supported, or if it has already been taken.
    pub fn take() -> Result<Self, Error> {
        /// Whether the framebuffer has already been taken. Since the
        /// framebuffer is shared as a mutable slice, it must only be taken
        /// once to avoid aliasing.
        static TAKEN: AtomicBool = AtomicBool::new(false);

        let info = arch::framebuffer::framebuffer().ok_or(Error::NotFound)?;
        let format = info.format;
        if format.bits_per_pixel != 32
            || info.pitch % 4 != 0
            || [format.red_size, format.green_size, format.blue_size]
                .iter()
                .any(|&size| size > 8)
        {
            return Err(Error::UnsupportedFormat(format));
        }

        if TAKEN.swap(true, Ordering::AcqRel) {
            return Err(Error::AlreadyTaken);
        }

        let stride = info.pitch / 4;
        // SAFETY: The bootloader guarantees that the framebuffer is mapped in
        // the kernel address space, is properly aligned and covers `pitch`
        // bytes for each of its `height` lines. The framebuffer memory is not
        // used by anything else in the kernel, and the `TAKEN` flag ensures
        // that this mutable slice is created only once.
        let pixels = unsafe {
            core::slice::from_raw_parts_mut(info.address.as_mut_ptr::<u32>(), stride * info.height)
        };

        Ok(Self {
            pixels,
            width: info.width,
            height: info.height,
            stride,
            format,
        })
    }

    /// Returns the width of the framebuffer, in pixels.
    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the framebuffer, in pixels.
    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }

    /// Converts a color into the value of a pixel of this framebuffer.
    #[must_use]
    pub fn encode(&self, color: Rgb) -> u32 {
        // Each component is truncated to its size in the pixel (keeping its
        // most significant bits), then shifted to its position.
        let component = |value: u8, shift: u8, size: u8| (u32::from(value) >> (8 - size)) << shift;
        component(color.red, self.format.red_shift, self.format.red_size)
            | component(color.green, self.format.green_shift, self.format.green_size)
            | component(color.blue, self.format.blue_shift, self.format.blue_size)
    }

    /// Fills the whole framebuffer with the given color.
    pub fn clear(&mut self, color: Rgb) {
        self.fill_rect(0, 0, self.width, self.height, color);
    }

    /// Changes the color of a single pixel.
    pub fn put_pixel(&mut self, x: usize, y: usize, color: Rgb) {
        if x < self.width && y < self.height {
            self.pixels[y * self.stride + x] = self.encode(color);
        }
    }

    /// Fills a rectangle with the given color.
    pub fn fill_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: Rgb) {
        let pixel = self.encode(color);
        let (x_end, y_end) = self.clip(x, y, width, height);
        for line in y..y_end {
            self.line_mut(line, x, x_end).fill(pixel);
        }
    }

    /// Draws the outline of a rectangle, with lines of 1 pixel.
    pub fn draw_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: Rgb) {
        if width == 0 || height == 0 {
            return;
        }
        self.fill_rect(x, y, width, 1, color);
        self.fill_rect(x, y + height - 1, width, 1, color);
        self.fill_rect(x, y, 1, height, color);
        self.fill_rect(x + width - 1, y, 1, height, color);
    }

    /// Draws a glyph with its top left corner at the given position. The bits
    /// set in the glyph are drawn with the `fg` color, and the others with
    /// the `bg` color.
    pub fn draw_glyph(&mut self, x: usize, y: usize, glyph: &Glyph<'_>, fg: Rgb, bg: Rgb) {
        let (fg, bg) = (self.encode(fg), self.encode(bg));
        let (x_end, y_end) = self.clip(x, y, glyph.width(), glyph.height());
        for row in 0..y_end.saturating_sub(y) {
            let bits = glyph.row(row);
            let line = self.line_mut(y + row, x, x_end);
            for (column, pixel) in line.iter_mut().enumerate() {
                let set = bits[column / 8] & (0x80 >> (column % 8)) != 0;
                *pixel = if set { fg } else { bg };
            }
        }
    }

    /// Draws a character with the given font, and returns the width of the
    /// character in pixels. Characters missing from the font are replaced by
    /// the replacement glyph of the font.
    pub fn draw_char(
        &mut self,
        x: usize,
        y: usize,
        ch: char,
        font: &Font,
        fg: Rgb,
        bg: Rgb,
    ) -> usize {
        let glyph = font.glyph_or_replacement(ch);
        self.draw_glyph(x, y, &glyph, fg, bg);
        glyph.width()
    }

    /// Draws a string on a single line with the given font, and returns the
    /// position just after the last character drawn.
    pub fn draw_text(
        &mut self,
        x: usize,
        y: usize,
        text: &str,
        font: &Font,
        fg: Rgb,
        bg: Rgb,
    ) -> usize {
        text.chars()
            .fold(x, |x, ch| x + self.draw_char(x, y, ch, font, fg, bg))
    }

    /// Returns the end coordinates (exclusive) of a rectangle clipped to the
    /// bounds of the framebuffer.
    fn clip(&self, x: usize, y: usize, width: usize, height: usize) -> (usize, usize) {
        (
            x.saturating_add(width).min(self.width),
            y.saturating_add(height).min(self.height),
        )
    }

    /// Returns the pixels of a line between `start` (inclusive) and `end`
    /// (exclusive), or an empty slice if `start >= end`.
    fn line_mut(&mut self, line: usize, start: usize, end: usize) -> &mut [u32] {
        let base = line * self.stride;
        &mut self.pixels[base + start.min(end)..base + end]
    }
}

impl core::fmt::Debug for Framebuffer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Framebuffer")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("stride", &self.stride)
            .field("format", &self.format)
            .finish_non_exhaustive()
    }
}
