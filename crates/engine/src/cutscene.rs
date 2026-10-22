//! Cutscenes: polygon pictures (`.pol`) animated by command scripts (`.cmd`).
//!
//! A `.pol` file starts with five big-endian `u32` offsets: the shape offset table, the 16-colour
//! palettes (`0RGB` words), the vertex offset table, the shape data and the vertex data. A shape is
//! a list of primitives (polygon, ellipse or point), each a vertex set, an optional offset and a
//! colour.
//!
//! A `.cmd` file is a `u16` count of sub-scripts with their offsets (all big-endian), then byte
//! code: an opcode byte (the opcode times 4, or `0x80` and up to end) and its arguments.

use crate::palette::Rgb;
use crate::screen::Bitmap;
use crate::SCREEN_W;

/// Palette entries used by cutscenes: two banks of 16 colours.
pub const CUTSCENE_BASE: u8 = 0xc0;

/// The polygon pictures of a cutscene.
#[derive(Debug, Clone)]
pub struct Pol {
    data: Vec<u8>,
    shape_table: usize,
    palettes: usize,
    vertex_table: usize,
    shape_data: usize,
    vertex_data: usize,
}

impl Pol {
    pub fn parse(data: Vec<u8>) -> Option<Pol> {
        let off = |i: usize| -> Option<usize> {
            let b = data.get(i * 4..i * 4 + 4)?;
            Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize)
        };
        Some(Pol {
            shape_table: off(0)?,
            palettes: off(1)?,
            vertex_table: off(2)?,
            shape_data: off(3)?,
            vertex_data: off(4)?,
            data,
        })
    }

    fn u16(&self, p: usize) -> u16 {
        self.data.get(p..p + 2).map_or(0, |b| u16::from_be_bytes([b[0], b[1]]))
    }

    fn i16(&self, p: usize) -> i32 {
        self.u16(p) as i16 as i32
    }

    fn i8(&self, p: usize) -> i32 {
        self.data.get(p).map_or(0, |&b| b as i8 as i32)
    }

    /// Number of shapes: the shape offset table runs up to the palettes.
    pub fn shape_count(&self) -> usize {
        self.palettes.saturating_sub(self.shape_table) / 2
    }

    /// Palette `n`.
    pub fn palette(&self, n: usize) -> [Rgb; 16] {
        let p = self.palettes + n * 32;
        std::array::from_fn(|i| Rgb::from_12bit(self.u16(p + i * 2)))
    }

    /// The primitives of shape `n`, positioned at (`x`, `y`).
    pub fn shape(&self, n: usize, x: i32, y: i32) -> Vec<Primitive> {
        let mut p = self.shape_data + self.u16(self.shape_table + n * 2) as usize;
        let count = self.u16(p);
        p += 2;
        let mut out = Vec::new();
        for _ in 0..count {
            let v = self.u16(p) as usize;
            p += 2;
            let (mut dx, mut dy) = (0, 0);
            if v & 0x8000 != 0 {
                dx = self.i16(p);
                dy = self.i16(p + 2);
                p += 4;
            }
            let colour = self.data.get(p).copied().unwrap_or(0);
            p += 1;
            let vp = self.vertex_data + self.u16(self.vertex_table + (v & 0x3fff) * 2) as usize;
            out.push(self.primitive(vp, x + dx, y + dy, colour & 15));
        }
        out
    }

    fn primitive(&self, mut p: usize, x: i32, y: i32, colour: u8) -> Primitive {
        let n = self.data.get(p).copied().unwrap_or(0);
        p += 1;
        let cx = self.i16(p) + x;
        let cy = self.i16(p + 2) + y;
        if n & 0x80 != 0 {
            let (rx, ry) = (self.i16(p + 4), self.i16(p + 6));
            return Primitive::Ellipse { cx, cy, rx, ry, colour };
        }
        if n == 0 {
            return Primitive::Point { x: cx, y: cy, colour };
        }
        let (mut ix, mut iy) = (cx, cy);
        let mut points = vec![(ix, iy)];
        p += 4;
        for k in 0..n as usize {
            ix += self.i8(p + k * 2);
            iy += self.i8(p + k * 2 + 1);
            points.push((ix, iy));
        }
        Primitive::Polygon { points, colour }
    }
}

