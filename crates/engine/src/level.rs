//! Level backgrounds (`levelN.map`) and the room connection table (`levelN.ct`).

use crate::screen::Bitmap;
use crate::unpack::{unpack, UnpackError};
use crate::{SCREEN_H, SCREEN_W};

/// Number of room slots in a level.
pub const ROOMS: usize = 64;

/// The base names of the five levels' background and palette files.
pub const LEVEL_FILES: [&str; 5] = ["level1", "level2", "level3", "level4", "level5"];

/// One room background.
#[derive(Debug, Clone)]
pub struct Room {
    /// Level palettes used by the room: `slots[0]` colours pixels `0x00..0x0f`, `slots[1]` pixels
    /// `0x80..0x8f` (the foreground layer drawn over characters).
    pub slots: [u8; 4],
    pub bitmap: Bitmap,
}

/// A level's background file: an offset table of 64 rooms followed by their pictures.
#[derive(Debug, Clone)]
pub struct LevelMap {
    data: Vec<u8>,
}

impl LevelMap {
    pub fn new(data: Vec<u8>) -> LevelMap {
        LevelMap { data }
    }

    /// Rooms that have a picture.
    pub fn rooms(&self) -> Vec<usize> {
        (0..ROOMS).filter(|&r| self.offset(r).is_some()).collect()
    }

    fn offset(&self, room: usize) -> Option<usize> {
        let e = self.data.get(room * 6..room * 6 + 4)?;
        let off = u32::from_le_bytes([e[0], e[1], e[2], e[3]]) as usize;
        (off != 0 && off < self.data.len()).then_some(off)
    }

    /// Decodes `room`: four palette slots, then four run-length packed bands of 256 x 56 pixels,
    /// top to bottom, each preceded by its packed size.
    pub fn room(&self, room: usize) -> Option<Room> {
        let mut p = self.offset(room)?;
        let d = &self.data;
        let slots = d.get(p..p + 4)?.try_into().ok()?;
        p += 4;
        let band = SCREEN_W * SCREEN_H / 4;
        let mut pixels = Vec::with_capacity(SCREEN_W * SCREEN_H);
        for _ in 0..4 {
            let size = u16::from_le_bytes([*d.get(p)?, *d.get(p + 1)?]) as usize;
            p += 2;
            let packed = d.get(p..p + size)?;
            p += size;
            let start = pixels.len();
            unpack_rle(packed, &mut pixels, band);
            pixels.resize(start + band, 0);
        }
        Some(Room {
            slots,
            bitmap: Bitmap {
                w: SCREEN_W,
                h: SCREEN_H,
                pixels,
            },
        })
    }
}

/// Byte run-length decoding: a control byte `c < 0x80` copies the next `c + 1` bytes, otherwise the
/// next byte repeats `257 - c` times. Stops after `limit` bytes of output.
fn unpack_rle(src: &[u8], out: &mut Vec<u8>, limit: usize) {
    let end = out.len() + limit;
    let mut i = 0;
    while i < src.len() && out.len() < end {
        let c = src[i] as usize;
        i += 1;
        if c < 0x80 {
            let n = (c + 1).min(src.len() - i);
            out.extend_from_slice(&src[i..i + n]);
            i += n;
        } else if i < src.len() {
            out.extend(std::iter::repeat_n(src[i], 257 - c));
            i += 1;
        }
    }
    out.truncate(end);
}

/// Direction of a neighbouring room in the connection table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Up = 0,
    Down = 1,
    Right = 2,
    Left = 3,
}

/// The unpacked connection table: for each direction one byte per room naming the neighbour,
/// then a 16 x 7 collision grid per room.
#[derive(Debug, Clone)]
pub struct ConnectionTable {
    data: Vec<u8>,
}

/// Columns in a room's collision grid (16 pixels each).
pub const GRID_W: usize = 16;
/// Rows in a room's collision grid.
pub const GRID_H: usize = 7;

impl ConnectionTable {
    /// Unpacks a `levelN.ct` file.
    pub fn parse(packed: &[u8]) -> Result<ConnectionTable, UnpackError> {
        let mut data = unpack(packed)?;
        data.resize(0x100 + ROOMS * GRID_W * GRID_H, 0);
        Ok(ConnectionTable { data })
    }

    /// The room next to `room` in direction `dir`, if any.
    pub fn neighbour(&self, room: usize, dir: Dir) -> Option<usize> {
        let r = *self.data.get(dir as usize * ROOMS + room)? as usize;
        (r < ROOMS && r != room).then_some(r)
    }

    /// The collision grid of `room`, row by row.
    pub fn grid(&self, room: usize) -> &[u8] {
        let start = 0x100 + room.min(ROOMS - 1) * GRID_W * GRID_H;
        &self.data[start..start + GRID_W * GRID_H]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_copies_and_repeats() {
        let mut out = Vec::new();
        unpack_rle(&[1, 7, 8, 0xfe, 9], &mut out, 100);
        assert_eq!(out, vec![7, 8, 9, 9, 9]);
    }

    #[test]
    fn rle_stops_at_limit() {
        let mut out = Vec::new();
        unpack_rle(&[0x80, 5], &mut out, 10);
        assert_eq!(out.len(), 10);
    }
}
