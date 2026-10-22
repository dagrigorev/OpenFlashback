//! Animation tables (`levelN.ani`): sequences of sprite frames with per-frame movement.
//!
//! The file starts with a count and a table of offsets (counted from byte 2). Each animation is a
//! frame count byte, five more header bytes, then four bytes per frame: a sprite number whose top
//! bit mirrors the sprite, and signed x and y movement applied when the frame is shown. Sprites
//! face left; for a character facing right both the picture and the x movement are mirrored.

/// One frame of an animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimFrame {
    /// Frame number in the character's sprite bank.
    pub sprite: u16,
    /// Draw the sprite mirrored (on top of the character's own facing).
    pub mirror: bool,
    /// Movement in pixels for a character facing left.
    pub dx: i8,
    pub dy: i8,
}

/// The animations of a level.
#[derive(Debug, Clone)]
pub struct AnimTable {
    data: Vec<u8>,
    count: usize,
}

impl AnimTable {
    pub fn parse(data: Vec<u8>) -> AnimTable {
        let count = data.get(..2).map_or(0, |c| u16::from_le_bytes([c[0], c[1]]) as usize);
        let count = count.min(data.len().saturating_sub(2) / 2);
        AnimTable { data, count }
    }

    /// Number of animations.
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The frames of animation `n`.
    pub fn frames(&self, n: usize) -> Option<Vec<AnimFrame>> {
        if n >= self.count {
            return None;
        }
        let d = &self.data;
        let p = 2 + u16::from_le_bytes([d[2 + n * 2], d[3 + n * 2]]) as usize;
        let count = *d.get(p)? as usize;
        let frames = d.get(p + 6..p + 6 + count * 4)?;
        Some(
            frames
                .as_chunks::<4>()
                .0
                .iter()
                .map(|f| {
                    let s = u16::from_le_bytes([f[0], f[1]]);
                    AnimFrame {
                        sprite: s & 0x7fff,
                        mirror: s & 0x8000 != 0,
                        dx: f[2] as i8,
                        dy: f[3] as i8,
                    }
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_follow_the_offset_table() {
        // Two animations; the second has one mirrored frame moving (-16, 2).
        let mut d = vec![2, 0, 4, 0, 10, 0];
        d.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        d.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0x05, 0x80, 0xf0, 0x02]);
        let t = AnimTable::parse(d);
        assert_eq!(t.len(), 2);
        assert_eq!(t.frames(0).unwrap(), vec![]);
        assert_eq!(
            t.frames(1).unwrap(),
            vec![AnimFrame {
                sprite: 5,
                mirror: true,
                dx: -16,
                dy: 2
            }]
        );
        assert!(t.frames(2).is_none());
    }
}
