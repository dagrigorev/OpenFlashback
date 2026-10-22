//! Developer tools: render the original data to PNG files for inspection and comparison.

use of_engine::level::{ConnectionTable, Dir, LevelMap, LEVEL_FILES, SCRIPT_FILES};
use of_engine::obj::{input_condition, ObjectScripts};
use of_engine::palette::LevelPalettes;
use of_engine::screen::{decode_planar_screen, Bitmap};
use of_engine::sprite::SpriteBank;
use of_engine::{DataDir, Palette, Rgb, SCREEN_H, SCREEN_W};
use of_game::{Game, Input};
use std::fs::File;
use std::io::BufWriter;

pub const HELP: &str = "  screen DATA NAME OUT.png        full-screen picture (menu1, menu2, menu3, instru_f)\n  \
     rooms DATA LEVEL OUT.png        contact sheet of every room of level 1-5\n  \
     shot DATA LEVEL ROOM OUT.png    one room with Conrad, as the game draws it\n  \
     sprites DATA BANK OUT.png       sheet of a sprite bank (perso, junky, mercenai, replican, glue)\n  \
     ct DATA LEVEL                   print the room connections of a level\n  \
     film DATA LEVEL INPUT EVERY OUT.png\n                                  \
     play scripted input, keep every EVERY-th tick in a contact sheet. INPUT is\n                                  \
     space-separated KEYS*TICKS, KEYS from l r u d s (run) or n (none): \"r*40 ru*1 n*30\"
       obj DATA LEVEL NODE [STATE]     print the script transitions of an object node (Conrad: 1)";

const TOOLS: [&str; 7] = ["screen", "rooms", "shot", "sprites", "ct", "film", "obj"];

/// Parses `film` input such as `r*40 ru*1 n*30` into one [`Input`] per tick.
fn parse_script(script: &str) -> Result<Vec<Input>, String> {
    let mut ticks = Vec::new();
    for tok in script.split_whitespace() {
        let (keys, n) = tok.split_once('*').unwrap_or((tok, "1"));
        let n: usize = n.parse().map_err(|_| format!("bad tick count in {tok}"))?;
        let mut i = Input::default();
        for k in keys.chars() {
            match k {
                'l' => i.left = true,
                'r' => i.right = true,
                'u' => i.up = true,
                'd' => i.down = true,
                's' => i.run = true,
                'n' => {}
                _ => return Err(format!("unknown key {k} in {tok}")),
            }
        }
        ticks.extend(std::iter::repeat_n(i, n));
    }
    Ok(ticks)
}

pub fn is_tool(name: &str) -> bool {
    TOOLS.contains(&name)
}

fn arg<'a>(args: &'a [String], i: usize, what: &str) -> Result<&'a str, String> {
    args.get(i)
        .map(String::as_str)
        .ok_or_else(|| format!("missing {what}\n{HELP}"))
}

fn level_arg(args: &[String], i: usize) -> Result<usize, String> {
    let n: usize = arg(args, i, "LEVEL")?
        .parse()
        .map_err(|_| "LEVEL must be 1-5".to_string())?;
    if !(1..=LEVEL_FILES.len()).contains(&n) {
        return Err("LEVEL must be 1-5".into());
    }
    Ok(n - 1)
}