/// A drawable part of a shape. Colours are 0 to 15 within a palette bank.
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Polygon {
        points: Vec<(i32, i32)>,
        colour: u8,
    },
    Ellipse {
        cx: i32,
        cy: i32,
        rx: i32,
        ry: i32,
        colour: u8,
    },
    Point {
        x: i32,
        y: i32,
        colour: u8,
    },
}

impl Primitive {
    /// Moves every point by `f`, used for scaling and rotation.
    fn map(&self, f: &dyn Fn(i32, i32) -> (i32, i32), scale: f64) -> Primitive {
        match self {
            Primitive::Polygon { points, colour } => Primitive::Polygon {
                points: points.iter().map(|&(x, y)| f(x, y)).collect(),
                colour: *colour,
            },
            Primitive::Ellipse { cx, cy, rx, ry, colour } => {
                let (cx, cy) = f(*cx, *cy);
                let s = |r: i32| (r as f64 * scale).round() as i32;
                Primitive::Ellipse {
                    cx,
                    cy,
                    rx: s(*rx),
                    ry: s(*ry),
                    colour: *colour,
                }
            }
            Primitive::Point { x, y, colour } => {
                let (x, y) = f(*x, *y);
                Primitive::Point { x, y, colour: *colour }
            }
        }
    }

    /// Fills the primitive into `dst` with palette entry `base + colour`, inside `clip`.
    pub fn draw(&self, dst: &mut Bitmap, base: u8, clip: Rect) {
        let mut span = |y: i32, x0: i32, x1: i32, c: u8| {
            if y < clip.y || y >= clip.y + clip.h {
                return;
            }
            let (a, b) = (x0.max(clip.x), x1.min(clip.x + clip.w - 1));
            for x in a..=b {
                dst.pixels[y as usize * dst.w + x as usize] = base + c;
            }
        };
        match self {
            Primitive::Point { x, y, colour } => span(*y, *x, *x, *colour),
            Primitive::Ellipse { cx, cy, rx, ry, colour } => {
                let (rx, ry) = (rx.abs().max(0), ry.abs().max(0));
                for dy in -ry..=ry {
                    let w = if ry == 0 {
                        rx
                    } else {
                        (rx as f64 * (1.0 - (dy as f64 / ry as f64).powi(2)).max(0.0).sqrt()).round() as i32
                    };
                    span(cy + dy, cx - w, cx + w, *colour);
                }
            }
            Primitive::Polygon { points, colour } => fill_polygon(points, |y, a, b| span(y, a, b, *colour)),
        }
    }
}

/// Even-odd scanline fill, sampling pixel centres; thin polygons still cover their outline.
fn fill_polygon(points: &[(i32, i32)], mut span: impl FnMut(i32, i32, i32)) {
    if points.is_empty() {
        return;
    }
    let ymin = points.iter().map(|p| p.1).min().unwrap_or(0);
    let ymax = points.iter().map(|p| p.1).max().unwrap_or(0);
    for y in ymin..=ymax {
        let yc = y as f64 + 0.5;
        let mut xs: Vec<f64> = Vec::new();
        for i in 0..points.len() {
            let (x0, y0) = points[i];
            let (x1, y1) = points[(i + 1) % points.len()];
            let (fy0, fy1) = (y0 as f64, y1 as f64);
            if (fy0 <= yc && fy1 > yc) || (fy1 <= yc && fy0 > yc) {
                xs.push(x0 as f64 + (yc - fy0) * (x1 - x0) as f64 / (fy1 - fy0));
            }
        }
        xs.sort_by(|a, b| a.total_cmp(b));
        let mut drew = false;
        for pair in xs.as_chunks::<2>().0 {
            span(
                y,
                pair[0].round() as i32,
                (pair[1].round() as i32 - 1).max(pair[0].round() as i32),
            );
            drew = true;
        }
        if !drew && ymin == ymax {
            // A flat polygon (a horizontal line) has no crossings: draw its points directly.
            let row: Vec<i32> = points.iter().filter(|p| p.1 == y).map(|p| p.0).collect();
            if let (Some(&a), Some(&b)) = (row.iter().min(), row.iter().max()) {
                span(y, a, b);
            }
        }
    }
}

