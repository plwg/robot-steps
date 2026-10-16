//! Sound effects, made from simple waves when the game starts, so there are
//! no sound files to look after.

use std::f32::consts::TAU;

use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound};

use crate::level::Dir;

const RATE: u32 = 22050;

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    Triangle,
    /// A square wave, softened so it isn't harsh.
    Square,
    Noise,
}

/// One note: a wave sliding in pitch from `from` to `to` Hz over `len`
/// seconds, fading in quickly and out by the end.
#[derive(Clone, Copy)]
struct Note {
    at: f32,
    len: f32,
    from: f32,
    to: f32,
    wave: Wave,
    volume: f32,
    /// Wobble in pitch (Hz of wobble, and how far as a fraction).
    vibrato: (f32, f32),
}

const fn note(at: f32, len: f32, from: f32, to: f32, wave: Wave, volume: f32) -> Note {
    Note {
        at,
        len,
        from,
        to,
        wave,
        volume,
        vibrato: (0.0, 0.0),
    }
}

impl Note {
    const fn wobble(mut self, rate: f32, depth: f32) -> Note {
        self.vibrato = (rate, depth);
        self
    }
}

/// Mix the notes into samples.
fn render(notes: &[Note]) -> Vec<f32> {
    let end = notes.iter().map(|n| n.at + n.len).fold(0.0, f32::max);
    let mut out = vec![0.0; (end * RATE as f32) as usize + 1];
    let mut noise: u32 = 0x1234_5678;
    for n in notes {
        let start = (n.at * RATE as f32) as usize;
        let count = (n.len * RATE as f32) as usize;
        let mut phase = 0.0_f32;
        for i in 0..count {
            let t = i as f32 / RATE as f32;
            let k = t / n.len;
            // Slide in pitch evenly on a musical scale.
            let mut freq = n.from * (n.to / n.from).powf(k);
            freq *= 1.0 + (t * n.vibrato.0 * TAU).sin() * n.vibrato.1;
            phase = (phase + freq / RATE as f32).fract();
            let v = match n.wave {
                Wave::Sine => (phase * TAU).sin(),
                Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
                Wave::Square => ((phase * TAU).sin() * 3.0).clamp(-1.0, 1.0),
                Wave::Noise => {
                    noise ^= noise << 13;
                    noise ^= noise >> 17;
                    noise ^= noise << 5;
                    noise as f32 / u32::MAX as f32 * 2.0 - 1.0
                }
            };
            let attack = (t / 0.008).min(1.0);
            let fade = (1.0 - k).powf(1.5);
            if let Some(s) = out.get_mut(start + i) {
                *s += v * n.volume * attack * fade;
            }
        }
    }
    out
}

