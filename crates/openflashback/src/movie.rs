//! Playing cutscenes in the window: a queue of `.cmd` / `.pol` scripts shown one after another.

use of_engine::cutscene::{Cmd, Player, Pol};
use of_engine::screen::Bitmap;
use of_engine::{DataDir, Palette};
use std::collections::VecDeque;
use std::time::Duration;

/// Cutscene frames per second. Not yet measured against the original.
pub const CUTSCENE_FPS: u32 = 12;

/// Cutscenes shown at startup, before the title screen: the publishers' logos.
pub const STARTUP: [(&str, usize); 1] = [("logos", 0)];

/// Cutscenes shown when a new game starts: the escape and the crash on Titan, before level 1.
pub const NEW_GAME: [(&str, usize); 2] = [("intro1", 0), ("intro2", 0)];

/// A queue of cutscenes being played.
pub struct Movie {
    queue: VecDeque<Player>,
    /// Frames left to show the current picture.
    hold: u32,
    accum: Duration,
}

impl Movie {
    /// Loads `scenes` (name and sub-script); scenes whose files are missing are left out.
    pub fn load(data: &DataDir, scenes: &[(&str, usize)]) -> Movie {
        let queue = scenes
            .iter()
            .filter_map(|&(name, sub)| {
                let pol = Pol::parse(data.read(&format!("{name}.pol")).ok()?)?;
                let cmd = Cmd::parse(data.read(&format!("{name}.cmd")).ok()?);
                Some(Player::new(pol, cmd.ops(sub)))
            })
            .collect();
        Movie {
            queue,
            hold: 0,
            accum: Duration::ZERO,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.queue.is_empty()
    }

    /// Advances by `dt` of real time.
    pub fn update(&mut self, dt: Duration) {
        let frame = Duration::from_secs(1) / CUTSCENE_FPS;
        self.accum = (self.accum + dt).min(Duration::from_millis(250));
        while self.accum >= frame && !self.queue.is_empty() {
            self.accum -= frame;
            if self.hold > 0 {
                self.hold -= 1;
                continue;
            }
            match self.queue.front_mut().and_then(|p| p.next_frame()) {
                Some(n) => self.hold = n.saturating_sub(1),
                None => {
                    self.queue.pop_front();
                }
            }
        }
    }

    /// Copies the current picture and its palette.
    pub fn render(&self, screen: &mut Bitmap, palette: &mut Palette) {
        if let Some(p) = self.queue.front() {
            screen.pixels.copy_from_slice(&p.screen.pixels);
            *palette = p.full_palette();
        }
    }
}
