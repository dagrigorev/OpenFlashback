//! Flashback's music: MIDI songs (`.mid`) with a profile (`.prf`) naming an AdLib instrument
//! (`.ins`) for each MIDI channel.
//!
//! A profile starts with 16 instrument names of 30 bytes, one per MIDI track (track 0, the tempo
//! track, never has one; songs often put every track on channel 0), empty when unused,
//! then 16 little-endian `i16` transposes in semitones (offset 480), 16 `i16` volume adjustments
//! (offset 512, not used yet) and the name of the song file (offset 550).
//!
//! An `.ins` file is the 80-byte AdLib instrument format: a mode byte (0 melodic, 1 percussive)
//! and a voice byte, 13 little-endian words for the modulator and 13 for the carrier (key-scale
//! level, multiplier, feedback, attack, sustain level, sustaining envelope, decay, release, total
//! level, tremolo, vibrato, key-scale rate, connection), then the two waveforms.

use crate::fm::Timbre;
use crate::midi::{Song, BANK_SIZE};
use of_engine::DataDir;

/// A song's profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// Instrument per MIDI track, without the `.ins` extension.
    pub instruments: [String; 16],
    pub transpose: [i16; 16],
    pub song: String,
}

const NAME: usize = 30;

/// Reads a `.prf` profile.
pub fn parse_profile(b: &[u8]) -> Option<Profile> {
    // The AdLib profiles are 753 bytes; other sizes belong to other sound drivers.
    if b.len() != 753 {
        return None;
    }
    let name = |s: &[u8]| {
        let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
        String::from_utf8_lossy(&s[..end]).trim().to_string()
    };
    Some(Profile {
        instruments: std::array::from_fn(|i| name(&b[i * NAME..(i + 1) * NAME])),
        transpose: std::array::from_fn(|i| i16::from_le_bytes([b[480 + i * 2], b[481 + i * 2]])),
        song: name(&b[550..570]),
    })
}

/// Converts an 80-byte AdLib `.ins` instrument to the FM synthesizer's register form.
pub fn parse_ins(b: &[u8]) -> Option<Timbre> {
    if b.len() < 58 {
        return None;
    }
    let w = |i: usize| u16::from_le_bytes([b[2 + i * 2], b[3 + i * 2]]) as u8;
    // Operator fields: 0 KSL, 1 MULT, 2 FB, 3 AR, 4 SL, 5 EG, 6 DR, 7 RR, 8 TL, 9 AM, 10 VIB,
    // 11 KSR, 12 CON.
    let op = |o: usize| {
        let f = |k: usize| w(o * 13 + k);
        (
            (f(9) & 1) << 7 | (f(10) & 1) << 6 | (f(5) & 1) << 5 | (f(11) & 1) << 4 | (f(1) & 15),
            (f(0) & 3) << 6 | (f(8) & 63),
            (f(3) & 15) << 4 | (f(6) & 15),
            (f(4) & 15) << 4 | (f(7) & 15),
        )
    };
    let (m, c) = (op(0), op(1));
    let wave = |i: usize| u16::from_le_bytes([b[54 + i * 2], b[55 + i * 2]]) as u8 & 7;
    let feedback = (w(2) & 7) << 1 | u8::from(w(12) & 1 == 0);
    Some(Timbre {
        flags: [m.0, c.0],
        level: [m.1, c.1],
        attack_decay: [m.2, c.2],
        sustain_release: [m.3, c.3],
        wave: [wave(0), wave(1)],
        feedback,
        transpose: 0,
        velocity: 0,
    })
}

/// Loads the song of profile `prf` (without extension) and its instruments. Each track is moved
/// to the channel with its own number, so bank entry `n` is track `n`'s instrument.
pub fn load_song(data: &DataDir, prf: &str) -> Option<(Song, Vec<Timbre>)> {
    let profile = parse_profile(&data.read(&format!("{prf}.prf")).ok()?)?;
    let mut song = Song::parse(&data.read(&profile.song).ok()?).ok()?;
    for (n, track) in song.tracks.iter_mut().enumerate() {
        track.set_channel((n % 16) as u8);
    }
    let mut bank = vec![Timbre::default(); BANK_SIZE];
    for (ch, name) in profile.instruments.iter().enumerate() {
        if name.is_empty() {
            continue;
        }
        let file = if name.ends_with(".ins") {
            name.clone()
        } else {
            format!("{name}.ins")
        };
        if let Some(t) = data.read(&file).ok().and_then(|b| parse_ins(&b)) {
            bank[ch] = Timbre {
                transpose: profile.transpose[ch].clamp(-48, 48) as i8,
                ..t
            };
        }
    }
    Some((song, bank))
}

