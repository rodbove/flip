use serde::{Deserialize, Serialize};

use crate::kit;
use crate::project::{Fm, Mode, Morph, Op, Pad, Pluck, Source, Synth, Wave};
use crate::synth::fm::HOLD;

pub const CATEGORIES: [&str; 9] = [
    "KEYS", "BASS", "LEAD", "PAD", "PLUCK", "BELL", "BRASS", "DRUM", "USER",
];
pub const USER: &str = "USER";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub category: String,
    pub sound: Pad,
}

impl Preset {
    pub fn capture(name: &str, pad: &Pad) -> Self {
        let mut sound = pad.clone();
        sound.sample = None;
        sound.sample_id = None;
        sound.choke = 0;
        sound.name = name.to_lowercase();
        Self {
            name: name.to_string(),
            category: USER.into(),
            sound,
        }
    }

    pub fn apply(&self, pad: &mut Pad) {
        let sample = pad.sample.take();
        let sample_id = pad.sample_id.take();
        let choke = pad.choke;
        *pad = self.sound.clone();
        pad.sample = sample;
        pad.sample_id = sample_id;
        pad.choke = choke;
        pad.name = self.name.to_lowercase();
    }
}

pub(crate) struct Amp {
    pub(crate) attack: f32,
    pub(crate) decay: f32,
    pub(crate) sustain: f32,
    pub(crate) release: f32,
}

pub(crate) const fn amp(attack: f32, decay: f32, sustain: f32, release: f32) -> Amp {
    Amp {
        attack,
        decay,
        sustain,
        release,
    }
}

pub(crate) fn voice(source: Source, env: Amp, gain: f32) -> Pad {
    Pad {
        source,
        attack: env.attack,
        decay: env.decay,
        sustain: env.sustain,
        release: env.release,
        mode: Mode::Gate,
        gain,
        ..Pad::default()
    }
}

fn preset(name: &str, category: &str, sound: Pad) -> Preset {
    let mut sound = sound;
    sound.name = name.to_lowercase();
    Preset {
        name: name.into(),
        category: category.into(),
        sound,
    }
}

pub(crate) fn op(ratio: f32, level: f32, decay: f32, fine: f32) -> Op {
    Op {
        ratio,
        level,
        decay,
        fine,
    }
}

pub(crate) fn fm(
    algorithm: u8,
    ops: [Op; 4],
    feedback: f32,
    depth: f32,
    touch: f32,
    env: Amp,
) -> Pad {
    Pad {
        fm: Fm {
            ops,
            algorithm,
            feedback,
            depth,
            velocity: touch,
        },
        ..voice(Source::Fm, env, 0.7)
    }
}

pub(crate) fn string(damping: f32, brightness: f32, body: f32, bow: f32, env: Amp) -> Pad {
    Pad {
        pluck: Pluck {
            damping,
            brightness,
            body,
            bow,
        },
        ..voice(Source::String, env, 0.8)
    }
}

pub(crate) fn morph(
    position: f32,
    spread: f32,
    sub: f32,
    drive: f32,
    cutoff: f32,
    env: Amp,
) -> Pad {
    Pad {
        morph: Morph {
            position,
            spread,
            sub,
            drive,
        },
        cutoff,
        ..voice(Source::Morph, env, 0.6)
    }
}

pub(crate) fn analog(a: Wave, b: Wave, detune: f32, mix: f32, cutoff: f32, env: Amp) -> Pad {
    Pad {
        synth: Synth {
            wave_a: a,
            wave_b: b,
            detune,
            mix,
            sweep: 0.0,
        },
        cutoff,
        ..voice(Source::Synth, env, 0.6)
    }
}

pub(crate) fn pitched(mut pad: Pad, pitch: f32) -> Pad {
    pad.pitch = pitch;
    pad
}