pub fn run(args: &[String]) -> Result<(), String> {
    let data = DataDir::open(arg(args, 1, "DATA")?).map_err(|e| e.to_string())?;
    let read = |name: &str| data.read(name).map_err(|e| e.to_string());
    match args[0].as_str() {
        "screen" => {
            let name = arg(args, 2, "NAME")?;
            let bmp = decode_planar_screen(&read(&format!("{name}.map"))?);
            let pal = Palette::from_rgb8(&read(&format!("{name}.pal"))?);
            write_png(arg(args, 3, "OUT")?, &bmp, &pal.0, 3)
        }
        "rooms" => {
            let name = LEVEL_FILES[level_arg(args, 2)?];
            let map = LevelMap::new(read(&format!("{name}.map"))?);
            let pals = LevelPalettes::parse(&read(&format!("{name}.pal"))?);
            let rooms = map.rooms();
            let cols = 8;
            let rows = rooms.len().div_ceil(cols);
            let mut sheet = vec![Rgb::default(); cols * SCREEN_W * rows * SCREEN_H];
            for (k, &r) in rooms.iter().enumerate() {
                let room = map.room(r).ok_or(format!("room {r} does not decode"))?;
                let (ox, oy) = ((k % cols) * SCREEN_W, (k / cols) * SCREEN_H);
                for (i, &c) in room.bitmap.pixels.iter().enumerate() {
                    let slot = room.slots[(c >> 7) as usize] as usize;
                    sheet[(oy + i / SCREEN_W) * cols * SCREEN_W + ox + i % SCREEN_W] =
                        pals.get(slot)[(c & 15) as usize];
                }
            }
            println!("{} rooms: {rooms:?}", rooms.len());
            write_rgb_png(arg(args, 3, "OUT")?, cols * SCREEN_W, rows * SCREEN_H, &sheet)
        }
        "shot" => {
            let level = level_arg(args, 2)?;
            let room: usize = arg(args, 3, "ROOM")?.parse().map_err(|_| "ROOM must be a number")?;
            let mut game = Game::new(&data, level).map_err(|e| e.to_string())?;
            if !game.goto_room(room) {
                return Err(format!("level {} has no room {room}", level + 1));
            }
            let mut screen = Bitmap::screen();
            let mut pal = Palette::default();
            game.render(&mut screen, &mut pal);
            write_png(arg(args, 4, "OUT")?, &screen, &pal.0, 3)
        }
        "sprites" => {
            let bank = arg(args, 2, "BANK")?;
            let spr = SpriteBank::parse(read(&format!("{bank}.spr"))?, &read(&format!("{bank}.off"))?);
            let ids = spr.ids();
            let (cell_w, cell_h, cols) = (64, 80, 16);
            let mut sheet = Bitmap::new(cell_w * cols, cell_h * ids.len().div_ceil(cols));
            for (k, &id) in ids.iter().enumerate() {
                if let Some(f) = spr.frame(id) {
                    let (x, y) = ((k % cols) * cell_w, (k / cols) * cell_h);
                    sheet.blit(&f.bitmap, x as i32, y as i32, 0, false);
                }
            }
            let grey: Vec<Rgb> = (0..256)
                .map(|i| if i == 0 { Rgb(255, 0, 255) } else { gray(i) })
                .collect();
            println!("{} frames", ids.len());
            write_png(arg(args, 3, "OUT")?, &sheet, &grey, 2)
        }
        "ct" => {
            let name = LEVEL_FILES[level_arg(args, 2)?];
            let map = LevelMap::new(read(&format!("{name}.map"))?);
            let ct = ConnectionTable::parse(&read(&format!("{name}.ct"))?).map_err(|e| e.to_string())?;
            for r in map.rooms() {
                let n = |d| ct.neighbour(r, d).map_or("-".to_string(), |v| v.to_string());
                println!(
                    "room {r:2}: up {:>2} down {:>2} left {:>2} right {:>2}",
                    n(Dir::Up),
                    n(Dir::Down),
                    n(Dir::Left),
                    n(Dir::Right)
                );
            }
            Ok(())
        }
        "film" => {
            let level = level_arg(args, 2)?;
            let ticks = parse_script(arg(args, 3, "INPUT")?)?;
            let every: usize = arg(args, 4, "EVERY")?.parse().map_err(|_| "EVERY must be a number")?;
            let every = every.max(1);
            let mut game = Game::new(&data, level).map_err(|e| e.to_string())?;
            let shots = ticks.len() / every + 1;
            let cols = 6;
            let rows = shots.div_ceil(cols);
            let mut sheet = vec![Rgb::default(); cols * SCREEN_W * rows * SCREEN_H];
            let mut screen = Bitmap::screen();
            let mut pal = Palette::default();
            for k in 0..=ticks.len() {
                if k % every == 0 {
                    game.render(&mut screen, &mut pal);
                    let n = k / every;
                    let (ox, oy) = ((n % cols) * SCREEN_W, (n / cols) * SCREEN_H);
                    for (i, &c) in screen.pixels.iter().enumerate() {
                        sheet[(oy + i / SCREEN_W) * cols * SCREEN_W + ox + i % SCREEN_W] = pal.0[c as usize];
                    }
                    let c = &game.conrad;
                    println!(
                        "tick {k:4}: room {:2} x {:4} y {:4} anim {:3}",
                        c.room,
                        c.x,
                        c.y,
                        c.anim()
                    );
                }
                if let Some(&i) = ticks.get(k) {
                    game.tick(i);
                }
            }
            write_rgb_png(arg(args, 5, "OUT")?, cols * SCREEN_W, rows * SCREEN_H, &sheet)
        }
        "obj" => {
            let script = SCRIPT_FILES[level_arg(args, 2)?];
            let node: usize = arg(args, 3, "NODE")?.parse().map_err(|_| "NODE must be a number")?;
            let state: Option<u16> = args.get(4).and_then(|s| s.parse().ok());
            let scripts = ObjectScripts::parse(&read(&format!("{script}.obj"))?);
            let list = scripts.node(node).ok_or(format!("{script}.obj has no node {node}"))?;
            println!("{script}.obj node {node}: {} transitions", list.len());
            for (i, t) in list.iter().enumerate() {
                if state.is_some_and(|s| s != t.state) {
                    continue;
                }
                let cond = |op: u8| match input_condition(op) {
                    Some(c) => {
                        let dir = [(c.up, "up"), (c.down, "down"), (c.forward, "forward"), (c.back, "back")]
                            .iter()
                            .find(|d| d.0)
                            .map_or("", |d| d.1);
                        format!("{dir}{}", if c.modifier { "+mod" } else { "" })
                    }
                    None if op == 0 => "-".to_string(),
                    None => format!("op{op:#04x}"),
                };
                println!(
                    "#{i:4} {:3} -> {:3} [next #{:4}]  if {:>10} {:>10}  flags {:#04x}  do {:>6}  args {:?}  move ({}, {})",
                    t.state,
                    t.next_state,
                    t.next_record,
                    cond(t.cond1),
                    cond(t.cond2),
                    t.flags,
                    if t.action == 0 { "-".to_string() } else { format!("{:#04x}", t.action) },
                    t.args,
                    t.dx,
                    t.dy
                );
            }
            Ok(())
        }
        other => Err(format!("unknown tool {other}")),
    }
}

fn gray(i: usize) -> Rgb {
    let v = ((i & 15) * 17) as u8;
    Rgb(v, v, v)
}

/// Writes an indexed bitmap as RGB, each pixel enlarged to `scale` x `scale`.
fn write_png(path: &str, bmp: &Bitmap, pal: &[Rgb], scale: usize) -> Result<(), String> {
    let (w, h) = (bmp.w * scale, bmp.h * scale);
    let mut rgb = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            rgb.push(pal[bmp.pixels[(y / scale) * bmp.w + x / scale] as usize]);
        }
    }
    write_rgb_png(path, w, h, &rgb)
}

fn write_rgb_png(path: &str, w: usize, h: usize, px: &[Rgb]) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("{path}: {e}"))?;
    let mut enc = png::Encoder::new(BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    let bytes: Vec<u8> = px.iter().flat_map(|c| [c.0, c.1, c.2]).collect();
    wr.write_image_data(&bytes).map_err(|e| e.to_string())?;
    println!("wrote {path} ({w}x{h})");
    Ok(())
}
