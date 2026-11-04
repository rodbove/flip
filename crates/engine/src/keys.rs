use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::fx::Fx;
use crate::project::{Filter, Fm, Mode, Morph, Op, PADS, Pad, Pluck, Source, Synth, Wave};
use crate::synth::fm::HOLD;

pub const KEYS: usize = PADS;
pub const VIBRATO_HZ: f32 = 5.5;
pub const CATEGORIES: [&str; 8] = [
    "PIANO", "ORGAN", "STRINGS", "PAD", "LEAD", "BASS", "MALLET", "PLUCK",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Keys {
    pub patch: String,
    pub sound: Pad,
    pub fx: Fx,
    pub vibrato: f32,
}

impl Default for Keys {
    fn default() -> Self {
        PATCHES[0].keys.clone()
    }
}

pub struct Patch {
    pub name: &'static str,
    pub category: usize,
    pub keys: Keys,
}

pub static PATCHES: LazyLock<Vec<Patch>> = LazyLock::new(patches);

pub fn in_category(category: usize) -> Vec<usize> {
    PATCHES
        .iter()
        .enumerate()
        .filter(|(_, p)| p.category == category)
        .map(|(i, _)| i)
        .collect()
}

pub fn find(name: &str) -> Option<usize> {
    PATCHES.iter().position(|p| p.name == name)
}

const H: f32 = HOLD + 1.0;

struct B(Keys);

fn base(source: Source) -> B {
    B(Keys {
        patch: String::new(),
        sound: Pad {
            source,
            mode: Mode::Gate,
            gain: 0.7,
            attack: 0.002,
            decay: 1.0,
            sustain: 1.0,
            release: 0.2,
            ..Pad::default()
        },
        fx: Fx::default(),
        vibrato: 0.0,
    })
}

fn analog(wave_a: Wave, wave_b: Wave, detune: f32, mix: f32) -> B {
    let mut b = base(Source::Synth);
    b.0.sound.synth = Synth {
        wave_a,
        wave_b,
        detune,
        mix,
        sweep: 0.0,
    };
    b
}

fn fm(algorithm: u8, ops: [(f32, f32, f32); 4], feedback: f32, touch: f32) -> B {
    let mut b = base(Source::Fm);
    b.0.sound.fm = Fm {
        ops: ops.map(|(ratio, level, decay)| Op {
            ratio,
            level,
            decay,
            fine: 0.0,
        }),
        algorithm,
        feedback,
        depth: 1.0,
        velocity: touch,
    };
    b
}

fn organ(ratios: [f32; 4], levels: [f32; 4]) -> B {
    let ops = std::array::from_fn(|i| (ratios[i], levels[i], H));
    fm(7, ops, 0.0, 0.2).env(0.004, 1.0, 1.0, 0.05).gain(0.75)
}

fn pluck(damping: f32, brightness: f32, body: f32, bow: f32) -> B {
    let mut b = base(Source::String);
    b.0.sound.pluck = Pluck {
        damping,
        brightness,
        body,
        bow,
    };
    b.env(0.001, 1.0, 1.0, 0.25).gain(1.0)
}

fn morph(position: f32, spread: f32, sub: f32, drive: f32) -> B {
    let mut b = base(Source::Morph);
    b.0.sound.morph = Morph {
        position,
        spread,
        sub,
        drive,
    };
    b
}

impl B {
    fn env(mut self, attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        let s = &mut self.0.sound;
        s.attack = attack;
        s.decay = decay;
        s.sustain = sustain;
        s.release = release;
        self
    }

    fn filter(mut self, cutoff: f32, resonance: f32, amount: f32) -> Self {
        let s = &mut self.0.sound;
        s.cutoff = cutoff;
        s.resonance = resonance;
        s.env_amount = amount;
        self
    }

    fn band(mut self) -> Self {
        self.0.sound.filter = Filter::Band;
        self
    }

    fn pitch(mut self, semis: f32) -> Self {
        self.0.sound.pitch = semis;
        self
    }

    fn gain(mut self, gain: f32) -> Self {
        self.0.sound.gain = gain;
        self
    }

    fn sweep(mut self, semis: f32) -> Self {
        self.0.sound.synth.sweep = semis;
        self
    }

    fn fine(mut self, op: usize, cents: f32) -> Self {
        self.0.sound.fm.ops[op].fine = cents;
        self
    }

    fn vib(mut self, semis: f32) -> Self {
        self.0.vibrato = semis;
        self
    }

    fn drive(mut self, v: f32) -> Self {
        self.0.fx.drive = v;
        self
    }

    fn phaser(mut self, v: f32) -> Self {
        self.0.fx.phaser = v;
        self
    }

    fn chorus(mut self, v: f32) -> Self {
        self.0.fx.chorus = v;
        self
    }

    fn rotary(mut self, v: f32) -> Self {
        self.0.fx.rotary = v;
        self
    }

    fn trem(mut self, v: f32) -> Self {
        self.0.fx.tremolo = v;
        self
    }

    fn delay(mut self, v: f32, time: u8) -> Self {
        self.0.fx.delay = v;
        self.0.fx.time = time;
        self
    }

    fn reverb(mut self, v: f32) -> Self {
        self.0.fx.reverb = v;
        self
    }

    fn named(mut self, name: &'static str, category: usize) -> Patch {
        self.0.patch = name.into();
        self.0.sound.name = name.to_lowercase();
        Patch {
            name,
            category,
            keys: self.0,
        }
    }
}

const PIANO: usize = 0;
const ORGAN: usize = 1;
const STRINGS: usize = 2;
const PAD: usize = 3;
const LEAD: usize = 4;
const BASS: usize = 5;
const MALLET: usize = 6;
const PLUCK: usize = 7;

fn patches() -> Vec<Patch> {
    use Wave::*;
    vec![
        fm(
            3,
            [
                (1.0, 1.0, 3.0),
                (1.0, 0.45, 0.8),
                (2.0, 0.35, 2.0),
                (7.0, 0.12, 0.08),
            ],
            0.0,
            0.8,
        )
        .fine(2, 3.0)
        .env(0.002, 3.5, 0.0, 0.4)
        .filter(0.8, 0.0, 0.2)
        .reverb(0.3)
        .named("GRAND PIANO", PIANO),
        fm(
            3,
            [
                (1.0, 1.0, 2.5),
                (1.0, 0.6, 0.6),
                (2.0, 0.4, 1.6),
                (7.0, 0.22, 0.1),
            ],
            0.0,
            0.7,
        )
        .fine(2, 4.0)
        .env(0.001, 3.0, 0.0, 0.35)
        .reverb(0.2)
        .named("BRIGHT PIANO", PIANO),
        fm(
            3,
            [
                (1.0, 1.0, 2.5),
                (1.0, 0.5, 0.7),
                (1.0, 0.8, 2.5),
                (5.0, 0.15, 0.1),
            ],
            0.0,
            0.7,
        )
        .fine(2, 14.0)
        .env(0.001, 2.8, 0.0, 0.3)
        .filter(0.85, 0.0, 0.1)
        .reverb(0.15)
        .named("HONKY TONK", PIANO),
        fm(
            3,
            [
                (1.0, 1.0, 2.5),
                (1.0, 0.25, 1.5),
                (1.0, 0.6, 1.2),
                (14.0, 0.35, 0.12),
            ],
            0.0,
            0.9,
        )
        .fine(2, 2.0)
        .env(0.002, 4.0, 0.0, 0.35)
        .trem(0.25)
        .chorus(0.2)
        .reverb(0.2)
        .named("TINE EP", PIANO),
        fm(
            3,
            [
                (1.0, 1.0, 2.5),
                (1.0, 0.3, 1.2),
                (1.0, 0.6, 1.2),
                (14.0, 0.3, 0.1),
            ],
            0.0,
            0.9,
        )
        .env(0.002, 4.0, 0.0, 0.35)
        .trem(0.7)
        .drive(0.15)
        .reverb(0.15)
        .named("SUITCASE EP", PIANO),
        fm(
            3,
            [
                (1.0, 1.0, 3.0),
                (1.0, 0.35, 1.5),
                (1.0, 0.7, 1.5),
                (14.0, 0.55, 0.15),
            ],
            0.0,
            0.9,
        )
        .fine(2, 5.0)
        .env(0.001, 4.0, 0.0, 0.4)
        .chorus(0.6)
        .reverb(0.3)
        .named("DX EP", PIANO),
        fm(
            3,
            [
                (1.0, 1.0, 2.0),
                (1.0, 0.7, 0.35),
                (3.0, 0.25, 0.5),
                (1.0, 0.3, 0.2),
            ],
            0.0,
            0.85,
        )
        .env(0.002, 2.5, 0.0, 0.2)
        .drive(0.25)
        .trem(0.45)
        .named("REED EP", PIANO),
        fm(
            3,
            [
                (1.0, 1.0, 2.5),
                (1.0, 0.35, 1.0),
                (1.0, 0.6, 1.2),
                (14.0, 0.3, 0.1),
            ],
            0.0,
            0.8,
        )
        .env(0.002, 3.0, 0.0, 0.3)
        .filter(0.55, 0.0, 0.0)
        .drive(0.3)
        .chorus(0.4)
        .vib(0.06)
        .reverb(0.2)
        .named("LO-FI KEYS", PIANO),
        pluck(0.7, 1.0, 0.15, 0.0)
            .env(0.001, 1.0, 1.0, 0.06)
            .filter(0.85, 0.3, 0.15)
            .phaser(0.35)
            .named("CLAV", PIANO),
        pluck(0.45, 0.95, 0.45, 0.0)
            .reverb(0.3)
            .named("HARPSICHORD", PIANO),
        fm(
            3,
            [
                (1.0, 1.0, 0.8),
                (3.5, 0.3, 0.2),
                (4.0, 0.2, 0.4),
                (1.0, 0.0, 1.0),
            ],
            0.0,
            0.6,
        )
        .pitch(12.0)
        .env(0.001, 1.2, 0.0, 0.2)
        .reverb(0.2)
        .named("TOY PIANO", PIANO),
        organ([0.5, 1.5, 1.0, 3.0], [1.0, 1.0, 1.0, 0.0])
            .rotary(0.2)
            .drive(0.1)
            .named("JAZZ ORGAN", ORGAN),
        fm(
            7,
            [
                (0.5, 1.0, H),
                (1.5, 1.0, H),
                (1.0, 1.0, H),
                (3.0, 0.6, 0.25),
            ],
            0.0,
            0.3,
        )
        .env(0.004, 1.0, 1.0, 0.05)
        .gain(0.75)
        .rotary(0.2)
        .drive(0.1)
        .named("PERC ORGAN", ORGAN),
        organ([0.5, 1.5, 1.0, 2.0], [1.0, 1.0, 1.0, 1.0])
            .rotary(0.9)
            .drive(0.25)
            .reverb(0.2)
            .named("GOSPEL ORGAN", ORGAN),
        organ([0.5, 1.5, 1.0, 4.0], [1.0, 1.0, 1.0, 0.6])
            .drive(0.65)
            .rotary(0.6)
            .named("ROCK ORGAN", ORGAN),
        organ([0.5, 1.0, 2.0, 4.0], [1.0, 1.0, 0.9, 0.8])
            .drive(0.2)
            .rotary(0.9)
            .named("FULL DRAWBARS", ORGAN),
        organ([0.5, 1.0, 2.0, 3.0], [0.9, 1.0, 0.6, 0.4])
            .env(0.08, 1.0, 1.0, 0.6)
            .reverb(0.8)
            .named("PIPE ORGAN", ORGAN),
        organ([1.0, 2.0, 1.0, 1.0], [1.0, 0.5, 0.0, 0.0])
            .gain(1.0)
            .rotary(0.15)
            .reverb(0.3)
            .named("SOFT FLUTES", ORGAN),
        analog(Square, Square, 12.0, 0.35)
            .env(0.003, 1.0, 1.0, 0.05)
            .filter(0.75, 0.0, 0.0)
            .gain(0.5)
            .vib(0.15)
            .reverb(0.15)
            .named("COMBO ORGAN", ORGAN),
        analog(Saw, Square, 0.12, 0.5)
            .env(0.06, 1.0, 1.0, 0.15)
            .filter(0.45, 0.0, 0.0)
            .gain(0.55)
            .chorus(0.2)
            .named("HARMONIUM", ORGAN),
        analog(Saw, Saw, 0.2, 0.5)
            .env(0.03, 1.0, 1.0, 0.08)
            .filter(0.55, 0.2, 0.0)
            .band()
            .gain(0.65)
            .chorus(0.3)
            .named("ACCORDION", ORGAN),
        analog(Saw, Saw, 0.15, 0.5)
            .env(0.25, 1.0, 1.0, 0.9)
            .filter(0.62, 0.0, 0.0)
            .gain(0.5)
            .chorus(1.0)
            .phaser(0.2)
            .reverb(0.35)
            .named("STRING MACHINE", STRINGS),
        analog(Saw, Saw, 0.08, 0.5)
            .env(0.35, 1.0, 1.0, 1.0)
            .filter(0.55, 0.1, 0.0)
            .gain(0.5)
            .chorus(0.5)
            .vib(0.05)
            .reverb(0.5)
            .named("POLY STRINGS", STRINGS),
        analog(Saw, Triangle, 0.05, 0.3)
            .env(0.08, 8.0, 0.0, 0.25)
            .filter(0.45, 0.0, 0.0)
            .gain(0.6)
            .vib(0.1)
            .reverb(0.3)
            .named("TAPE STRINGS", STRINGS),
        pluck(0.1, 0.5, 0.4, 0.7)
            .env(0.12, 1.0, 1.0, 0.3)
            .pitch(-12.0)
            .gain(1.3)
            .vib(0.2)
            .reverb(0.4)
            .named("CELLO", STRINGS),
        pluck(0.75, 0.35, 0.3, 0.0)
            .reverb(0.45)
            .named("PIZZICATO", STRINGS),
        analog(Saw, Saw, 0.1, 0.5)
            .env(0.06, 0.6, 0.7, 0.25)
            .filter(0.35, 0.15, 0.45)
            .gain(0.55)
            .chorus(0.3)
            .named("SYNTH BRASS", STRINGS),
        analog(Saw, Square, 0.08, 0.4)
            .env(0.005, 0.3, 0.4, 0.15)
            .filter(0.3, 0.2, 0.6)
            .gain(0.6)
            .reverb(0.25)
            .named("BRASS STAB", STRINGS),
        analog(Saw, Saw, 0.05, 0.5)
            .env(0.08, 1.0, 0.8, 0.2)
            .filter(0.4, 0.05, 0.3)
            .gain(0.55)
            .vib(0.1)
            .reverb(0.3)
            .named("HORN SECTION", STRINGS),
        analog(Saw, Triangle, 0.1, 0.5)
            .env(0.3, 1.0, 1.0, 0.8)
            .filter(0.55, 0.45, 0.0)
            .band()
            .gain(0.8)
            .chorus(0.8)
            .vib(0.08)
            .reverb(0.6)
            .named("CHOIR AAH", STRINGS),
        analog(Saw, Square, 0.07, 0.3)
            .env(0.6, 1.0, 1.0, 1.5)
            .filter(0.48, 0.1, 0.1)
            .gain(0.55)
            .chorus(0.9)
            .reverb(0.5)
            .named("WARM PAD", PAD),
        fm(
            3,
            [
                (1.0, 1.0, H),
                (1.0, 0.2, H),
                (2.0, 0.5, H),
                (7.0, 0.25, 3.0),
            ],
            0.0,
            0.4,
        )
        .env(0.4, 1.0, 1.0, 1.8)
        .gain(0.6)
        .chorus(0.4)
        .delay(0.3, 3)
        .reverb(0.6)
        .named("GLASS PAD", PAD),
        analog(Saw, Saw, 0.12, 0.5)
            .env(1.2, 3.0, 0.6, 2.0)
            .filter(0.2, 0.5, 0.6)
            .gain(0.55)
            .phaser(0.4)
            .reverb(0.5)
            .named("SWEEP PAD", PAD),
        analog(Saw, Triangle, -12.0, 0.4)
            .env(0.8, 1.0, 1.0, 2.0)
            .filter(0.3, 0.1, 0.0)
            .gain(1.2)
            .chorus(0.5)
            .reverb(0.7)
            .named("DARK PAD", PAD),
        morph(2.0, 0.25, 0.3, 0.0)
            .env(0.3, 1.0, 1.0, 1.2)
            .filter(0.7, 0.0, 0.0)
            .gain(0.5)
            .chorus(0.3)
            .delay(0.2, 3)
            .reverb(0.6)
            .named("SUPERSAW PAD", PAD),
        analog(Sine, Noise, 0.0, 0.15)
            .env(1.0, 1.0, 1.0, 2.0)
            .filter(0.7, 0.3, 0.0)
            .band()
            .gain(1.5)
            .chorus(0.4)
            .reverb(0.8)
            .named("AIR PAD", PAD),
        morph(1.0, 0.15, 0.0, 0.0)
            .env(0.8, 1.0, 1.0, 2.0)
            .gain(0.55)
            .phaser(0.3)
            .vib(0.05)
            .delay(0.5, 4)
            .reverb(0.8)
            .named("SPACE PAD", PAD),
        analog(Square, Saw, 0.1, 0.5)
            .env(0.4, 1.0, 1.0, 1.2)
            .filter(0.5, 0.5, 0.0)
            .band()
            .gain(0.8)
            .chorus(0.7)
            .reverb(0.5)
            .named("VOX PAD", PAD),
        analog(Saw, Saw, 0.08, 0.5)
            .env(0.005, 0.4, 0.8, 0.15)
            .filter(0.55, 0.3, 0.25)
            .gain(0.55)
            .drive(0.3)
            .vib(0.1)
            .delay(0.25, 2)
            .named("MINI LEAD", LEAD),
        analog(Square, Square, 0.05, 0.5)
            .env(0.005, 1.0, 1.0, 0.1)
            .filter(0.7, 0.1, 0.1)
            .gain(0.45)
            .vib(0.15)
            .delay(0.3, 2)
            .named("SQUARE LEAD", LEAD),
        analog(Saw, Square, 7.0, 0.4)
            .env(0.003, 0.5, 0.8, 0.12)
            .filter(0.6, 0.6, 0.3)
            .gain(0.5)
            .drive(0.6)
            .delay(0.2, 2)
            .named("SCREAMER", LEAD),
        morph(2.0, 0.3, 0.0, 0.1)
            .env(0.005, 0.5, 0.9, 0.3)
            .filter(0.8, 0.0, 0.0)
            .gain(0.5)
            .delay(0.35, 2)
            .reverb(0.4)
            .named("SUPERSAW", LEAD),
        morph(2.2, 0.45, 0.4, 0.3)
            .env(0.005, 0.6, 1.0, 0.3)
            .sweep(-5.0)
            .gain(0.5)
            .chorus(0.8)
            .phaser(0.3)
            .reverb(0.3)
            .named("HOOVER", LEAD),
        analog(Sine, Noise, 0.0, 0.04)
            .env(0.05, 1.0, 1.0, 0.15)
            .pitch(12.0)
            .gain(0.6)
            .vib(0.25)
            .reverb(0.35)
            .named("WHISTLE", LEAD),
        analog(Square, Square, 12.0, 0.25)
            .env(0.001, 0.1, 0.8, 0.05)
            .gain(0.4)
            .vib(0.3)
            .named("CHIP LEAD", LEAD),
        fm(
            0,
            [(1.0, 1.0, H), (1.0, 0.6, H), (2.0, 0.3, H), (1.0, 0.2, H)],
            0.4,
            0.6,
        )
        .env(0.01, 1.0, 1.0, 0.15)
        .gain(0.5)
        .vib(0.1)
        .delay(0.25, 2)
        .named("FM LEAD", LEAD),
        analog(Sine, Triangle, 0.0, 0.3)
            .env(0.03, 1.0, 1.0, 0.2)
            .gain(0.7)
            .vib(0.35)
            .delay(0.25, 2)
            .reverb(0.2)
            .named("SINE LEAD", LEAD),
        analog(Saw, Square, -12.0, 0.45)
            .env(0.002, 0.35, 0.55, 0.08)
            .filter(0.38, 0.35, 0.35)
            .pitch(-12.0)
            .gain(0.7)
            .drive(0.2)
            .named("MOOG BASS", BASS),
        analog(Sine, Triangle, 0.0, 0.1)
            .env(0.003, 1.0, 1.0, 0.08)
            .pitch(-12.0)
            .gain(0.9)
            .named("SUB BASS", BASS),
        analog(Saw, Saw, 0.3, 0.5)
            .env(0.003, 1.0, 1.0, 0.1)
            .filter(0.4, 0.2, 0.0)
            .pitch(-12.0)
            .gain(0.6)
            .drive(0.35)
            .chorus(0.2)
            .named("REESE BASS", BASS),
        fm(
            0,
            [
                (1.0, 1.0, 1.5),
                (1.0, 0.65, 0.35),
                (1.0, 0.0, 1.0),
                (1.0, 0.0, 1.0),
            ],
            0.3,
            0.8,
        )
        .env(0.001, 1.5, 0.3, 0.08)
        .pitch(-12.0)
        .gain(0.8)
        .named("FM BASS", BASS),
        analog(Saw, Saw, 0.0, 0.0)
            .env(0.002, 0.25, 0.3, 0.08)
            .filter(0.25, 0.85, 0.55)
            .pitch(-12.0)
            .gain(0.6)
            .drive(0.45)
            .delay(0.3, 2)
            .named("ACID BASS", BASS),
        pluck(0.55, 0.85, 0.5, 0.0)
            .env(0.001, 1.0, 1.0, 0.06)
            .filter(0.8, 0.2, 0.2)
            .pitch(-12.0)
            .gain(1.0)
            .drive(0.2)
            .named("SLAP BASS", BASS),
        pluck(0.4, 0.35, 0.3, 0.0)
            .env(0.001, 1.0, 1.0, 0.08)
            .filter(0.6, 0.0, 0.0)
            .pitch(-12.0)
            .gain(1.3)
            .named("FINGER BASS", BASS),
        analog(Square, Saw, -12.0, 0.3)
            .env(0.002, 0.25, 0.35, 0.1)
            .filter(0.3, 0.25, 0.45)
            .pitch(-12.0)
            .gain(0.65)
            .chorus(0.3)
            .named("SYNTH BASS", BASS),
        fm(
            5,
            [
                (1.0, 1.0, 3.5),
                (1.0, 0.15, 0.5),
                (4.0, 0.25, 0.6),
                (10.0, 0.08, 0.1),
            ],
            0.0,
            0.7,
        )
        .env(0.001, 4.0, 0.0, 0.4)
        .trem(0.6)
        .reverb(0.4)
        .named("VIBRAPHONE", MALLET),
        fm(
            5,
            [
                (1.0, 1.0, 0.6),
                (1.0, 0.2, 0.05),
                (4.0, 0.35, 0.15),
                (10.0, 0.05, 0.03),
            ],
            0.0,
            0.7,
        )
        .env(0.001, 1.2, 0.0, 0.15)
        .reverb(0.3)
        .named("MARIMBA", MALLET),
        fm(
            5,
            [
                (1.0, 1.0, 0.35),
                (3.0, 0.3, 0.04),
                (4.0, 0.4, 0.1),
                (10.0, 0.1, 0.03),
            ],
            0.0,
            0.7,
        )
        .env(0.001, 0.7, 0.0, 0.1)
        .pitch(12.0)
        .reverb(0.3)
        .named("XYLOPHONE", MALLET),
        fm(
            5,
            [
                (1.0, 1.0, 2.0),
                (3.5, 0.3, 0.3),
                (2.76, 0.4, 1.0),
                (5.4, 0.2, 0.6),
            ],
            0.0,
            0.6,
        )
        .env(0.001, 3.0, 0.0, 0.6)
        .pitch(12.0)
        .reverb(0.45)
        .named("GLOCKENSPIEL", MALLET),
        fm(
            3,
            [
                (1.0, 1.0, 1.6),
                (4.0, 0.35, 0.2),
                (2.0, 0.3, 0.8),
                (1.0, 0.1, 0.3),
            ],
            0.0,
            0.6,
        )
        .env(0.001, 2.0, 0.0, 0.4)
        .pitch(12.0)
        .reverb(0.4)
        .named("CELESTA", MALLET),
        fm(
            3,
            [
                (1.0, 1.0, 6.0),
                (3.5, 0.5, 4.0),
                (2.0, 0.5, 5.0),
                (5.5, 0.4, 2.0),
            ],
            0.0,
            0.6,
        )
        .env(0.001, 7.0, 0.0, 1.5)
        .reverb(0.5)
        .named("TUBULAR BELLS", MALLET),
        fm(
            5,
            [
                (1.0, 1.0, 1.8),
                (5.0, 0.2, 0.1),
                (4.0, 0.3, 0.8),
                (8.0, 0.1, 0.3),
            ],
            0.0,
            0.5,
        )
        .env(0.001, 2.5, 0.0, 0.5)
        .pitch(12.0)
        .delay(0.2, 3)
        .reverb(0.5)
        .named("MUSIC BOX", MALLET),
        fm(
            3,
            [
                (1.0, 1.0, 0.9),
                (1.0, 0.3, 0.08),
                (5.6, 0.25, 0.25),
                (1.0, 0.1, 0.1),
            ],
            0.0,
            0.7,
        )
        .env(0.001, 1.5, 0.0, 0.3)
        .reverb(0.3)
        .named("KALIMBA", MALLET),
        fm(
            5,
            [
                (1.0, 1.0, 1.2),
                (2.0, 0.35, 0.4),
                (3.0, 0.3, 0.5),
                (1.5, 0.2, 0.3),
            ],
            0.0,
            0.7,
        )
        .env(0.001, 1.5, 0.0, 0.3)
        .reverb(0.3)
        .named("STEEL DRUM", MALLET),
        fm(
            3,
            [
                (1.0, 1.0, 4.0),
                (3.5, 0.6, 3.0),
                (1.0, 0.5, 4.0),
                (14.0, 0.3, 0.5),
            ],
            0.0,
            0.6,
        )
        .fine(2, 7.0)
        .env(0.001, 5.0, 0.0, 1.2)
        .chorus(0.3)
        .delay(0.25, 3)
        .reverb(0.6)
        .named("FM BELLS", MALLET),
        pluck(0.45, 0.45, 0.3, 0.0)
            .reverb(0.25)
            .named("NYLON GUITAR", PLUCK),
        pluck(0.35, 0.7, 0.25, 0.0)
            .env(0.001, 1.0, 1.0, 0.3)
            .chorus(0.15)
            .reverb(0.2)
            .named("STEEL GUITAR", PLUCK),
        pluck(0.35, 0.75, 0.2, 0.0)
            .chorus(0.7)
            .reverb(0.3)
            .named("12 STRING", PLUCK),
        pluck(0.85, 0.5, 0.3, 0.0)
            .env(0.001, 1.0, 1.0, 0.04)
            .named("MUTED GUITAR", PLUCK),
        pluck(0.25, 0.5, 0.15, 0.0)
            .env(0.001, 1.0, 1.0, 1.0)
            .reverb(0.55)
            .named("HARP", PLUCK),
        pluck(0.45, 0.8, 0.1, 0.0)
            .vib(0.1)
            .delay(0.2, 3)
            .reverb(0.3)
            .named("KOTO", PLUCK),
        pluck(0.25, 1.0, 0.05, 0.0)
            .drive(0.15)
            .phaser(0.3)
            .reverb(0.3)
            .named("SITAR", PLUCK),
        pluck(0.6, 0.9, 0.6, 0.0)
            .env(0.001, 1.0, 1.0, 0.1)
            .named("BANJO", PLUCK),
        analog(Saw, Saw, 0.1, 0.5)
            .env(0.001, 0.3, 0.0, 0.3)
            .filter(0.3, 0.3, 0.55)
            .gain(0.6)
            .delay(0.35, 2)
            .reverb(0.3)
            .named("SYNTH PLUCK", PLUCK),
        morph(2.0, 0.2, 0.0, 0.0)
            .env(0.001, 0.25, 0.0, 0.25)
            .filter(0.3, 0.25, 0.6)
            .gain(0.55)
            .delay(0.5, 2)
            .reverb(0.5)
            .named("TRANCE PLUCK", PLUCK),
        analog(Square, Square, 0.0, 0.0)
            .env(0.001, 0.15, 0.0, 0.05)
            .gain(0.4)
            .delay(0.3, 1)
            .named("CHIP ARP", PLUCK),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    mod when_listing_patches {
        use super::*;

        #[test]
        fn fills_every_category() {
            for (c, name) in CATEGORIES.iter().enumerate() {
                assert!(in_category(c).len() >= 6, "{name}");
            }
        }

        #[test]
        fn gives_each_patch_a_unique_name() {
            for (i, p) in PATCHES.iter().enumerate() {
                assert_eq!(find(p.name), Some(i), "{}", p.name);
                assert_eq!(p.keys.patch, p.name);
            }
        }

        #[test]
        fn holds_notes_while_keys_are_down() {
            for p in PATCHES.iter() {
                assert_eq!(p.keys.sound.mode, Mode::Gate, "{}", p.name);
                assert!(p.keys.sound.source.is_synth(), "{}", p.name);
            }
        }
    }

    mod when_loading_an_old_project {
        use super::*;

        #[test]
        fn starts_on_the_first_patch() {
            let keys: Keys = serde_json::from_str("{}").unwrap();
            assert_eq!(keys.patch, PATCHES[0].name);
        }
    }
}
