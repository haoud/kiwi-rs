//! Bitmap fonts in the PC Screen Font format (PSF), version 1 and 2. This is
//! the format used by the Linux console, which makes a lot of fonts available.
use alloc::vec::Vec;

/// The regular font embedded into the kernel: Terminus, with 8x16 pixels
/// glyphs covering Latin-1, box drawing characters and some symbols.
static REGULAR_FONT: &[u8] = include_bytes!("../../assets/fonts/terminus16.psf");

/// The bold variant of the embedded font.
static BOLD_FONT: &[u8] = include_bytes!("../../assets/fonts/terminus16-bold.psf");

const PSF1_MAGIC: [u8; 2] = [0x36, 0x04];
const PSF1_MODE_512: u8 = 0x01;
const PSF1_MODE_HAS_TABLE: u8 = 0x02;
const PSF1_MODE_HAS_SEQUENCES: u8 = 0x04;
const PSF1_SEPARATOR: u16 = 0xFFFF;
const PSF1_START_SEQUENCE: u16 = 0xFFFE;

const PSF2_MAGIC: [u8; 4] = [0x72, 0xB5, 0x4A, 0x86];
const PSF2_HAS_UNICODE_TABLE: u32 = 0x01;
const PSF2_SEPARATOR: u8 = 0xFF;
const PSF2_START_SEQUENCE: u8 = 0xFE;

/// The reasons why a font cannot be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    /// The data does not start with the magic number of a PSF font.
    BadMagic,

    /// The data is shorter than what the header announces.
    Truncated,

    /// The header contains invalid values (e.g. a glyph of zero pixels).
    InvalidHeader,
}

/// A glyph: a bitmap of `width` x `height` pixels, stored line by line with
/// each line padded to a whole number of bytes, most significant bit first.
#[derive(Debug, Clone, Copy)]
pub struct Glyph<'a> {
    bitmap: &'a [u8],
    width: usize,
    height: usize,
    bytes_per_row: usize,
}

impl Glyph<'_> {
    /// Returns the width of the glyph, in pixels.
    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the glyph, in pixels.
    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the bytes of a row of the glyph, most significant bit first.
    ///
    /// # Panics
    /// Panics if the row is out of the bounds of the glyph.
    #[must_use]
    pub fn row(&self, y: usize) -> &[u8] {
        &self.bitmap[y * self.bytes_per_row..(y + 1) * self.bytes_per_row]
    }

    /// Returns true if the pixel at the given position is set.
    #[must_use]
    pub fn is_set(&self, x: usize, y: usize) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let byte = self.bitmap[y * self.bytes_per_row + x / 8];
        byte & (0x80 >> (x % 8)) != 0
    }
}

/// A bitmap font where all the glyphs have the same size.
#[derive(Debug, Clone)]
pub struct Font {
    glyphs: &'static [u8],
    count: usize,
    width: usize,
    height: usize,
    bytes_per_glyph: usize,

    /// The index of the glyph of each character from U+0000 to U+00FF, or
    /// `u16::MAX` if the font has no glyph for it. This avoids a search for
    /// the most common characters.
    latin1: [u16; 256],

    /// The index of the glyph of the other characters, sorted by character.
    others: Vec<(char, u16)>,

    /// The glyphs synthesized by [`Font::synthesize_missing`], whose indexes
    /// follow the indexes of the glyphs of the font.
    synthetic: Vec<u8>,
}

impl Font {
    /// Parses the regular font embedded into the kernel, completed with the
    /// synthesized block elements and Braille patterns.
    ///
    /// # Panics
    /// Never panics in practice: the embedded font is known to be valid.
    #[must_use]
    pub fn regular() -> Self {
        let mut font = Self::parse(REGULAR_FONT).expect("The embedded font is invalid");
        font.synthesize_missing();
        font
    }

    /// Parses the bold font embedded into the kernel, completed with the
    /// synthesized block elements and Braille patterns.
    ///
    /// # Panics
    /// Never panics in practice: the embedded font is known to be valid.
    #[must_use]
    pub fn bold() -> Self {
        let mut font = Self::parse(BOLD_FONT).expect("The embedded font is invalid");
        font.synthesize_missing();
        font
    }

    /// Parses a PSF1 or PSF2 font.
    ///
    /// # Errors
    /// Returns an error if the data is not a valid PSF font.
    pub fn parse(data: &'static [u8]) -> Result<Self, Error> {
        if data.starts_with(&PSF2_MAGIC) {
            Self::parse_psf2(data)
        } else if data.starts_with(&PSF1_MAGIC) {
            Self::parse_psf1(data)
        } else {
            Err(Error::BadMagic)
        }
    }