fn fill_rect(b: &mut Bitmap, r: Rect, c: u8) {
    for y in r.y.max(0)..(r.y + r.h).min(b.h as i32) {
        let row = &mut b.pixels[y as usize * b.w..][..b.w];
        row[r.x.max(0) as usize..(r.x + r.w).min(b.w as i32) as usize].fill(c);
    }
}

/// A clipping rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// The cutscene window: 240 x 128 pixels centred on the game screen. Shape coordinates are
/// relative to its top-left corner.
pub const WINDOW: Rect = Rect {
    x: (SCREEN_W as i32 - 240) / 2,
    y: 50,
    w: 240,
    h: 128,
};

/// One decoded script instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Shows the picture drawn so far as the next frame.
    Frame,
    /// Sets how frames start: 0 keeps the saved background, other values clear the screen.
    Refresh(u8),
    /// Holds the current picture for this many frames.
    Wait(u8),
    Shape {
        shape: u16,
        x: i32,
        y: i32,
    },
    /// Loads `.pol` palette `palette` into bank `bank` (0 or 1).
    Palette {
        palette: u8,
        bank: u8,
    },
    /// Shows caption text number `n` (from `fr_cine.txt`).
    Caption(u16),
    Nop,
    RefreshAll,
    ShapeScaled {
        shape: u16,
        x: i32,
        y: i32,
        zoom: i32,
        ox: u8,
        oy: u8,
    },
    ShapeRotated {
        shape: u16,
        x: i32,
        y: i32,
        zoom: i32,
        ox: u8,
        oy: u8,
        angles: [i32; 3],
    },
    /// Saves the screen as the background later frames start from.
    SaveBackground,
    Text {
        n: u16,
        x: u8,
        y: u8,
    },
    /// The interactive menu part of a script; playback stops here.
    Keys,
}

/// A cutscene script file.
#[derive(Debug, Clone)]
pub struct Cmd {
    data: Vec<u8>,
    starts: Vec<usize>,
}

impl Cmd {
    pub fn parse(data: Vec<u8>) -> Cmd {
        let w = |p: usize| {
            data.get(p..p + 2)
                .map_or(0, |b| u16::from_be_bytes([b[0], b[1]]) as usize)
        };
        let n = w(0);
        let starts = if n == 0 {
            vec![2]
        } else {
            (0..n).map(|i| 2 + n * 2 + w(2 + i * 2)).collect()
        };
        Cmd { data, starts }
    }

    /// Number of sub-scripts.
    pub fn len(&self) -> usize {
        self.starts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.starts.is_empty()
    }