pub fn factory() -> Vec<Preset> {
    use Wave::*;
    let mut out = vec![
        preset(
            "E-Piano",
            "KEYS",
            fm(
                3,
                [
                    op(1.0, 1.0, 3.0, 0.0),
                    op(14.0, 0.25, 0.3, 0.0),
                    op(1.0, 0.8, 2.5, 3.0),
                    op(1.0, 0.5, 1.2, 0.0),
                ],
                0.0,
                1.0,
                0.8,
                amp(0.002, 2.5, 0.25, 0.4),
            ),
        ),
        preset(
            "DX Piano",
            "KEYS",
            fm(
                3,
                [
                    op(1.0, 1.0, 6.0, 0.0),
                    op(1.0, 0.7, 0.9, 0.0),
                    op(2.0, 0.5, 3.0, 0.0),
                    op(3.0, 0.4, 0.5, 0.0),
                ],
                0.0,
                1.0,
                0.7,
                amp(0.001, 3.0, 0.2, 0.3),
            ),
        ),
        preset(
            "Clav",
            "KEYS",
            fm(
                0,
                [
                    op(1.0, 1.0, HOLD, 0.0),
                    op(3.0, 0.8, 0.25, 0.0),
                    op(1.0, 0.4, 0.2, 0.0),
                    op(1.0, 0.2, 0.2, 0.0),
                ],
                0.3,
                1.0,
                0.6,
                amp(0.001, 0.4, 0.3, 0.08),
            ),
        ),
        preset(
            "Organ",
            "KEYS",
            fm(
                7,
                [
                    op(0.5, 0.8, HOLD, 0.0),
                    op(1.0, 0.9, HOLD, 0.0),
                    op(2.0, 0.6, HOLD, 0.0),
                    op(3.0, 0.4, HOLD, 0.0),
                ],
                0.0,
                1.0,
                0.0,
                amp(0.005, 0.1, 1.0, 0.06),
            ),
        ),
        preset(
            "Morph Organ",
            "KEYS",
            morph(0.6, 0.0, 0.4, 0.1, 1.0, amp(0.01, 0.1, 1.0, 0.05)),
        ),
        preset(
            "FM Bass",
            "BASS",
            pitched(
                fm(
                    0,
                    [
                        op(1.0, 1.0, HOLD, 0.0),
                        op(1.0, 0.9, 0.35, 0.0),
                        op(1.0, 0.3, 0.2, 0.0),
                        op(1.0, 0.2, 0.2, 0.0),
                    ],
                    0.4,
                    1.0,
                    0.5,
                    amp(0.001, 0.6, 0.5, 0.08),
                ),
                -24.0,
            ),
        ),
        preset(
            "Analog Bass",
            "BASS",
            pitched(
                Pad {
                    env_amount: 0.3,
                    ..analog(Saw, Square, 0.05, 0.4, 0.35, amp(0.002, 0.25, 0.3, 0.1))
                },
                -24.0,
            ),
        ),
        preset(
            "Acid",
            "BASS",
            pitched(
                Pad {
                    resonance: 0.8,
                    env_amount: 0.6,
                    ..analog(Saw, Saw, 0.0, 0.0, 0.25, amp(0.001, 0.2, 0.1, 0.08))
                },
                -24.0,
            ),
        ),
        preset(
            "Reese",
            "BASS",
            pitched(
                morph(2.0, 0.3, 0.5, 0.4, 0.35, amp(0.005, 0.3, 0.9, 0.15)),
                -24.0,
            ),
        ),
        preset(
            "Pluck Bass",
            "BASS",
            pitched(
                string(0.55, 0.35, 0.1, 0.0, amp(0.001, 1.0, 1.0, 0.2)),
                -24.0,
            ),
        ),
        preset(
            "Square Lead",
            "LEAD",
            morph(3.0, 0.05, 0.0, 0.3, 0.8, amp(0.005, 0.2, 0.9, 0.1)),
        ),
        preset(
            "FM Lead",
            "LEAD",
            fm(
                1,
                [
                    op(1.0, 1.0, HOLD, 0.0),
                    op(2.0, 0.6, HOLD, 0.0),
                    op(1.0, 0.3, HOLD, 0.0),
                    op(1.0, 0.4, HOLD, 0.0),
                ],
                0.5,
                1.0,
                0.4,
                amp(0.01, 0.2, 0.8, 0.15),
            ),
        ),
        preset(
            "Saw Lead",
            "LEAD",
            analog(Saw, Saw, 0.12, 0.5, 0.7, amp(0.005, 0.2, 0.8, 0.12)),
        ),
        preset(
            "Warm Pad",
            "PAD",
            morph(1.4, 0.25, 0.3, 0.0, 0.5, amp(0.8, 0.5, 0.9, 1.5)),
        ),
        preset(
            "Strings",
            "PAD",
            morph(2.0, 0.2, 0.0, 0.0, 0.55, amp(0.4, 0.5, 0.9, 1.2)),
        ),
        preset(
            "Analog Pad",
            "PAD",
            Pad {
                env_amount: 0.1,
                ..analog(Saw, Saw, 0.12, 0.4, 0.45, amp(0.4, 0.5, 0.8, 1.0))
            },
        ),
        preset(
            "Cello",
            "PAD",
            pitched(string(0.7, 0.3, 0.2, 0.8, amp(0.15, 0.3, 1.0, 0.4)), -12.0),
        ),
        preset(
            "Violin",
            "PAD",
            string(0.6, 0.5, 0.3, 0.7, amp(0.1, 0.3, 1.0, 0.3)),
        ),
        preset(
            "Harp",
            "PLUCK",
            string(0.35, 0.55, 0.2, 0.0, amp(0.001, 1.0, 1.0, 0.6)),
        ),
        preset(
            "Koto",
            "PLUCK",
            string(0.5, 0.8, 0.45, 0.0, amp(0.001, 1.0, 1.0, 0.4)),
        ),
        preset(
            "Nylon",
            "PLUCK",
            string(0.45, 0.4, 0.15, 0.0, amp(0.001, 1.0, 1.0, 0.5)),
        ),
        preset(
            "Steel",
            "PLUCK",
            string(0.2, 0.9, 0.3, 0.0, amp(0.001, 1.0, 1.0, 0.6)),
        ),
        preset(
            "Analog Pluck",
            "PLUCK",
            Pad {
                env_amount: 0.3,
                ..analog(Triangle, Saw, 0.05, 0.4, 0.6, amp(0.001, 0.3, 0.0, 0.2))
            },
        ),
        preset(
            "DX Bells",
            "BELL",
            fm(
                3,
                [
                    op(1.0, 1.0, 4.0, 0.0),
                    op(3.5, 0.6, 2.0, 0.0),
                    op(1.0, 0.6, 3.0, 7.0),
                    op(7.0, 0.4, 1.0, 0.0),
                ],
                0.0,
                1.0,
                0.5,
                amp(0.001, 4.0, 0.0, 1.5),
            ),
        ),
        preset(
            "Glass",
            "BELL",
            fm(
                6,
                [
                    op(1.0, 0.6, 3.0, 0.0),
                    op(2.0, 0.5, 2.5, -5.0),
                    op(4.0, 0.5, 2.0, 0.0),
                    op(9.0, 0.5, 0.8, 0.0),
                ],
                0.0,
                1.0,
                0.5,
                amp(0.001, 3.0, 0.0, 1.2),
            ),
        ),
        preset(
            "Marimba",
            "BELL",
            fm(
                3,
                [
                    op(1.0, 1.0, 0.6, 0.0),
                    op(4.0, 0.5, 0.08, 0.0),
                    op(4.0, 0.2, 0.2, 0.0),
                    op(10.0, 0.3, 0.05, 0.0),
                ],
                0.0,
                1.0,
                0.6,
                amp(0.001, 0.6, 0.0, 0.3),
            ),
        ),
        preset(
            "Brass",
            "BRASS",
            fm(
                0,
                [
                    op(1.0, 1.0, HOLD, 0.0),
                    op(1.0, 0.75, HOLD, 0.0),
                    op(1.0, 0.2, HOLD, 0.0),
                    op(1.0, 0.3, HOLD, 0.0),
                ],
                0.5,
                1.0,
                0.7,
                amp(0.06, 0.3, 0.8, 0.15),
            ),
        ),
        preset(
            "Synth Brass",
            "BRASS",
            Pad {
                env_amount: 0.4,
                ..analog(Saw, Saw, 0.08, 0.5, 0.4, amp(0.05, 0.4, 0.7, 0.2))
            },
        ),
    ];
    for pad in kit::drums() {
        let name = pad.name.clone();
        let mut sound = pad;
        sound.choke = 0;
        out.push(preset(&title(&name), "DRUM", sound));
    }
    out
}

