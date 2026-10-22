//! Flashback game state. For now: loading a level, walking Conrad through its rooms and composing
//! the screen.

use of_engine::level::{ConnectionTable, Dir, LevelMap, Room, LEVEL_FILES};
use of_engine::palette::{conrad_palette_from_exe, LevelPalettes};
use of_engine::screen::Bitmap;
use of_engine::sprite::SpriteBank;
use of_engine::{DataDir, Palette, Rgb, SCREEN_W};
use std::io;

/// Palette entries reserved for Conrad's 16 colours.
pub const CONRAD_BASE: u8 = 0x40;

/// Feet height of the three floors of a room.
pub const FLOORS: [i32; 3] = [70, 142, 214];

/// Game updates per second.
pub const TICK_HZ: u32 = 60;

/// Conrad's standing frame in `perso.spr`.
const STAND_FRAME: u16 = 3;
/// Walking cycle frames in `perso.spr`.
const WALK_FRAMES: std::ops::RangeInclusive<u16> = 4..=11;
/// Ticks each walking frame is shown.
const WALK_FRAME_TICKS: u32 = 5;
/// Walking speed in pixels per tick.
const WALK_SPEED: i32 = 1;

/// A loaded level.
pub struct Level {
    pub index: usize,
    pub map: LevelMap,
    pub palettes: LevelPalettes,
    pub ct: ConnectionTable,
}

impl Level {
    /// Loads level `index` (0-based) from `data`.
    pub fn load(data: &DataDir, index: usize) -> io::Result<Level> {
        let name = LEVEL_FILES[index.min(LEVEL_FILES.len() - 1)];
        let map = LevelMap::new(data.read(&format!("{name}.map"))?);
        let palettes = LevelPalettes::parse(&data.read(&format!("{name}.pal"))?);
        let ct = ConnectionTable::parse(&data.read(&format!("{name}.ct"))?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{name}.ct: {e}")))?;
        Ok(Level {
            index,
            map,
            palettes,
            ct,
        })
    }

    /// The first room with a picture; where play starts until the level scripts are read.
    pub fn start_room(&self) -> usize {
        self.map.rooms().first().copied().unwrap_or(0)
    }
}

/// Buttons held this tick.
#[derive(Debug, Clone, Copy, Default)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

/// The player.
#[derive(Debug, Clone)]
pub struct Conrad {
    /// Horizontal centre in room pixels.
    pub x: i32,
    /// Floor index into [`FLOORS`].
    pub floor: usize,
    pub facing_left: bool,
    pub walking: bool,
    anim_ticks: u32,
}

/// The running game.
pub struct Game {
    pub level: Level,
    pub room: usize,
    room_cache: Option<(usize, Room)>,
    pub conrad: Conrad,
    perso: SpriteBank,
    conrad_palette: [Rgb; 16],
    prev: Input,
}

impl Game {
    /// Starts level `index` (0-based).
    pub fn new(data: &DataDir, index: usize) -> io::Result<Game> {
        let level = Level::load(data, index)?;
        let perso = SpriteBank::parse(data.read("perso.spr")?, &data.read("perso.off")?);
        let room = level.start_room();
        Ok(Game {
            level,
            room,
            room_cache: None,
            conrad: Conrad {
                x: 128,
                floor: 2,
                facing_left: false,
                walking: false,
                anim_ticks: 0,
            },
            perso,
            conrad_palette: data
                .read("fb.exe")
                .ok()
                .and_then(|exe| conrad_palette_from_exe(&exe))
                .unwrap_or_else(placeholder_conrad_palette),
            prev: Input::default(),
        })
    }

    /// Moves to `room` if the level has it.
    pub fn goto_room(&mut self, room: usize) -> bool {
        if self.level.map.room(room).is_none() {
            return false;
        }
        self.room = room;
        true
    }

    /// Advances one tick.
    pub fn tick(&mut self, input: Input) {
        let c = &mut self.conrad;
        let dx = input.right as i32 - input.left as i32;
        c.walking = dx != 0;
        if dx != 0 {
            c.facing_left = dx < 0;
            c.x += dx * WALK_SPEED;
            c.anim_ticks += 1;
        } else {
            c.anim_ticks = 0;
        }
        if input.up && !self.prev.up {
            self.change_floor(-1);
        }
        if input.down && !self.prev.down {
            self.change_floor(1);
        }
        self.prev = input;

        if self.conrad.x < 0 {
            match self.level.ct.neighbour(self.room, Dir::Left) {
                Some(r) if self.goto_room(r) => self.conrad.x += SCREEN_W as i32,
                _ => self.conrad.x = 0,
            }
        } else if self.conrad.x >= SCREEN_W as i32 {
            match self.level.ct.neighbour(self.room, Dir::Right) {
                Some(r) if self.goto_room(r) => self.conrad.x -= SCREEN_W as i32,
                _ => self.conrad.x = SCREEN_W as i32 - 1,
            }
        }
    }

    /// Steps one floor up (`-1`) or down (`1`), into the room above or below past the edge.
    fn change_floor(&mut self, step: i32) {
        let f = self.conrad.floor as i32 + step;
        if (0..FLOORS.len() as i32).contains(&f) {
            self.conrad.floor = f as usize;
            return;
        }
        let dir = if step < 0 { Dir::Up } else { Dir::Down };
        if let Some(r) = self.level.ct.neighbour(self.room, dir) {
            if self.goto_room(r) {
                self.conrad.floor = if step < 0 { FLOORS.len() - 1 } else { 0 };
            }
        }
    }

    fn current_room(&mut self) -> Option<&Room> {
        if self.room_cache.as_ref().map(|(r, _)| *r) != Some(self.room) {
            let room = self.level.map.room(self.room)?;
            self.room_cache = Some((self.room, room));
        }
        self.room_cache.as_ref().map(|(_, r)| r)
    }

    /// Draws the current room and Conrad into `screen` and sets up `palette` for it.
    pub fn render(&mut self, screen: &mut Bitmap, palette: &mut Palette) {
        let Some(room) = self.current_room() else {
            screen.pixels.fill(0);
            return;
        };
        screen.pixels.copy_from_slice(&room.bitmap.pixels);
        let slots = room.slots;
        palette.set_slot(0x00, &self.level.palettes.get(slots[0] as usize));
        palette.set_slot(0x80, &self.level.palettes.get(slots[1] as usize));
        palette.set_slot(CONRAD_BASE as usize, &self.conrad_palette);

        let c = &self.conrad;
        let id = if c.walking {
            let n = (WALK_FRAMES.end() - WALK_FRAMES.start() + 1) as u32;
            WALK_FRAMES.start() + ((c.anim_ticks / WALK_FRAME_TICKS) % n) as u16
        } else {
            STAND_FRAME
        };
        if let Some(f) = self.perso.frame(id) {
            let x = c.x - f.w as i32 / 2;
            let y = FLOORS[c.floor] - f.h as i32;
            screen.blit(&f, x, y, CONRAD_BASE, c.facing_left);
        }
    }
}

/// Grey stand-in colours when `fb.exe` is not next to the data files.
fn placeholder_conrad_palette() -> [Rgb; 16] {
    std::array::from_fn(|i| {
        let v = (i as u32 * 255 / 15) as u8;
        Rgb(v, v, v.saturating_sub(24))
    })
}