/// The music profile played with a cutscene, by the cutscene's name.
pub fn cutscene_music(cutscene: &str) -> Option<&'static str> {
    Some(match cutscene {
        "logos" => "present3",
        "intro1" => "introl3",
        "memo" => "memory3",
        "voyage" => "voyage3",
        "taxi" => "taxi3",
        "teleport" => "telepor3",
        "holoseq" => "holo3",
        "lift" => "lift3",
        "objet" => "objet3",
        "pont" => "pont3",
        "chute" => "chute3",
        "desinteg" => "desinte3",
        "fin" => "end31",
        "over" => "gameove3",
        "missions" => "mission3",
        "gen" => "generat3",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ins_operator_fields_become_opl_registers() {
        let mut b = vec![0u8; 80];
        let mut put = |word: usize, v: u16| b[2 + word * 2..4 + word * 2].copy_from_slice(&v.to_le_bytes());
        // Modulator: MULT 1, FB 5, AR 15, SL 1, DR 1, RR 2, TL 16, CON 1 (FM).
        for (k, v) in [(1, 1), (2, 5), (3, 15), (4, 1), (6, 1), (7, 2), (8, 16), (12, 1)] {
            put(k, v);
        }
        // Carrier: MULT 1, AR 15, SL 8, EG 1, DR 1, RR 2.
        for (k, v) in [(1, 1), (3, 15), (4, 8), (5, 1), (6, 1), (7, 2)] {
            put(13 + k, v);
        }
        b[56] = 2; // carrier waveform
        let t = parse_ins(&b).unwrap();
        assert_eq!(t.flags, [0x01, 0x21]);
        assert_eq!(t.level, [16, 0]);
        assert_eq!(t.attack_decay, [0xf1, 0xf1]);
        assert_eq!(t.sustain_release, [0x12, 0x82]);
        assert_eq!(t.wave, [0, 2]);
        assert_eq!(t.feedback, 5 << 1);
    }

    /// With `OPENFLASHBACK_DATA` set: every profile's song and instruments load, and the
    /// opening 10 seconds render audibly without clipping.
    #[test]
    fn original_songs_render() {
        let Some(dir) = std::env::var("OPENFLASHBACK_DATA").ok() else {
            return;
        };
        let data = DataDir::open(dir).unwrap();
        let mut n = 0;
        for e in std::fs::read_dir(data.root()).unwrap().flatten() {
            let name = e.file_name().to_string_lossy().to_lowercase();
            let Some(prf) = name.strip_suffix(".prf") else { continue };
            let Some((song, bank)) = load_song(&data, prf) else {
                continue;
            };
            let used = bank.iter().filter(|t| **t != Timbre::default()).count();
            if used == 0 {
                // opt.prf names no instruments; it is not played with the AdLib driver.
                continue;
            }
            let mut p = crate::midi::Player::new(song, bank, 22050.0).with_channel_timbres();
            let mut out = vec![0i16; 22050 * 2 * 10];
            p.render(&mut out);
            let peak = out.iter().map(|&s| (s as i32).abs()).max().unwrap();
            assert!(peak > 300 && peak < 32767, "{prf}: peak {peak}");
            n += 1;
        }
        assert!(n >= 25, "only {n} profiles");
    }

    #[test]
    fn profile_names_transposes_and_song() {
        let mut b = vec![0u8; 753];
        b[30..39].copy_from_slice(b"elpiano1a");
        b[482..484].copy_from_slice(&(-12i16).to_le_bytes());
        b[550..562].copy_from_slice(b"introlon.mid");
        let p = parse_profile(&b).unwrap();
        assert_eq!(p.instruments[1], "elpiano1a");
        assert_eq!(p.instruments[0], "");
        assert_eq!(p.transpose[1], -12);
        assert_eq!(p.song, "introlon.mid");
    }
}
