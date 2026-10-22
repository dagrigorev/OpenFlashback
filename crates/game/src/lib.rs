//! Flashback game state: loading a level, moving Conrad with his original animations through the
//! rooms, and composing the screen.

pub mod conrad;
pub mod world;

pub use conrad::{Conrad, Input};

use of_engine::anim::AnimTable;
use of_engine::level::{ConnectionTable, LevelMap, Room, LEVEL_FILES, SCRIPT_FILES};
use of_engine::palette::{conrad_palette_from_exe, LevelPalettes};
use of_engine::pge::parse_pge;
use of_engine::screen::Bitmap;
use of_engine::sprite::SpriteBank;
use of_engine::{DataDir, Palette, Rgb};
use std::io;
use world::World;

/// Palette entries reserved for Conrad's 16 colours.
pub const CONRAD_BASE: u8 = 0x40;

/// Game updates per second; every update shows the next animation frame.
///
/// Not yet measured against the original.
pub const TICK_HZ: u32 = 30;

/// A loaded level.
pub struct Level {
    pub index: usize,
    pub map: LevelMap,
    pub palettes: LevelPalettes,
    pub ct: ConnectionTable,
    pub anims: AnimTable,
}

/// Where and how Conrad starts a level.
#[derive(Debug, Clone, Copy)]
pub struct Start {
    pub room: usize,
    pub x: i32,
    pub y: i32,
    pub anim: u16,
}

impl Level {
    /// Loads level `index` (0-based) from `data`, returning it with Conrad's start.
    pub fn load(data: &DataDir, index: usize) -> io::Result<(Level, Start)> {
        let index = index.min(LEVEL_FILES.len() - 1);
        let (name, script) = (LEVEL_FILES[index], SCRIPT_FILES[index]);
        let map = LevelMap::new(data.read(&format!("{name}.map"))?);
        let palettes = LevelPalettes::parse(&data.read(&format!("{name}.pal"))?);
        let ct = ConnectionTable::parse(&data.read(&format!("{name}.ct"))?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{name}.ct: {e}")))?;
        let anims = AnimTable::parse(data.read(&format!("{script}.ani"))?);
        let pge = parse_pge(&data.read(&format!("{script}.pge"))?);
        let start = pge
            .first()
            .map(|p| Start {
                room: p.room as usize,
                x: p.x as i32,
                y: p.y as i32,
                anim: p.anim,
            })
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("{script}.pge is empty")))?;
        Ok((
            Level {
                index,
                map,
                palettes,
                ct,
                anims,
            },
            start,
        ))
    }
}

/// The running game.
pub struct Game {
    pub level: Level,
    room_cache: Option<(usize, Room)>,
    pub conrad: Conrad,
    perso: SpriteBank,
    conrad_palette: [Rgb; 16],
}

impl Game {
    /// Starts level `index` (0-based).
    pub fn new(data: &DataDir, index: usize) -> io::Result<Game> {
        let (level, start) = Level::load(data, index)?;
        let perso = SpriteBank::parse(data.read("perso.spr")?, &data.read("perso.off")?);
        let conrad = Conrad::new(start.room, start.x, start.y, start.anim, &level.anims);
        Ok(Game {
            level,
            room_cache: None,
            conrad,
            perso,
            conrad_palette: data
                .read("fb.exe")
                .ok()
                .and_then(|exe| conrad_palette_from_exe(&exe))
                .unwrap_or_else(placeholder_conrad_palette),
        })
    }

    /// The room on screen: the one Conrad is in.
    pub fn room(&self) -> usize {
        self.conrad.room
    }

    /// Moves Conrad to `room` if the level has it, keeping his place on screen.
    pub fn goto_room(&mut self, room: usize) -> bool {
        if self.level.map.room(room).is_none() {
            return false;
        }
        self.conrad.room = room;
        true
    }

    /// Advances one tick.
    pub fn tick(&mut self, input: Input) {
        let world = World { ct: &self.level.ct };
        self.conrad.tick(input, &self.level.anims, &world);
    }

    fn current_room(&mut self) -> Option<&Room> {
        let room = self.conrad.room;
        if self.room_cache.as_ref().map(|(r, _)| *r) != Some(room) {
            self.room_cache = Some((room, self.level.map.room(room)?));
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
        if let Some((id, mirror)) = c.sprite() {
            if let Some(s) = self.perso.frame(id) {
                let (x, y) = s.origin(c.x, c.y, mirror);
                // Room pixels of the foreground layer (bit 7) stay in front of Conrad.
                screen.blit_masked(&s.bitmap, x, y, CONRAD_BASE, mirror, 0x80);
            }
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