    fn parse_psf1(data: &'static [u8]) -> Result<Self, Error> {
        let mode = *data.get(2).ok_or(Error::Truncated)?;
        let height = usize::from(*data.get(3).ok_or(Error::Truncated)?);
        let count = if mode & PSF1_MODE_512 != 0 { 512 } else { 256 };

        let mut font = Self::new(data, 4, count, 8, height)?;
        if mode & (PSF1_MODE_HAS_TABLE | PSF1_MODE_HAS_SEQUENCES) != 0 {
            let table = &data[4 + count * font.bytes_per_glyph..];
            let mut entries = table
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]));

            for glyph in 0..count {
                let mut in_sequence = false;
                for entry in entries.by_ref() {
                    match entry {
                        PSF1_SEPARATOR => break,
                        PSF1_START_SEQUENCE => in_sequence = true,
                        _ if in_sequence => {}
                        code => {
                            if let Some(ch) = char::from_u32(u32::from(code)) {
                                font.map(ch, glyph);
                            }
                        }
                    }
                }
            }
            font.others.sort_unstable_by_key(|&(ch, _)| ch);
        } else {
            font.map_identity();
        }
        Ok(font)
    }

    fn parse_psf2(data: &'static [u8]) -> Result<Self, Error> {
        let field = |index: usize| -> Result<u32, Error> {
            let bytes = data.get(index * 4..index * 4 + 4).ok_or(Error::Truncated)?;
            Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        };
        let to_usize = |value: u32| usize::try_from(value).map_err(|_| Error::InvalidHeader);
        let header_size = to_usize(field(2)?)?;
        let flags = field(3)?;
        let count = to_usize(field(4)?)?;
        let bytes_per_glyph = to_usize(field(5)?)?;
        let height = to_usize(field(6)?)?;
        let width = to_usize(field(7)?)?;

        let mut font = Self::new(data, header_size, count, width, height)?;
        if font.bytes_per_glyph != bytes_per_glyph {
            return Err(Error::InvalidHeader);
        }

        if flags & PSF2_HAS_UNICODE_TABLE != 0 {
            let mut table = &data[header_size + count * bytes_per_glyph..];
            for glyph in 0..count {
                // Each glyph has a list of UTF-8 characters, followed by an
                // optional list of sequences, and ends with a separator.
                let end = table
                    .iter()
                    .position(|&b| b == PSF2_SEPARATOR)
                    .unwrap_or(table.len());
                let entry = &table[..end];
                let singles = entry
                    .iter()
                    .position(|&b| b == PSF2_START_SEQUENCE)
                    .map_or(entry, |start| &entry[..start]);

                if let Ok(chars) = core::str::from_utf8(singles) {
                    chars.chars().for_each(|ch| font.map(ch, glyph));
                }
                table = table.get(end + 1..).unwrap_or_default();
            }
            font.others.sort_unstable_by_key(|&(ch, _)| ch);
        } else {
            font.map_identity();
        }
        Ok(font)
    }

    /// Creates a font without any character mapped, after checking that the
    /// data contains all the glyphs.
    fn new(
        data: &'static [u8],
        offset: usize,
        count: usize,
        width: usize,
        height: usize,
    ) -> Result<Self, Error> {
        // Keep some room in the glyph indexes for the synthesized glyphs.
        if width == 0 || height == 0 || count == 0 || count > usize::from(u16::MAX / 2) {
            return Err(Error::InvalidHeader);
        }
        let bytes_per_glyph = width.div_ceil(8) * height;
        let end = offset
            .checked_add(count * bytes_per_glyph)
            .ok_or(Error::InvalidHeader)?;
        let glyphs = data.get(offset..end).ok_or(Error::Truncated)?;

        Ok(Self {
            glyphs,
            count,
            width,
            height,
            bytes_per_glyph,
            latin1: [u16::MAX; 256],
            others: Vec::new(),
            synthetic: Vec::new(),
        })
    }

    /// Maps a character to a glyph, unless the character is already mapped.
    fn map(&mut self, ch: char, glyph: usize) {
        // The number of glyphs was checked to fit in an `u16`.
        #[allow(clippy::cast_possible_truncation)]
        let glyph = glyph as u16;
        match usize::try_from(u32::from(ch)) {
            Ok(code) if code < self.latin1.len() => {
                if self.latin1[code] == u16::MAX {
                    self.latin1[code] = glyph;
                }
            }
            _ => self.others.push((ch, glyph)),
        }
    }

    /// Maps the characters from U+0000 to U+00FF to the glyph with the same
    /// index, for fonts without a Unicode table.
    fn map_identity(&mut self) {
        for code in 0..self.latin1.len().min(self.count) {
            // The index is lower than 256.
            #[allow(clippy::cast_possible_truncation)]
            {
                self.latin1[code] = code as u16;
            }
        }
    }

    /// Returns the width of the glyphs, in pixels.
    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the glyphs, in pixels.
    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the glyph of a character, or `None` if the font does not have
    /// a glyph for this character.
    #[must_use]
    pub fn glyph(&self, ch: char) -> Option<Glyph<'_>> {
        let index = match usize::try_from(u32::from(ch)) {
            Ok(code) if code < self.latin1.len() => {
                Some(self.latin1[code]).filter(|&index| index != u16::MAX)
            }
            _ => self
                .others
                .binary_search_by_key(&ch, |&(c, _)| c)
                .ok()
                .map(|i| self.others[i].1),
        }?;

        Some(self.glyph_at(usize::from(index)))
    }

    /// Returns the glyph at the given index, which may be a synthesized glyph.
    fn glyph_at(&self, index: usize) -> Glyph<'_> {
        let bitmap = if index < self.count {
            let start = index * self.bytes_per_glyph;
            &self.glyphs[start..start + self.bytes_per_glyph]
        } else {
            let start = (index - self.count) * self.bytes_per_glyph;
            &self.synthetic[start..start + self.bytes_per_glyph]
        };
        Glyph {
            bitmap,
            width: self.width,
            height: self.height,
            bytes_per_row: self.width.div_ceil(8),
        }
    }

    /// Adds the block elements (U+2580 to U+259F) and the Braille patterns
    /// (U+2800 to U+28FF) that are missing from the font, by drawing them
    /// geometrically. Text user interfaces use them a lot (gauges, sparklines,
    /// charts), and they are missing from most console fonts.
    ///
    /// The heavy horizontal and vertical lines (`━` and `┃`) are also drawn if
    /// the font approximates them with the light lines, as most console fonts
    /// with a limited number of glyphs do.
    pub fn synthesize_missing(&mut self) {
        let (width, height) = (self.width, self.height);
        let heavy = [('\u{2501}', '\u{2500}'), ('\u{2503}', '\u{2502}')];
        let characters = ('\u{2580}'..='\u{259F}')
            .chain('\u{2800}'..='\u{28FF}')
            .chain(heavy.iter().map(|&(heavy, _)| heavy));
        for ch in characters {
            if let Some(glyph) = self.glyph(ch) {
                let approximated = heavy.iter().any(|&(heavy, light)| {
                    heavy == ch && self.glyph(light).is_some_and(|l| l.bitmap == glyph.bitmap)
                });
                if !approximated {
                    continue;
                }
                self.others.retain(|&(c, _)| c != ch);
            }
            let Some(pixel) = synthetic_pixel(ch, width, height) else {
                continue;
            };

            let index = self.count + self.synthetic.len() / self.bytes_per_glyph;
            let bytes_per_row = width.div_ceil(8);
            for y in 0..height {
                for byte in 0..bytes_per_row {
                    let bits = (0..8)
                        .filter(|bit| {
                            let x = byte * 8 + bit;
                            x < width && pixel(x, y)
                        })
                        .fold(0u8, |acc, bit| acc | (0x80 >> bit));
                    self.synthetic.push(bits);
                }
            }
            self.others
                .push((ch, u16::try_from(index).unwrap_or(u16::MAX)));
        }
        self.others.sort_unstable_by_key(|&(ch, _)| ch);
    }

    /// Returns the glyph of a character, or the glyph of the replacement
    /// character (or of `?`, or the first glyph) if the font does not have a
    /// glyph for this character.
    #[must_use]
    pub fn glyph_or_replacement(&self, ch: char) -> Glyph<'_> {
        self.glyph(ch)
            .or_else(|| self.glyph(char::REPLACEMENT_CHARACTER))
            .or_else(|| self.glyph('?'))
            .unwrap_or_else(|| self.glyph_at(0))
    }
}

