//! Character sprite banks: `perso.spr` (Conrad) and the enemies' `.spr` files, indexed by `.off`.
//!
//! The `.off` file lists `(id: u16, offset: u32)` little-endian pairs ending with id `0xffff`.
//! Offsets count from byte 12 of the `.spr` file. Each frame starts with four header bytes, then a
//! big-endian count of packed bytes holding 4-bit pixels (colour 0 is transparent).

use crate::screen::Bitmap;
use std::collections::HashMap;

const SPR_HEADER: usize = 12;

/// A bank of character frames.
#[derive(Debug, Clone)]
pub struct SpriteBank {
    data: Vec<u8>,
    offsets: HashMap<u16, usize>,
}

impl SpriteBank {
    /// Reads a `.spr` file with its `.off` index.
    pub fn parse(spr: Vec<u8>, off: &[u8]) -> SpriteBank {
        let mut offsets = HashMap::new();
        for e in off.as_chunks::<6>().0 {
            let id = u16::from_le_bytes([e[0], e[1]]);
            if id == 0xffff {
                break;
            }
            offsets.insert(id, u32::from_le_bytes([e[2], e[3], e[4], e[5]]) as usize + SPR_HEADER);
        }
        SpriteBank { data: spr, offsets }
    }

    /// Frame ids in the bank, sorted.
    pub fn ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = self.offsets.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// Decodes frame `id`.
    ///
    /// Header bytes 2 and 3 are width and height. When byte 2 has bit `0x40` set the pixels are
    /// stored column by column instead, and the two sizes swap places.
    pub fn frame(&self, id: u16) -> Option<Bitmap> {
        let p = *self.offsets.get(&id)?;
        let h = self.data.get(p..p + 6)?;
        let (a, b) = (h[2], h[3]);
        let columns = a & 0x40 != 0;
        let (w, h, count) = if columns {
            (
                b as usize,
                (a & 0x3f) as usize,
                u16::from_be_bytes([h[4], h[5]]) as usize,
            )
        } else {
            (a as usize, b as usize, u16::from_be_bytes([h[4], h[5]]) as usize)
        };
        let packed = self.data.get(p + 6..p + 6 + count)?;
        let raw = unpack_nibbles(packed, w * h);
        let mut bmp = Bitmap::new(w, h);
        for (i, &c) in raw.iter().enumerate() {
            let (x, y) = if columns { (i / h, i % h) } else { (i % w, i / w) };
            bmp.pixels[y * w + x] = c;
        }
        Some(bmp)
    }
}

/// Expands 4-bit run-length data (high nibble first). Nibble 15 starts a run: `15, colour, n` repeats
/// `colour` `n + 4` times, and `15, 15, hi, lo, colour` repeats `colour` `(hi << 4 | lo) + 4` times.
fn unpack_nibbles(src: &[u8], size: usize) -> Vec<u8> {
    let mut nib = src.iter().flat_map(|&b| [b >> 4, b & 15]);
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        let Some(c) = nib.next() else { break };
        if c != 15 {
            out.push(c);
            continue;
        }
        let (Some(mut colour), Some(mut n)) = (nib.next(), nib.next()) else {
            break;
        };
        if colour == 15 {
            let (Some(lo), Some(c2)) = (nib.next(), nib.next()) else {
                break;
            };
            colour = c2;
            n = n << 4 | lo;
        }
        out.extend(std::iter::repeat_n(colour, n as usize + 4));
    }
    out.resize(size, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nibble_runs_expand() {
        // 1, 2, run of colour 3 x (0 + 4), long run of colour 2 x (0x15 + 4)
        let out = unpack_nibbles(&[0x12, 0xf3, 0x0f, 0xf1, 0x52], 31);
        assert_eq!(&out[..6], &[1, 2, 3, 3, 3, 3]);
        assert_eq!(out[6..].iter().filter(|&&c| c == 2).count(), 25);
    }

    #[test]
    fn column_frames_are_transposed() {
        // 12 byte file header, frame header with column flag: width 2, height 3.
        let mut spr = vec![0u8; 12];
        spr.extend_from_slice(&[0, 0, 0x43, 2, 0, 3, 0x12, 0x34, 0x56]);
        let off = [0, 0, 0, 0, 0, 0, 0xff, 0xff];
        let f = SpriteBank::parse(spr, &off).frame(0).unwrap();
        assert_eq!((f.w, f.h), (2, 3));
        assert_eq!(f.pixels, vec![1, 4, 2, 5, 3, 6]);
    }
}
