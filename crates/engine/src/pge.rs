//! Initial state of a level's objects (`levelN.pge`): a count, then 32-byte records. Record 0 is
//! Conrad.

/// The starting state of one object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PgeInit {
    /// Animation the object starts in.
    pub anim: u16,
    pub x: i16,
    /// Feet height: 70, 142 or 214 for the three floors of a room.
    pub y: i16,
    pub obj_node: u16,
    pub life: i16,
    pub object_type: u8,
    pub room: u8,
}

const RECORD: usize = 32;

/// Reads every record of a `.pge` file.
pub fn parse_pge(data: &[u8]) -> Vec<PgeInit> {
    let count = data.get(..2).map_or(0, |c| u16::from_le_bytes([c[0], c[1]]) as usize);
    data.get(2..)
        .unwrap_or(&[])
        .as_chunks::<RECORD>()
        .0
        .iter()
        .take(count)
        .map(|r| {
            let w = |i: usize| u16::from_le_bytes([r[i], r[i + 1]]);
            PgeInit {
                anim: w(0),
                x: w(2) as i16,
                y: w(4) as i16,
                obj_node: w(6),
                life: w(8) as i16,
                object_type: r[18],
                room: r[19],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_are_32_bytes_after_a_count() {
        let mut d = vec![1, 0];
        let mut r = [0u8; RECORD];
        r[..10].copy_from_slice(&[99, 0, 32, 0, 70, 0, 1, 0, 5, 0]);
        r[18] = 2;
        r[19] = 27;
        d.extend_from_slice(&r);
        d.extend_from_slice(&r); // beyond the count: ignored
        let p = parse_pge(&d);
        assert_eq!(p.len(), 1);
        assert_eq!((p[0].anim, p[0].x, p[0].y, p[0].room, p[0].life), (99, 32, 70, 27, 5));
    }
}
