//! Conrad's moves: a small state machine choosing among the original animations, with the room
//! collision grid deciding where he can stand, walk and climb.
//!
//! This stands in for the level scripts (`.obj`), which drive the original game; the animation
//! numbers below are the same in every level's `.ani`.

use crate::world::World;
use of_engine::anim::{AnimFrame, AnimTable};
use of_engine::SCREEN_W;
use std::collections::VecDeque;

/// Animation numbers of Conrad's moves in `levelN.ani`.
pub mod anims {
    pub const TURN: u16 = 0;
    pub const STAND: u16 = 1;
    /// The two halves of a walking step cycle, each moving one 16-pixel cell.
    pub const WALK: [u16; 2] = [3, 2];
    pub const JUMP_UP: u16 = 4;
    pub const JUMP_UP_LAND: u16 = 5;
    /// The running cycle, each moving one cell.
    pub const RUN: [u16; 4] = [6, 17, 7, 18];
    pub const RUN_START: u16 = 8;
    pub const RUN_STOP: u16 = 10;
    pub const JUMP_STANDING: u16 = 13;
    pub const CLIMB_DOWN: u16 = 15;
    pub const PULL_UP: u16 = 16;
    pub const JUMP_RUNNING: u16 = 21;
    pub const JUMP_FLY: u16 = 22;
    pub const JUMP_FALL: u16 = 23;
    pub const JUMP_LAND: u16 = 24;
}

/// Width of a collision cell in pixels: every move goes a whole number of cells.
pub const CELL: i32 = 16;
/// Feet height of the top floor of a room; the others are [`FLOOR_STEP`] apart.
pub const FLOOR_TOP: i32 = 70;
pub const FLOOR_STEP: i32 = 72;
const FLOORS: i32 = 3;
/// Falling speed limit in pixels per tick.
const MAX_FALL: i32 = 8;

/// Buttons held this tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    /// Run modifier (Shift in the original).
    pub run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Anim(u16),
    /// Moves Conrad `floors` floors down (negative: up), into the next room past the top or
    /// bottom floor, and `cells` cells forward.
    ToFloor {
        floors: i32,
        cells: i32,
    },
    Turn,
    Run,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Motion {
    Idle,
    Running,
    Falling { vy: i32 },
}

/// The player character.
#[derive(Debug, Clone)]
pub struct Conrad {
    pub room: usize,
    /// Position in room pixels; `y` is the height of his feet.
    pub x: i32,
    pub y: i32,
    /// Sprites are drawn facing left; facing right mirrors them and their movement.
    pub facing_left: bool,
    anim: u16,
    frames: Vec<AnimFrame>,
    frame: usize,
    queue: VecDeque<Step>,
    motion: Motion,
    walk_phase: usize,
    run_phase: usize,
    /// Up and Down presses seen since the last decision, with the direction held at the time, so
    /// a quick tap during an animation counts.
    latch: Input,
}

impl Conrad {
    /// Conrad at (`x`, `y`) in `room`, starting animation `anim`.
    pub fn new(room: usize, x: i32, y: i32, anim: u16, anims: &AnimTable) -> Conrad {
        let mut c = Conrad {
            room,
            x,
            y,
            facing_left: true,
            anim,
            frames: Vec::new(),
            frame: 0,
            queue: VecDeque::new(),
            motion: Motion::Idle,
            walk_phase: 0,
            run_phase: 0,
            latch: Input::default(),
        };
        c.start(anim, anims);
        c
    }

    /// The sprite to draw and whether to mirror it.
    pub fn sprite(&self) -> Option<(u16, bool)> {
        let f = self.frames.get(self.frame)?;
        Some((f.sprite, f.mirror != !self.facing_left))
    }

    /// The animation playing.
    pub fn anim(&self) -> u16 {
        self.anim
    }

    pub fn is_falling(&self) -> bool {
        matches!(self.motion, Motion::Falling { .. })
    }