    /// The instructions of sub-script `n`, up to its end.
    pub fn ops(&self, n: usize) -> Vec<Op> {
        let d = &self.data;
        let Some(&start) = self.starts.get(n) else {
            return Vec::new();
        };
        let mut p = start;
        let byte = |p: &mut usize| -> Option<u8> {
            let b = *d.get(*p)?;
            *p += 1;
            Some(b)
        };
        let word = |p: &mut usize| -> Option<u16> {
            let b = d.get(*p..*p + 2)?;
            *p += 2;
            Some(u16::from_be_bytes([b[0], b[1]]))
        };
        let mut ops = Vec::new();
        let decode = |p: &mut usize| -> Option<Op> {
            let b = byte(p)?;
            if b & 0x80 != 0 || b & 3 != 0 {
                return None;
            }
            let pos = |p: &mut usize, s: u16| -> Option<(i32, i32)> {
                if s & 0x8000 != 0 {
                    Some((word(p)? as i16 as i32, word(p)? as i16 as i32))
                } else {
                    Some((0, 0))
                }
            };
            Some(match b >> 2 {
                0 | 5 => Op::Frame,
                1 => Op::Refresh(byte(p)?),
                2 => Op::Wait(byte(p)?),
                3 => {
                    let s = word(p)?;
                    let (x, y) = pos(p, s)?;
                    Op::Shape { shape: s & 0x7ff, x, y }
                }
                4 => Op::Palette {
                    palette: byte(p)?,
                    bank: byte(p)?,
                },
                6 => Op::Caption(word(p)?),
                7 => Op::Nop,
                8 => {
                    *p += 3;
                    Op::Nop
                }
                9 => Op::RefreshAll,
                10 => {
                    let s = word(p)?;
                    let (x, y) = pos(p, s)?;
                    let zoom = word(p)? as i16 as i32 + 512;
                    Op::ShapeScaled {
                        shape: s & 0x7ff,
                        x,
                        y,
                        zoom,
                        ox: byte(p)?,
                        oy: byte(p)?,
                    }
                }
                11 => {
                    let s = word(p)?;
                    let (x, y) = pos(p, s)?;
                    let zoom = 512 + if s & 0x4000 != 0 { word(p)? as i16 as i32 } else { 0 };
                    let (ox, oy) = (byte(p)?, byte(p)?);
                    let a = word(p)? as i32;
                    let b = if s & 0x2000 != 0 { word(p)? as i32 } else { 180 };
                    let c = if s & 0x1000 != 0 { word(p)? as i32 } else { 90 };
                    Op::ShapeRotated {
                        shape: s & 0x7ff,
                        x,
                        y,
                        zoom,
                        ox,
                        oy,
                        angles: [a, b, c],
                    }
                }
                12 => Op::SaveBackground,
                13 => {
                    let n = word(p)?;
                    let (x, y) = if n != 0xffff { (byte(p)?, byte(p)?) } else { (0, 0) };
                    Op::Text { n, x, y }
                }
                14 => Op::Keys,
                _ => return None,
            })
        };
        while let Some(op) = decode(&mut p) {
            let stop = op == Op::Keys;
            ops.push(op);
            if stop {
                break;
            }
        }
        ops
    }
}

/// Plays a cutscene into a sequence of frames.
///
/// Pictures are built from two layers. While the refresh mode is non-zero, shapes are drawn into
/// the background layer with palette bank 0, and setting such a mode clears it. While the mode is
/// 0, shapes are drawn into the front layer with bank 1; it is cleared after every frame. A frame
/// shows the front layer over the background.
pub struct Player {
    pol: Pol,
    ops: Vec<Op>,
    pc: usize,
    /// The picture of the last frame.
    pub screen: Bitmap,
    background: Bitmap,
    front: Bitmap,
    refresh: u8,
    pub palette: [[Rgb; 16]; 2],
    clip: Rect,
    ended: bool,
}

impl Player {
    pub fn new(pol: Pol, ops: Vec<Op>) -> Player {
        let mut background = Bitmap::screen();
        fill_rect(&mut background, WINDOW, CUTSCENE_BASE);
        Player {
            pol,
            ops,
            pc: 0,
            screen: background.clone(),
            background,
            front: Bitmap::screen(),
            refresh: 1,
            palette: [[Rgb::default(); 16]; 2],
            clip: WINDOW,
            ended: false,
        }
    }

