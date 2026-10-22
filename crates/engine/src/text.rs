//! Text: the 8 x 8 font (`fb_txt.fnt`) and the cutscene texts (`fr_cine.txt` with its index
//! `fr_cine.bin`).
//!
//! The font holds glyphs for the characters from space (0x20) on, 32 bytes each: 8 rows of 8
//! pixels, 4 bits per pixel, high nibble first. Pixel values are 0 (transparent), 15 (the face of
//! the letter), 8 and 2 (shading).
//!
//! The text file holds one text per line; `|` breaks a text into lines. The index file is a list
//! of big-endian `u16` offsets of the texts.

use crate::screen::Bitmap;
use crate::DataDir;

const GLYPH: usize = 32;

/// The game's 8 x 8 font.
#[derive(Debug, Clone)]
pub struct Font {
    data: Vec<u8>,
}

impl Font {
    pub fn parse(data: Vec<u8>) -> Font {
        Font { data }
    }

    /// Pixel (`x`, `y`) of the glyph for `c`; 0 for characters the font lacks.
    pub fn pixel(&self, c: u8, x: usize, y: usize) -> u8 {
        let Some(i) = (c as usize).checked_sub(0x20) else {
            return 0;
        };
        let b = self.data.get(i * GLYPH + y * 4 + x / 2).copied().unwrap_or(0);
        if x.is_multiple_of(2) {
            b >> 4
        } else {
            b & 15
        }
    }

    /// Draws `line` with its top-left corner at (`x`, `y`), 8 pixels per character; `colour`
    /// maps each font pixel value (1 to 15) to a palette entry, or `None` to leave it out.
    pub fn draw(&self, dst: &mut Bitmap, line: &str, x: i32, y: i32, colour: &dyn Fn(u8) -> Option<u8>) {
        for (k, c) in line.bytes().enumerate() {
            for gy in 0..8 {
                for gx in 0..8 {
                    let v = self.pixel(c, gx, gy);
                    let (px, py) = (x + (k * 8 + gx) as i32, y + gy as i32);
                    if v == 0 || px < 0 || py < 0 || px >= dst.w as i32 || py >= dst.h as i32 {
                        continue;
                    }
                    if let Some(c) = colour(v) {
                        dst.pixels[py as usize * dst.w + px as usize] = c;
                    }
                }
            }
        }
    }
}

/// Numbered texts with their line breaks.
#[derive(Debug, Clone, Default)]
pub struct Texts {
    texts: Vec<Vec<String>>,
}

impl Texts {
    /// Reads a text file with its offset index.
    pub fn parse(index: &[u8], text: &[u8]) -> Texts {
        let texts = index
            .as_chunks::<2>()
            .0
            .iter()
            .map(|o| {
                let start = (u16::from_be_bytes(*o) as usize).min(text.len());
                let rest = &text[start..];
                let end = rest
                    .iter()
                    .position(|&b| b == b'\n' || b == b'\r')
                    .unwrap_or(rest.len());
                String::from_utf8_lossy(&rest[..end])
                    .split('|')
                    .map(|l| l.trim_end().to_string())
                    .collect()
            })
            .collect();
        Texts { texts }
    }

    pub fn len(&self) -> usize {
        self.texts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }

    /// The lines of text `n`.
    pub fn get(&self, n: usize) -> Option<&[String]> {
        self.texts.get(n).map(Vec::as_slice)
    }
}

/// The font and the cutscene texts of `data`, if it has them.
pub fn load_cutscene_text(data: &DataDir) -> Option<(Font, Texts)> {
    let font = Font::parse(data.read("fb_txt.fnt").ok()?);
    let texts = Texts::parse(&data.read("fr_cine.bin").ok()?, &data.read("fr_cine.txt").ok()?);
    Some((font, texts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_split_on_bars_and_end_at_newline() {
        let text = b"HELLO|WORLD  \nSECOND\n";
        let index = [0, 0, 0, 14];
        let t = Texts::parse(&index, text);
        assert_eq!(t.len(), 2);
        assert_eq!(t.get(0).unwrap(), &["HELLO".to_string(), "WORLD".to_string()]);
        assert_eq!(t.get(1).unwrap(), &["SECOND".to_string()]);
    }

    #[test]
    fn glyph_pixels_are_nibbles_from_space_on() {
        let mut data = vec![0u8; GLYPH * 2];
        data[GLYPH] = 0xf2; // '!' row 0: pixels 15, 2
        let f = Font::parse(data);
        assert_eq!((f.pixel(b'!', 0, 0), f.pixel(b'!', 1, 0)), (15, 2));
        assert_eq!(f.pixel(b' ', 0, 0), 0);
        let mut b = Bitmap::new(8, 8);
        f.draw(&mut b, "!", 0, 0, &|v| (v == 15).then_some(0xe1));
        assert_eq!(&b.pixels[..2], &[0xe1, 0]);
    }
}
