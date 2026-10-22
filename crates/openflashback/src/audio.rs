//! Sound output: streams music (interleaved 16-bit stereo at 44.1 kHz) to the default device.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

const MUSIC_RATE: f64 = 44100.0;

#[derive(Default)]
struct Mixer {
    out_rate: f64,
    music: Option<Receiver<Vec<i16>>>,
    chunk: Vec<i16>,
    /// Position in `chunk`, in stereo frames (fractional for resampling).
    pos: f64,
    volume: f32,
}

impl Mixer {
    fn mix(&mut self, out: &mut [f32], channels: usize) {
        out.fill(0.0);
        let step = MUSIC_RATE / self.out_rate;
        for frame in out.chunks_mut(channels) {
            let Some(rx) = &self.music else { return };
            while self.pos as usize + 1 >= self.chunk.len() / 2 {
                match rx.try_recv() {
                    Ok(next) => {
                        self.pos -= (self.chunk.len() / 2) as f64;
                        self.pos = self.pos.max(0.0);
                        self.chunk = next;
                    }
                    // The renderer is behind: leave the rest silent.
                    Err(_) => return,
                }
            }
            let i = self.pos as usize;
            let t = (self.pos - i as f64) as f32;
            let at = |k: usize, c: usize| self.chunk[k * 2 + c] as f32 / 32768.0;
            let l = at(i, 0) * (1.0 - t) + at(i + 1, 0) * t;
            let r = at(i, 1) * (1.0 - t) + at(i + 1, 1) * t;
            frame[0] = l * self.volume;
            if channels > 1 {
                frame[1] = r * self.volume;
            }
            self.pos += step;
        }
    }
}

/// The open audio device.
pub struct Audio {
    _stream: cpal::Stream,
    mixer: Arc<Mutex<Mixer>>,
}

impl Audio {
    pub fn new() -> Result<Audio, String> {
        let device = cpal::default_host().default_output_device().ok_or("no audio device")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let nch = config.channels as usize;
        let mixer = Arc::new(Mutex::new(Mixer {
            out_rate: config.sample_rate.0 as f64,
            volume: 0.7,
            ..Default::default()
        }));
        let m2 = mixer.clone();
        let err = |e| eprintln!("audio error: {e}");
        let stream = match format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data: &mut [f32], _| m2.lock().unwrap().mix(data, nch),
                err,
                None,
            ),
            cpal::SampleFormat::I16 => {
                let mut tmp = Vec::new();
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _| {
                        tmp.resize(data.len(), 0.0f32);
                        m2.lock().unwrap().mix(&mut tmp, nch);
                        for (d, s) in data.iter_mut().zip(&tmp) {
                            *d = (s * 32767.0) as i16;
                        }
                    },
                    err,
                    None,
                )
            }
            other => return Err(format!("unsupported sample format {other:?}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Audio { _stream: stream, mixer })
    }

    /// Plays a music stream, replacing any playing one; `None` stops the music.
    pub fn play_music(&self, music: Option<Receiver<Vec<i16>>>) {
        let mut m = self.mixer.lock().unwrap();
        m.music = music;
        m.chunk.clear();
        m.pos = 0.0;
    }
}
