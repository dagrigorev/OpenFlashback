//! The game window: title screen, play, and scaling the 256 x 224 screen to the window.

use crate::Options;
use of_engine::screen::{decode_planar_screen, Bitmap};
use of_engine::{Palette, SCREEN_H, SCREEN_W};
use of_game::{Game, Input, TICK_HZ};
use std::collections::HashSet;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

/// Displayed aspect ratio of the game screen (a 4:3 monitor).
const ASPECT: f64 = 4.0 / 3.0;

enum Mode {
    Title,
    Play(Box<Game>),
}

struct App {
    opts: Options,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    mode: Mode,
    title: (Bitmap, Palette),
    screen: Bitmap,
    palette: Palette,
    keys: HashSet<KeyCode>,
    last: Instant,
    accum: Duration,
    error: Option<String>,
}

/// Opens the window and runs until it is closed.
pub fn run(opts: Options) -> Result<(), String> {
    let title = load_title(&opts).map_err(|e| e.to_string())?;
    let mut app = App {
        mode: Mode::Title,
        title,
        screen: Bitmap::screen(),
        palette: Palette::default(),
        keys: HashSet::new(),
        last: Instant::now(),
        accum: Duration::ZERO,
        window: None,
        surface: None,
        error: None,
        opts,
    };
    if let Some(level) = app.opts.level {
        app.start(level);
    }
    let el = EventLoop::new().map_err(|e| e.to_string())?;
    el.run_app(&mut app).map_err(|e| e.to_string())?;
    app.error.map_or(Ok(()), Err)
}

fn load_title(opts: &Options) -> std::io::Result<(Bitmap, Palette)> {
    let bmp = decode_planar_screen(&opts.data.read("menu1.map")?);
    let pal = Palette::from_rgb8(&opts.data.read("menu1.pal")?);
    Ok((bmp, pal))
}

impl App {
    fn start(&mut self, level: usize) {
        match Game::new(&self.opts.data, level) {
            Ok(mut g) => {
                if let Some(r) = self.opts.room.take() {
                    g.goto_room(r);
                }
                self.mode = Mode::Play(Box::new(g));
            }
            Err(e) => eprintln!("cannot start level {}: {e}", level + 1),
        }
    }

    fn input(&self) -> Input {
        let k = |a: KeyCode, b: KeyCode| self.keys.contains(&a) || self.keys.contains(&b);
        Input {
            left: k(KeyCode::ArrowLeft, KeyCode::KeyA),
            right: k(KeyCode::ArrowRight, KeyCode::KeyD),
            up: k(KeyCode::ArrowUp, KeyCode::KeyW),
            down: k(KeyCode::ArrowDown, KeyCode::KeyS),
        }
    }

    fn key_pressed(&mut self, el: &ActiveEventLoop, code: KeyCode) {
        match code {
            KeyCode::F11 => self.toggle_fullscreen(),
            KeyCode::Escape => match self.mode {
                Mode::Title => el.exit(),
                Mode::Play(_) => self.mode = Mode::Title,
            },
            KeyCode::Enter | KeyCode::Space if matches!(self.mode, Mode::Title) => self.start(0),
            KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit3 | KeyCode::Digit4 | KeyCode::Digit5 => {
                let n = code as usize - KeyCode::Digit1 as usize;
                self.start(n);
            }
            KeyCode::BracketLeft | KeyCode::BracketRight => {
                if let Mode::Play(g) = &mut self.mode {
                    let rooms = g.level.map.rooms();
                    if let Some(i) = rooms.iter().position(|&r| r == g.room) {
                        let n = rooms.len();
                        let j = if code == KeyCode::BracketLeft {
                            (i + n - 1) % n
                        } else {
                            (i + 1) % n
                        };
                        g.goto_room(rooms[j]);
                    }
                }
            }
            _ => {}
        }
    }

    fn toggle_fullscreen(&self) {
        if let Some(w) = &self.window {
            let fs = if w.fullscreen().is_some() {
                None
            } else {
                Some(Fullscreen::Borderless(None))
            };
            w.set_fullscreen(fs);
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        self.accum += now - self.last;
        self.last = now;
        let step = Duration::from_secs(1) / TICK_HZ;
        // Never try to catch up more than a quarter second after a stall.
        self.accum = self.accum.min(Duration::from_millis(250));
        let input = self.input();
        while self.accum >= step {
            self.accum -= step;
            if let Mode::Play(g) = &mut self.mode {
                g.tick(input);
            }
        }
        match &mut self.mode {
            Mode::Title => {
                self.screen.pixels.copy_from_slice(&self.title.0.pixels);
                self.palette = self.title.1.clone();
            }
            Mode::Play(g) => {
                g.render(&mut self.screen, &mut self.palette);
                if let Some(w) = &self.window {
                    w.set_title(&format!("OpenFlashback - level {} room {}", g.level.index + 1, g.room));
                }
            }
        }
    }

    fn present(&mut self) {
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buf) = surface.buffer_mut() else { return };
        let (ww, wh) = (size.width as usize, size.height as usize);
        let lut: Vec<u32> = self.palette.0.iter().map(|c| c.to_u32()).collect();
        // Largest 4:3 rectangle centred in the window.
        let (dw, dh) = if ww as f64 / wh as f64 > ASPECT {
            ((wh as f64 * ASPECT) as usize, wh)
        } else {
            (ww, (ww as f64 / ASPECT) as usize)
        };
        let (ox, oy) = ((ww - dw) / 2, (wh - dh) / 2);
        let xs: Vec<usize> = (0..dw).map(|x| x * SCREEN_W / dw).collect();
        buf.fill(0);
        for y in 0..dh {
            let src = &self.screen.pixels[(y * SCREEN_H / dh) * SCREEN_W..][..SCREEN_W];
            let row = &mut buf[(oy + y) * ww + ox..][..dw];
            for (d, &sx) in row.iter_mut().zip(&xs) {
                *d = lut[src[sx] as usize];
            }
        }
        let _ = buf.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let mut attrs = Window::default_attributes()
            .with_title("OpenFlashback")
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0));
        if !self.opts.windowed {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let window = match el.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                self.error = Some(format!("cannot create window: {e}"));
                el.exit();
                return;
            }
        };
        let surface =
            softbuffer::Context::new(window.clone()).and_then(|ctx| softbuffer::Surface::new(&ctx, window.clone()));
        match surface {
            Ok(s) => self.surface = Some(s),
            Err(e) => {
                self.error = Some(format!("cannot create drawing surface: {e}"));
                el.exit();
                return;
            }
        }
        self.window = Some(window);
        self.last = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Focused(false) => self.keys.clear(),
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                if event.state == ElementState::Pressed {
                    if !event.repeat {
                        self.keys.insert(code);
                        self.key_pressed(el, code);
                    }
                } else {
                    self.keys.remove(&code);
                }
            }
            WindowEvent::RedrawRequested => {
                self.update();
                self.present();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
        el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(8)));
    }
}
