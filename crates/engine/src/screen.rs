//! 8-bit indexed bitmaps and the full-screen pictures (`menuN.map`, `instru_f.map`).

use crate::{SCREEN_H, SCREEN_W};

/// An 8-bit indexed image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap {
    pub w: usize,
    pub h: usize,
    pub pixels: Vec<u8>,
}

impl Bitmap {
    /// A `w` x `h` image filled with colour 0.
    pub fn new(w: usize, h: usize) -> Bitmap {
        Bitmap {
            w,
            h,
            pixels: vec![0; w * h],
        }
    }

    /// A blank image the size of the game screen.
    pub fn screen() -> Bitmap {
        Bitmap::new(SCREEN_W, SCREEN_H)
    }

    /// Draws `src` with its top-left corner at (`x`, `y`), skipping colour 0, adding `base` to every
    /// colour and mirroring horizontally when `flip` is set. Parts outside the image are clipped.
    pub fn blit(&mut self, src: &Bitmap, x: i32, y: i32, base: u8, flip: bool) {
        self.blit_masked(src, x, y, base, flip, 0);
    }

    /// Like [`Bitmap::blit`], but leaves pixels of this image that have any bit of `mask` set:
    /// with `0x80`, a sprite passes behind a room's foreground layer.
    pub fn blit_masked(&mut self, src: &Bitmap, x: i32, y: i32, base: u8, flip: bool, mask: u8) {
        for sy in 0..src.h {
            let dy = y + sy as i32;
            if dy < 0 || dy >= self.h as i32 {
                continue;
            }
            let row = &src.pixels[sy * src.w..(sy + 1) * src.w];
            for (sx, &c) in row.iter().enumerate() {
                if c == 0 {
                    continue;
                }
                let dx = if flip {
                    x + (src.w - 1 - sx) as i32
                } else {
                    x + sx as i32
                };
                if dx >= 0 && dx < self.w as i32 {
                    let d = &mut self.pixels[dy as usize * self.w + dx as usize];
                    if *d & mask == 0 {
                        *d = c.wrapping_add(base);
                    }
                }
            }
        }
    }
}

/// Decodes a full-screen picture: four planes of 64 x 224 bytes, plane `i` holding every pixel
/// whose x is `i` modulo 4 (the VGA "mode X" layout).
pub fn decode_planar_screen(data: &[u8]) -> Bitmap {
    let mut bmp = Bitmap::screen();
    let plane = SCREEN_W / 4 * SCREEN_H;
    for (i, &c) in data.iter().take(plane * 4).enumerate() {
        let (p, j) = (i / plane, i % plane);
        let (y, x) = (j / (SCREEN_W / 4), j % (SCREEN_W / 4));
        bmp.pixels[y * SCREEN_W + x * 4 + p] = c;
    }
    bmp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planes_interleave_by_column() {
        let plane = SCREEN_W / 4 * SCREEN_H;
        let mut data = vec![0u8; plane * 4];
        data[0] = 1; // plane 0, x 0
        data[plane] = 2; // plane 1, x 1
        data[plane * 3 + 1] = 3; // plane 3, x 7
        let b = decode_planar_screen(&data);
        assert_eq!(&b.pixels[..8], &[1, 2, 0, 0, 0, 0, 0, 3]);
    }

    #[test]
    fn blit_clips_flips_and_skips_transparent() {
        let mut dst = Bitmap::new(4, 1);
        let src = Bitmap {
            w: 3,
            h: 1,
            pixels: vec![1, 0, 2],
        };
        dst.blit(&src, 2, 0, 0x40, true);
        assert_eq!(dst.pixels, vec![0, 0, 0x42, 0]);
    }

    #[test]
    fn masked_blit_stays_behind_foreground_pixels() {
        let mut dst = Bitmap {
            w: 3,
            h: 1,
            pixels: vec![0x01, 0x85, 0x02],
        };
        let src = Bitmap {
            w: 3,
            h: 1,
            pixels: vec![7, 7, 7],
        };
        dst.blit_masked(&src, 0, 0, 0x40, false, 0x80);
        assert_eq!(dst.pixels, vec![0x47, 0x85, 0x47]);
    }
}
