//! Object scripts (`levelN.obj`): for every kind of object, a list of transitions between states.
//!
//! A state is an animation number. When an object's animation ends, the transitions of its state
//! are tried in order; the first whose conditions hold moves it by `dx`, `dy`, runs the action
//! and makes `next_state` the new state, whose transitions start at record `next_record`.
//!
//! The file is a `u16` count of object nodes and a `u32` offset per node, counted from byte 2;
//! nodes may share an offset. A node is a `u16` record count followed by 18-byte records.

/// One transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    /// State (animation number) the transition leaves from.
    pub state: u16,
    pub dx: i8,
    pub dy: i8,
    pub next_state: u16,
    /// Condition opcodes; 0 means no condition. `cond1` values 1 to 8 test the controls, see
    /// [`input_condition`].
    pub cond2: u8,
    pub cond1: u8,
    pub flags: u8,
    /// Action opcode run when the transition is taken; 0 means none.
    pub action: u8,
    /// Index of the first transition of `next_state` in the node.
    pub next_record: u16,
    pub args: [i16; 3],
}

const RECORD: usize = 18;

/// The scripts of a level.
#[derive(Debug, Clone, Default)]
pub struct ObjectScripts {
    nodes: Vec<Vec<Transition>>,
    /// Node number to index in `nodes`.
    index: Vec<Option<usize>>,
}

impl ObjectScripts {
    pub fn parse(data: &[u8]) -> ObjectScripts {
        let u16_at = |p: usize| data.get(p..p + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
        let count = u16_at(0).unwrap_or(0) as usize;
        let mut s = ObjectScripts::default();
        let mut seen: Vec<(usize, usize)> = Vec::new();
        for n in 0..count {
            let Some(b) = data.get(2 + n * 4..6 + n * 4) else { break };
            let start = 2 + u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
            if let Some(&(_, i)) = seen.iter().find(|(o, _)| *o == start) {
                s.index.push(Some(i));
                continue;
            }
            let Some(records) = u16_at(start) else {
                s.index.push(None);
                continue;
            };
            let body = data.get(start + 2..).unwrap_or(&[]);
            let list: Vec<Transition> = body
                .as_chunks::<RECORD>()
                .0
                .iter()
                .take(records as usize)
                .map(parse_record)
                .collect();
            seen.push((start, s.nodes.len()));
            s.index.push(Some(s.nodes.len()));
            s.nodes.push(list);
        }
        s
    }

    /// Number of node slots (several may share one script).
    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// The transitions of node `n`.
    pub fn node(&self, n: usize) -> Option<&[Transition]> {
        Some(&self.nodes[(*self.index.get(n)?)?])
    }

    /// The transitions leaving `state` in node `n`, in the order they are tried.
    pub fn from_state(&self, n: usize, state: u16) -> Vec<&Transition> {
        self.node(n)
            .map_or(Vec::new(), |t| t.iter().filter(|t| t.state == state).collect())
    }
}

fn parse_record(r: &[u8; RECORD]) -> Transition {
    let w = |i: usize| u16::from_le_bytes([r[i], r[i + 1]]);
    Transition {
        state: w(0),
        dx: r[2] as i8,
        dy: r[3] as i8,
        next_state: w(4),
        cond2: r[6],
        cond1: r[7],
        flags: r[8],
        action: r[9],
        next_record: w(10),
        args: [w(12) as i16, w(14) as i16, w(16) as i16],
    }
}

/// Controls tested by a condition opcode, relative to the way the object faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputCondition {
    pub up: bool,
    pub down: bool,
    pub forward: bool,
    pub back: bool,
    /// The modifier key (Shift: run, take, use).
    pub modifier: bool,
}

/// The controls tested by condition opcodes 1 to 8: up, back, down, forward, then the same four
/// with the modifier held. Inferred from Conrad's script: back leads to the turn, forward with
/// the modifier to the start of running, up to jumping.
pub fn input_condition(op: u8) -> Option<InputCondition> {
    if !(1..=8).contains(&op) {
        return None;
    }
    let k = (op - 1) % 4;
    Some(InputCondition {
        up: k == 0,
        back: k == 1,
        down: k == 2,
        forward: k == 3,
        modifier: op > 4,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(state: u16, next: u16, cond1: u8, next_record: u16) -> [u8; RECORD] {
        let mut r = [0u8; RECORD];
        r[0..2].copy_from_slice(&state.to_le_bytes());
        r[2] = 0xf0; // dx -16
        r[4..6].copy_from_slice(&next.to_le_bytes());
        r[7] = cond1;
        r[10..12].copy_from_slice(&next_record.to_le_bytes());
        r[12..14].copy_from_slice(&(-4i16).to_le_bytes());
        r
    }

    #[test]
    fn nodes_share_offsets_and_records_parse() {
        // Two node slots pointing at the same script of two records.
        let mut d = vec![2, 0];
        d.extend_from_slice(&8u32.to_le_bytes());
        d.extend_from_slice(&8u32.to_le_bytes());
        d.extend_from_slice(&[2, 0]);
        d.extend_from_slice(&record(3, 2, 4, 1));
        d.extend_from_slice(&record(2, 3, 4, 0));
        let s = ObjectScripts::parse(&d);
        assert_eq!(s.len(), 2);
        assert_eq!(s.node(0), s.node(1));
        let t = s.from_state(1, 3);
        assert_eq!(t.len(), 1);
        assert_eq!(
            (t[0].next_state, t[0].cond1, t[0].dx, t[0].args[0], t[0].next_record),
            (2, 4, -16, -4, 1)
        );
    }

    #[test]
    fn input_opcodes_cover_four_directions_with_and_without_modifier() {
        assert!(input_condition(1).unwrap().up);
        assert!(input_condition(2).unwrap().back);
        assert!(input_condition(3).unwrap().down);
        let f = input_condition(8).unwrap();
        assert!(f.forward && f.modifier);
        assert!(input_condition(0).is_none() && input_condition(9).is_none());
    }
}