fn title(name: &str) -> String {
    name.split(' ')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().chain(c).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::sample::Sample;

    mod when_listing_the_factory_bank {
        use super::*;

        #[test]
        fn fills_every_built_in_category() {
            let bank = factory();
            for c in CATEGORIES.iter().filter(|c| **c != USER) {
                assert!(bank.iter().any(|p| p.category == *c), "{c}");
            }
        }

        #[test]
        fn uses_unique_names() {
            let bank = factory();
            for (i, p) in bank.iter().enumerate() {
                assert!(
                    !bank[i + 1..].iter().any(|q| q.name == p.name),
                    "{}",
                    p.name
                );
            }
        }
    }

    mod when_applying_a_preset {
        use super::*;

        #[test]
        fn keeps_the_sample_and_choke() {
            let mut pad = Pad {
                sample: Some(Arc::new(Sample::new("vox", 48000, vec![[0.1, 0.1]; 10]))),
                sample_id: Some("abc".into()),
                choke: 2,
                ..Pad::default()
            };
            let bank = factory();
            let piano = bank.iter().find(|p| p.name == "E-Piano").unwrap();
            piano.apply(&mut pad);
            assert_eq!(pad.source, Source::Fm);
            assert_eq!(pad.choke, 2);
            assert_eq!(pad.sample_id.as_deref(), Some("abc"));
            assert!(pad.sample.is_some());
            assert_eq!(pad.name, "e-piano");
        }
    }

    mod when_saving_a_pad {
        use super::*;

        #[test]
        fn drops_the_sample_and_round_trips() {
            let mut pad = Pad {
                sample_id: Some("abc".into()),
                choke: 3,
                ..Pad::default()
            };
            pad.source = Source::Morph;
            pad.morph.position = 2.5;
            let preset = Preset::capture("My Pad", &pad);
            assert_eq!(preset.category, USER);
            assert!(preset.sound.sample_id.is_none());
            let json = serde_json::to_string(&preset).unwrap();
            let back: Preset = serde_json::from_str(&json).unwrap();
            assert_eq!(back.sound.morph.position, 2.5);
            assert_eq!(back.sound.source, Source::Morph);
        }
    }
}