    fn start(&mut self, anim: u16, anims: &AnimTable) {
        self.anim = anim;
        self.frames = anims.frames(anim as usize).unwrap_or_default();
        self.frame = 0;
        if let Some(f) = self.frames.first().copied() {
            self.apply(f);
        }
    }

    /// Moves by a frame's offsets, mirrored when facing right.
    fn apply(&mut self, f: AnimFrame) {
        let dx = f.dx as i32;
        self.x += if self.facing_left { dx } else { -dx };
        self.y += f.dy as i32;
    }

    /// Cell column under Conrad.
    pub fn col(&self) -> i32 {
        self.x.div_euclid(CELL)
    }

    /// Index of the floor he stands on (0 top to 2 bottom), if his feet are at floor height.
    pub fn floor(&self) -> Option<i32> {
        let d = self.y - FLOOR_TOP;
        (d % FLOOR_STEP == 0 && (0..FLOORS).contains(&(d / FLOOR_STEP))).then_some(d / FLOOR_STEP)
    }

    fn forward(&self) -> i32 {
        if self.facing_left {
            -1
        } else {
            1
        }
    }

    /// Advances one tick.
    pub fn tick(&mut self, input: Input, anims: &AnimTable, world: &World) {
        if input.up || input.down {
            // Remember the direction held with the press too: Up with Right is a jump forward.
            self.latch.up |= input.up;
            self.latch.down |= input.down;
            self.latch.left |= input.left;
            self.latch.right |= input.right;
        }
        if let Motion::Falling { vy } = self.motion {
            self.fall(vy, anims, world);
            self.cross_rooms(world);
            return;
        }
        if self.frame + 1 < self.frames.len() {
            self.frame += 1;
            let f = self.frames[self.frame];
            self.apply(f);
            self.cross_rooms(world);
            return;
        }
        while let Some(step) = self.queue.pop_front() {
            match step {
                Step::Anim(a) => {
                    self.start(a, anims);
                    self.cross_rooms(world);
                    return;
                }
                Step::ToFloor { floors, cells } => {
                    self.x += cells * CELL * self.forward();
                    self.cross_rooms(world);
                    self.change_floor(floors, world);
                }
                Step::Turn => self.facing_left = !self.facing_left,
                Step::Run => self.motion = Motion::Running,
            }
        }
        let latched = Input {
            up: input.up || self.latch.up,
            down: input.down || self.latch.down,
            left: input.left || self.latch.left,
            right: input.right || self.latch.right,
            run: input.run,
        };
        self.latch = Input::default();
        self.decide(latched, anims, world);
        self.cross_rooms(world);
    }

    /// Chooses the next move once the current animation has ended.
    fn decide(&mut self, input: Input, anims: &AnimTable, world: &World) {
        let Some(f) = self.floor() else {
            self.motion = Motion::Falling { vy: 1 };
            return;
        };
        let col = self.col();
        if !world.floor(self.room, col, f) {
            self.motion = Motion::Falling { vy: 1 };
            return;
        }
        let dir = self.forward();
        let held_forward = if self.facing_left { input.left } else { input.right };
        let held_back = if self.facing_left { input.right } else { input.left };
        let running = self.motion == Motion::Running;
        let mut plan: Vec<Step> = Vec::new();

        if input.up && held_forward {
            let take_off = if running {
                anims::JUMP_RUNNING
            } else {
                anims::JUMP_STANDING
            };
            plan.extend([take_off, anims::JUMP_FLY, anims::JUMP_FALL, anims::JUMP_LAND].map(Step::Anim));
            self.motion = Motion::Idle;
        } else if input.up {
            plan.push(Step::Anim(anims::JUMP_UP));
            if world.floor(self.room, col, f - 1) && !running {
                plan.extend([Step::ToFloor { floors: -1, cells: 0 }, Step::Anim(anims::PULL_UP)]);
            } else {
                plan.push(Step::Anim(anims::JUMP_UP_LAND));
            }
            self.motion = Motion::Idle;
        } else if input.down && !world.floor(self.room, col + dir, f) && world.floor(self.room, col + dir, f + 1) {
            plan.extend([
                Step::Anim(anims::CLIMB_DOWN),
                Step::ToFloor { floors: 1, cells: 1 },
                Step::Anim(anims::JUMP_UP_LAND),
            ]);
            self.motion = Motion::Idle;
        } else if held_back {
            if running {
                plan.push(Step::Anim(anims::RUN_STOP));
            }
            plan.extend([Step::Anim(anims::TURN), Step::Turn]);
            self.motion = Motion::Idle;
        } else if held_forward && !world.wall(self.room, col + dir, f) {
            if input.run && running {
                plan.push(Step::Anim(anims::RUN[self.run_phase]));
                self.run_phase = (self.run_phase + 1) % anims::RUN.len();
            } else if input.run {
                plan.extend([Step::Anim(anims::RUN_START), Step::Run]);
                self.run_phase = 0;
            } else {
                if running {
                    plan.push(Step::Anim(anims::RUN_STOP));
                    self.motion = Motion::Idle;
                }
                plan.push(Step::Anim(anims::WALK[self.walk_phase]));
                self.walk_phase = (self.walk_phase + 1) % anims::WALK.len();
            }
        } else if running {
            plan.push(Step::Anim(anims::RUN_STOP));
            self.motion = Motion::Idle;
        } else {
            plan.push(Step::Anim(anims::STAND));
        }
        self.queue.extend(plan);
        if let Some(Step::Anim(a)) = self.queue.pop_front() {
            self.start(a, anims);
        }
    }

