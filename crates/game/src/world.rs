//! Questions about the level's layout: neighbouring rooms and the collision grid.
//!
//! Each room's grid is 16 columns of 16 pixels by 7 rows. Rows 2, 4 and 6 are the ground of the
//! top, middle and bottom floors; rows 1, 3 and 5 the space a standing character fills above them,
//! where a set cell is a wall.

use of_engine::level::{ConnectionTable, Dir, GRID_W};

/// The level layout seen by moving characters.
pub struct World<'a> {
    pub ct: &'a ConnectionTable,
}

impl World<'_> {
    pub fn left(&self, room: usize) -> Option<usize> {
        self.ct.neighbour(room, Dir::Left)
    }

    pub fn right(&self, room: usize) -> Option<usize> {
        self.ct.neighbour(room, Dir::Right)
    }

    pub fn above(&self, room: usize) -> Option<usize> {
        self.ct.neighbour(room, Dir::Up)
    }

    pub fn below(&self, room: usize) -> Option<usize> {
        self.ct.neighbour(room, Dir::Down)
    }

    /// The room and column holding `col` of `room`, which may lie in a neighbour.
    fn locate(&self, room: usize, col: i32) -> Option<(usize, usize)> {
        let w = GRID_W as i32;
        if col < 0 {
            Some((self.left(room)?, (col + w) as usize))
        } else if col >= w {
            Some((self.right(room)?, (col - w) as usize))
        } else {
            Some((room, col as usize))
        }
    }

    /// The room and floor holding floor `floor` of `room`, which may lie above or below it.
    fn floor_room(&self, room: usize, floor: i32) -> Option<(usize, usize)> {
        match floor {
            -1 => Some((self.above(room)?, 2)),
            0..=2 => Some((room, floor as usize)),
            3 => Some((self.below(room)?, 0)),
            _ => None,
        }
    }

    fn cell(&self, room: usize, col: i32, floor: i32, row_offset: usize) -> bool {
        let Some((room, floor)) = self.floor_room(room, floor) else {
            return false;
        };
        let Some((room, col)) = self.locate(room, col) else {
            return false;
        };
        self.ct.grid(room)[(1 + floor * 2 + row_offset) * GRID_W + col] != 0
    }

    /// Whether there is ground at `col` of floor `floor` (0 top to 2 bottom; -1 and 3 reach into the
    /// rooms above and below).
    pub fn floor(&self, room: usize, col: i32, floor: i32) -> bool {
        self.cell(room, col, floor, 1)
    }

    /// Whether a wall blocks `col` at the height of a character standing on floor `floor`.
    /// Outside the level counts as a wall.
    pub fn wall(&self, room: usize, col: i32, floor: i32) -> bool {
        if self.locate(room, col).is_none() {
            return true;
        }
        self.cell(room, col, floor, 0)
    }
}