/// Returns a function telling whether a pixel of a synthesized glyph is set,
/// for the block elements and the Braille patterns, or `None` for the other
/// characters.
fn synthetic_pixel(ch: char, width: usize, height: usize) -> Option<impl Fn(usize, usize) -> bool> {
    let code = u32::from(ch);
    let (w, h) = (width, height);

    // The pixel is set if it is inside the given rectangle, expressed in
    // eighths of the cell.
    let rect = move |x: usize, y: usize, (x0, y0, x1, y1): (usize, usize, usize, usize)| {
        x * 8 >= x0 * w && x * 8 < x1 * w && y * 8 >= y0 * h && y * 8 < y1 * h
    };
    // Quadrants of the cell: upper left, upper right, lower left, lower right.
    let quadrants = |mask: u8| {
        move |x: usize, y: usize| {
            let right = x * 2 >= w;
            let lower = y * 2 >= h;
            let bit = match (lower, right) {
                (false, false) => 1,
                (false, true) => 2,
                (true, false) => 4,
                (true, true) => 8,
            };
            mask & bit != 0
        }
    };

    let pixel: alloc::boxed::Box<dyn Fn(usize, usize) -> bool> = match code {
        // Heavy horizontal and vertical lines, 3 pixels thick for a 16 pixels
        // high glyph, centered on the light lines.
        0x2501 => {
            alloc::boxed::Box::new(move |_, y| y + 1 >= h / 2 - h / 16 && y <= h / 2 - 1 + h / 16)
        }
        0x2503 => {
            alloc::boxed::Box::new(move |x, _| x + 1 >= w / 2 - w / 8 && x <= w / 2 - 1 + w / 8)
        }
        // Upper half, and lower one to eight eighths.
        0x2580 => alloc::boxed::Box::new(move |x, y| rect(x, y, (0, 0, 8, 4))),
        0x2581..=0x2588 => {
            let eighths = (code - 0x2580) as usize;
            alloc::boxed::Box::new(move |x, y| rect(x, y, (0, 8 - eighths, 8, 8)))
        }
        // Left seven to one eighths.
        0x2589..=0x258F => {
            let eighths = (0x2590 - code) as usize;
            alloc::boxed::Box::new(move |x, y| rect(x, y, (0, 0, eighths, 8)))
        }
        0x2590 => alloc::boxed::Box::new(move |x, y| rect(x, y, (4, 0, 8, 8))),
        // Light, medium and dark shades.
        0x2591 => alloc::boxed::Box::new(|x, y| x % 2 == 0 && y % 2 == 0),
        0x2592 => alloc::boxed::Box::new(|x, y| (x + y) % 2 == 0),
        0x2593 => alloc::boxed::Box::new(|x, y| !(x % 2 == 0 && y % 2 == 0)),
        0x2594 => alloc::boxed::Box::new(move |x, y| rect(x, y, (0, 0, 8, 1))),
        0x2595 => alloc::boxed::Box::new(move |x, y| rect(x, y, (7, 0, 8, 8))),
        0x2596..=0x259F => {
            const MASKS: [u8; 10] = [
                4,
                8,
                1,
                1 | 4 | 8,
                1 | 8,
                1 | 2 | 4,
                1 | 2 | 8,
                2,
                2 | 4,
                2 | 4 | 8,
            ];
            alloc::boxed::Box::new(quadrants(MASKS[(code - 0x2596) as usize]))
        }
        // Braille patterns: 2 columns of 4 dots, numbered from top to bottom
        // in the left column (1, 2, 3, 7) then in the right one (4, 5, 6, 8).
        0x2800..=0x28FF => {
            let dots = u8::try_from(code - 0x2800).unwrap_or(0);
            alloc::boxed::Box::new(move |x, y| {
                let column = x * 2 / w;
                let row = y * 4 / h;
                let bit = match (column, row) {
                    (0, 3) => 6,
                    (1, 3) => 7,
                    (0, row) => row,
                    (_, row) => row + 3,
                };
                // Draw each dot as a square in the middle of its sub-cell.
                let (sub_w, sub_h) = (w / 2, h / 4);
                let (dx, dy) = (x % sub_w.max(1), y % sub_h.max(1));
                let inside = dx >= sub_w / 4
                    && dx < sub_w - sub_w / 4
                    && dy >= sub_h / 4
                    && dy < sub_h - sub_h / 4;
                inside && dots & (1 << bit) != 0
            })
        }
        _ => return None,
    };
    Some(pixel)
}
