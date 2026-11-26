//! Bitmap fonts in the PC Screen Font format (PSF), version 1 and 2. This is
//! the format used by the Linux console, which makes a lot of fonts available.
use alloc::vec::Vec;

/// The font embedded into the kernel: the classic VGA font, with 8x16 pixels
/// glyphs covering Latin-1 and the box drawing characters.
static DEFAULT_FONT: &[u8] = include_bytes!("../../assets/fonts/vga16.psf");

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
}

impl Font {
    /// Parses the font embedded into the kernel.
    ///
    /// # Panics
    /// Never panics in practice: the embedded font is known to be valid.
    #[must_use]
    pub fn default_font() -> Self {
        Self::parse(DEFAULT_FONT).expect("The embedded font is invalid")
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
        if width == 0 || height == 0 || count == 0 || count > usize::from(u16::MAX) {
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

        let start = usize::from(index) * self.bytes_per_glyph;
        Some(Glyph {
            bitmap: &self.glyphs[start..start + self.bytes_per_glyph],
            width: self.width,
            height: self.height,
            bytes_per_row: self.width.div_ceil(8),
        })
    }

    /// Returns the glyph of a character, or the glyph of the replacement
    /// character (or of `?`, or the first glyph) if the font does not have a
    /// glyph for this character.
    #[must_use]
    pub fn glyph_or_replacement(&self, ch: char) -> Glyph<'_> {
        self.glyph(ch)
            .or_else(|| self.glyph(char::REPLACEMENT_CHARACTER))
            .or_else(|| self.glyph('?'))
            .unwrap_or(Glyph {
                bitmap: &self.glyphs[..self.bytes_per_glyph],
                width: self.width,
                height: self.height,
                bytes_per_row: self.width.div_ceil(8),
            })
    }
}