    /// Falls until his feet cross the height of a floor that has ground under him.
    fn fall(&mut self, vy: i32, anims: &AnimTable, world: &World) {
        if self.anim != anims::JUMP_FALL {
            self.anim = anims::JUMP_FALL;
            self.frames = anims.frames(anims::JUMP_FALL as usize).unwrap_or_default();
            self.frame = self.frames.len().saturating_sub(1);
        }
        let from = self.y;
        self.y += vy;
        self.motion = Motion::Falling {
            vy: (vy + 1).min(MAX_FALL),
        };
        for f in 0..FLOORS {
            let line = FLOOR_TOP + f * FLOOR_STEP;
            if from < line && self.y >= line && world.floor(self.room, self.col(), f) {
                self.y = line;
                self.motion = Motion::Idle;
                self.queue.clear();
                self.start(anims::JUMP_LAND, anims);
                return;
            }
        }
    }

    /// Puts his feet `floors` floors lower (negative: higher), changing rooms past the top or
    /// bottom floor.
    fn change_floor(&mut self, floors: i32, world: &World) {
        let Some(f) = self.floor() else { return };
        let to = f + floors;
        let (room, to) = if to < 0 {
            (world.above(self.room), to + FLOORS)
        } else if to >= FLOORS {
            (world.below(self.room), to - FLOORS)
        } else {
            (Some(self.room), to)
        };
        if let Some(r) = room {
            self.room = r;
            self.y = FLOOR_TOP + to * FLOOR_STEP;
        }
    }

    /// Moves into the neighbouring room when he walks or falls past the screen edge.
    fn cross_rooms(&mut self, world: &World) {
        let w = SCREEN_W as i32;
        if self.x < 0 {
            match world.left(self.room) {
                Some(r) => {
                    self.room = r;
                    self.x += w;
                }
                None => self.x = 0,
            }
        } else if self.x >= w {
            match world.right(self.room) {
                Some(r) => {
                    self.room = r;
                    self.x -= w;
                }
                None => self.x = w - 1,
            }
        }
        let h = of_engine::SCREEN_H as i32;
        if self.y >= h {
            if let Some(r) = world.below(self.room) {
                self.room = r;
                self.y -= h;
            } else {
                // Nothing below: back on the bottom floor of this room.
                self.y = FLOOR_TOP + (FLOORS - 1) * FLOOR_STEP;
                self.motion = Motion::Idle;
            }
        } else if self.y < 0 {
            if let Some(r) = world.above(self.room) {
                self.room = r;
                self.y += h;
            }
        }
    }
}