    /// Runs until the next frame is complete and composes it into `screen`; returns how many
    /// frames to show it for, or `None` at the end.
    pub fn next_frame(&mut self) -> Option<u32> {
        if self.ended {
            return None;
        }
        let mut hold = 1;
        while let Some(op) = self.ops.get(self.pc).cloned() {
            self.pc += 1;
            match op {
                Op::Frame => {
                    self.compose();
                    return Some(hold);
                }
                Op::Refresh(m) => {
                    self.refresh = m;
                    if m != 0 {
                        fill_rect(&mut self.background, self.clip, CUTSCENE_BASE);
                    }
                }
                Op::Wait(n) => hold += n as u32,
                Op::Shape { shape, x, y } => self.draw(shape, &|px, py| (px + x, py + y), 1.0),
                Op::ShapeScaled {
                    shape,
                    x,
                    y,
                    zoom,
                    ox,
                    oy,
                } => {
                    let z = zoom as f64 / 512.0;
                    let (ox, oy) = (ox as f64, oy as f64);
                    let f = move |px: i32, py: i32| {
                        (
                            (x as f64 + ox + (px as f64 - ox) * z).round() as i32,
                            (y as f64 + oy + (py as f64 - oy) * z).round() as i32,
                        )
                    };
                    self.draw(shape, &f, z);
                }
                Op::ShapeRotated {
                    shape,
                    x,
                    y,
                    zoom,
                    ox,
                    oy,
                    angles,
                } => {
                    let z = zoom as f64 / 512.0;
                    let (s, c) = (angles[0] as f64).to_radians().sin_cos();
                    let (ox, oy) = (ox as f64, oy as f64);
                    let f = move |px: i32, py: i32| {
                        let (dx, dy) = (px as f64 - ox, py as f64 - oy);
                        (
                            (x as f64 + ox + (dx * c - dy * s) * z).round() as i32,
                            (y as f64 + oy + (dx * s + dy * c) * z).round() as i32,
                        )
                    };
                    self.draw(shape, &f, z);
                }
                Op::Palette { palette, bank } => {
                    self.palette[(bank & 1) as usize] = self.pol.palette(palette as usize);
                }
                Op::SaveBackground | Op::RefreshAll => {
                    self.compose();
                    self.background.pixels.copy_from_slice(&self.screen.pixels);
                }
                Op::Caption(_) | Op::Text { .. } | Op::Nop => {}
                Op::Keys => break,
            }
        }
        // The script ended: show what is left once more.
        self.ended = true;
        self.compose();
        Some(hold)
    }

    /// Puts the front layer over the background into `screen` and clears the front layer.
    fn compose(&mut self) {
        for ((d, &b), f) in self
            .screen
            .pixels
            .iter_mut()
            .zip(&self.background.pixels)
            .zip(&mut self.front.pixels)
        {
            *d = if *f != 0 { *f } else { b };
            *f = 0;
        }
    }

    fn draw(&mut self, shape: u16, f: &dyn Fn(i32, i32) -> (i32, i32), scale: f64) {
        let (layer, base) = if self.refresh == 0 {
            (&mut self.front, CUTSCENE_BASE + 16)
        } else {
            (&mut self.background, CUTSCENE_BASE)
        };
        let (wx, wy) = (self.clip.x, self.clip.y);
        let place = |px: i32, py: i32| {
            let (x, y) = f(px, py);
            (x + wx, y + wy)
        };
        for prim in self.pol.shape(shape as usize, 0, 0) {
            prim.map(&place, scale).draw(layer, base, self.clip);
        }
    }

    /// The 256-colour palette for the current frame (only the cutscene entries are set).
    pub fn full_palette(&self) -> crate::Palette {
        let mut p = crate::Palette::default();
        p.set_slot(CUTSCENE_BASE as usize, &self.palette[0]);
        p.set_slot(CUTSCENE_BASE as usize + 16, &self.palette[1]);
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_bytes_decode_to_ops() {
        // One sub-script: refresh 1, shape 5 at (2, -3), palette 7 to bank 1, frame, end.
        let mut d = vec![0, 1, 0, 0];
        d.extend_from_slice(&[0x04, 1, 0x0c, 0x80, 5, 0, 2, 0xff, 0xfd, 0x10, 7, 1, 0x00, 0xff]);
        let ops = Cmd::parse(d).ops(0);
        assert_eq!(
            ops,
            vec![
                Op::Refresh(1),
                Op::Shape { shape: 5, x: 2, y: -3 },
                Op::Palette { palette: 7, bank: 1 },
                Op::Frame
            ]
        );
    }

    #[test]
    fn polygon_fill_covers_a_square() {
        let mut b = Bitmap::new(8, 8);
        let sq = Primitive::Polygon {
            points: vec![(1, 1), (5, 1), (5, 5), (1, 5)],
            colour: 2,
        };
        sq.draw(&mut b, 0xc0, Rect { x: 0, y: 0, w: 8, h: 8 });
        let filled = b.pixels.iter().filter(|&&c| c == 0xc2).count();
        assert_eq!(filled, 16);
    }
}