/// Samples as a 16-bit mono WAV file.
fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = samples.len() as u32 * 2;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend(b"RIFF");
    out.extend((36 + data_len).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes()); // PCM
    out.extend(1u16.to_le_bytes()); // mono
    out.extend(RATE.to_le_bytes());
    out.extend((RATE * 2).to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend(data_len.to_le_bytes());
    for s in samples {
        out.extend(((s.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes());
    }
    out
}

// Musical notes (Hz).
const C5: f32 = 523.25;
const D5: f32 = 587.33;
const E5: f32 = 659.26;
const G5: f32 = 783.99;
const A5: f32 = 880.0;
const C6: f32 = 1046.5;
const E6: f32 = 1318.5;
const G6: f32 = 1568.0;

/// Every sound the game makes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    /// A step in each direction has its own note (up is highest), so a
    /// program plays a little tune.
    Step(Dir),
    Slide,
    /// Coming to a stop against something at the end of a slide.
    Stop,
    Bump,
    Chop,
    Star,
    Key,
    Unlock,
    /// Looking to see what's there.
    Look,
    Yes,
    No,
    /// The mystery clouds puffing away.
    Poof,
    Sad,
    Win,
    /// A button.
    Tap,
    /// Sleeping a tick: a little snore.
    Sleep,
    /// Rain starting on the robot.
    Rain,
}

impl Sfx {
    const ALL: [Sfx; 20] = [
        Sfx::Step(Dir::Up),
        Sfx::Step(Dir::Down),
        Sfx::Step(Dir::Left),
        Sfx::Step(Dir::Right),
        Sfx::Slide,
        Sfx::Stop,
        Sfx::Bump,
        Sfx::Chop,
        Sfx::Star,
        Sfx::Key,
        Sfx::Unlock,
        Sfx::Look,
        Sfx::Yes,
        Sfx::No,
        Sfx::Poof,
        Sfx::Sad,
        Sfx::Win,
        Sfx::Tap,
        Sfx::Sleep,
        Sfx::Rain,
    ];

    fn index(self) -> usize {
        Sfx::ALL
            .iter()
            .position(|&s| s == self)
            .expect("every sound is listed")
    }

    fn notes(self) -> Vec<Note> {
        use Wave::*;
        match self {
            Sfx::Step(dir) => {
                let f = match dir {
                    Dir::Up => G5,
                    Dir::Right => E5,
                    Dir::Left => D5,
                    Dir::Down => C5,
                };
                vec![note(0.0, 0.11, f, f * 1.06, Triangle, 0.5)]
            }
            Sfx::Slide => vec![
                note(0.0, 0.6, 500.0, 1100.0, Sine, 0.3).wobble(7.0, 0.03),
                note(0.0, 0.6, 1.0, 1.0, Noise, 0.06),
            ],
            Sfx::Stop => vec![note(0.0, 0.12, 180.0, 90.0, Sine, 0.6)],
            Sfx::Bump => vec![note(0.0, 0.45, 320.0, 90.0, Sine, 0.7).wobble(18.0, 0.12)],
            Sfx::Chop => vec![
                note(0.0, 0.07, 1.0, 1.0, Noise, 0.5),
                note(0.0, 0.1, 220.0, 110.0, Sine, 0.6),
                note(0.2, 0.07, 1.0, 1.0, Noise, 0.5),
                note(0.2, 0.1, 220.0, 110.0, Sine, 0.6),
                note(0.42, 0.3, 1.0, 1.0, Noise, 0.15),
            ],
            Sfx::Star => vec![
                note(0.0, 0.12, C6, C6, Sine, 0.35),
                note(0.06, 0.12, E6, E6, Sine, 0.35),
                note(0.12, 0.12, G6, G6, Sine, 0.35),
                note(0.18, 0.3, C6 * 2.0, C6 * 2.0, Sine, 0.3),
            ],
            Sfx::Key => vec![
                note(0.0, 0.1, E6, E6, Triangle, 0.3),
                note(0.08, 0.1, G6, G6, Triangle, 0.3),
                note(0.16, 0.2, E6, E6, Triangle, 0.25).wobble(25.0, 0.02),
            ],
            Sfx::Unlock => vec![
                note(0.0, 0.03, 1.0, 1.0, Noise, 0.5),
                note(0.12, 0.03, 1.0, 1.0, Noise, 0.5),
                note(0.12, 0.15, 240.0, 140.0, Square, 0.25),
                note(0.3, 0.3, C5, G5, Triangle, 0.3),
            ],
            Sfx::Look => vec![note(0.0, 0.25, 420.0, 640.0, Sine, 0.3)],
            Sfx::Sleep => vec![
                note(0.0, 0.3, 110.0, 90.0, Triangle, 0.35).wobble(30.0, 0.08),
                note(0.0, 0.3, 1.0, 1.0, Noise, 0.04),
                note(0.4, 0.2, 700.0, 900.0, Sine, 0.12),
            ],
            Sfx::Rain => vec![note(0.0, 0.8, 1.0, 1.0, Noise, 0.25)],
            Sfx::Yes => vec![
                note(0.0, 0.1, E5, E5, Triangle, 0.35),
                note(0.09, 0.16, A5, A5, Triangle, 0.35),
            ],
            Sfx::No => vec![
                note(0.0, 0.1, 440.0, 440.0, Triangle, 0.3),
                note(0.09, 0.16, 330.0, 330.0, Triangle, 0.3),
            ],
            Sfx::Poof => vec![note(0.0, 0.4, 1.0, 1.0, Noise, 0.25)],
            Sfx::Sad => vec![
                note(0.0, 0.3, 392.0, 370.0, Triangle, 0.4).wobble(6.0, 0.02),
                note(0.3, 0.55, 349.0, 294.0, Triangle, 0.4).wobble(6.0, 0.03),
            ],
            Sfx::Win => vec![
                note(0.0, 0.14, C5, C5, Square, 0.22),
                note(0.12, 0.14, E5, E5, Square, 0.22),
                note(0.24, 0.14, G5, G5, Square, 0.22),
                note(0.36, 0.6, C6, C6, Square, 0.22).wobble(6.0, 0.01),
                note(0.36, 0.6, E5, E5, Triangle, 0.25),
            ],
            Sfx::Tap => vec![note(0.0, 0.05, 900.0, 700.0, Sine, 0.3)],
        }
    }
}

pub struct Sounds {
    /// One per sound, in the order of `Sfx::ALL`.
    sounds: Vec<Sound>,
}

impl Sounds {
    pub async fn new() -> Sounds {
        let mut sounds = Vec::new();
        for sfx in Sfx::ALL {
            let bytes = wav(&render(&sfx.notes()));
            let sound = load_sound_from_bytes(&bytes)
                .await
                .expect("a made-up sound always loads");
            sounds.push(sound);
        }
        Sounds { sounds }
    }

    pub fn play(&self, sfx: Sfx) {
        play_sound(
            &self.sounds[sfx.index()],
            PlaySoundParams {
                looped: false,
                volume: 0.8,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_are_loud_enough_but_never_clip() {
        for sfx in Sfx::ALL {
            let samples = render(&sfx.notes());
            let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
            assert!((0.15..=1.0).contains(&peak), "{sfx:?} peaks at {peak}");
            assert!(samples.len() < RATE as usize * 2, "{sfx:?} is too long");
        }
    }

    #[test]
    fn wav_header_matches_the_samples() {
        let bytes = wav(&[0.0, 0.5, -0.5]);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(bytes.len(), 44 + 6);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 6);
    }
}
