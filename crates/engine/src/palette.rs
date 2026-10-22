//! Palettes: 8-bit RGB triplets for full-screen pictures and 12-bit `0RGB` words for levels.

/// An 8-bit per channel colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Expands a big-endian 12-bit `0x0RGB` colour word.
    pub fn from_12bit(w: u16) -> Rgb {
        let c = |n: u16| ((n & 15) * 17) as u8;
        Rgb(c(w >> 8), c(w >> 4), c(w))
    }

    /// Packs as `0x00RRGGBB`, the pixel format of the window surface.
    pub fn to_u32(self) -> u32 {
        (self.0 as u32) << 16 | (self.1 as u32) << 8 | self.2 as u32
    }
}

/// A 256-colour palette.
#[derive(Debug, Clone)]
pub struct Palette(pub [Rgb; 256]);

impl Default for Palette {
    fn default() -> Self {
        Palette([Rgb::default(); 256])
    }
}

impl Palette {
    /// Reads 256 RGB triplets (menu `.pal` files are 768 bytes of 8-bit components).
    pub fn from_rgb8(data: &[u8]) -> Palette {
        let mut p = Palette::default();
        for (i, c) in data.as_chunks::<3>().0.iter().take(256).enumerate() {
            p.0[i] = Rgb(c[0], c[1], c[2]);
        }
        p
    }

    /// Copies 16 colours into entries `base..base + 16`.
    pub fn set_slot(&mut self, base: usize, colours: &[Rgb; 16]) {
        self.0[base..base + 16].copy_from_slice(colours);
    }
}

/// The 16-colour palettes of a level (`levelN.pal`: 16 big-endian `0RGB` words each).
#[derive(Debug, Clone)]
pub struct LevelPalettes(pub Vec<[Rgb; 16]>);

impl LevelPalettes {
    /// Parses a `levelN.pal` file.
    pub fn parse(data: &[u8]) -> LevelPalettes {
        LevelPalettes(
            data.as_chunks::<32>()
                .0
                .iter()
                .map(|slot| {
                    std::array::from_fn(|i| Rgb::from_12bit(u16::from_be_bytes([slot[i * 2], slot[i * 2 + 1]])))
                })
                .collect(),
        )
    }

    /// Palette `n`, or black when the level has fewer.
    pub fn get(&self, n: usize) -> [Rgb; 16] {
        self.0.get(n).copied().unwrap_or([Rgb::default(); 16])
    }
}

/// Number of 16-colour palettes in the character palette table of `FB.EXE`.
const CHARACTER_PALETTES: usize = 7;
/// Index of Conrad's normal colours in that table.
const CONRAD_PALETTE: usize = 4;

/// Finds Conrad's palette in the DOS executable.
///
/// The executable holds a table of seven 16-colour palettes as little-endian `0RGB` words, each
/// starting with black. The table is found by that shape rather than by a fixed offset.
pub fn conrad_palette_from_exe(exe: &[u8]) -> Option<[Rgb; 16]> {
    let word = |p: usize| u16::from_le_bytes([exe[p], exe[p + 1]]);
    let valid = |p: usize| {
        p + 32 <= exe.len() && word(p) == 0 && (0..16).all(|i| word(p + i * 2) < 0x1000) && {
            let mut w: Vec<u16> = (0..16).map(|i| word(p + i * 2)).collect();
            w.sort_unstable();
            w.dedup();
            w.len() >= 10
        }
    };
    let base = (0..exe.len().saturating_sub(32 * CHARACTER_PALETTES))
        .step_by(2)
        .find(|&p| (0..CHARACTER_PALETTES).all(|k| valid(p + k * 32)))?;
    let p = base + CONRAD_PALETTE * 32;
    Some(std::array::from_fn(|i| Rgb::from_12bit(word(p + i * 2))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conrad_palette_is_fifth_of_seven_in_table() {
        let mut exe = vec![0xffu8; 10];
        for k in 0..7u16 {
            for i in 0..16u16 {
                let w = if i == 0 { 0 } else { (k << 8) | (i * 3) };
                exe.extend_from_slice(&w.to_le_bytes());
            }
        }
        exe.extend_from_slice(&[0xff; 6]);
        let p = conrad_palette_from_exe(&exe).unwrap();
        assert_eq!(p[0], Rgb(0, 0, 0));
        assert_eq!(p[1], Rgb::from_12bit(0x0403));
    }

    #[test]
    fn twelve_bit_colours_expand_to_full_range() {
        assert_eq!(Rgb::from_12bit(0x0f80), Rgb(255, 136, 0));
        assert_eq!(Rgb::from_12bit(0x0fff).to_u32(), 0xffffff);
    }

    #[test]
    fn level_palettes_are_16_words() {
        let mut data = vec![0u8; 64];
        data[32] = 0x0f;
        data[33] = 0xff;
        let p = LevelPalettes::parse(&data);
        assert_eq!(p.0.len(), 2);
        assert_eq!(p.get(1)[0], Rgb(255, 255, 255));
    }
}
