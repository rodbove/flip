use std::f32::consts::{PI, TAU};

use serde::{Deserialize, Serialize};

pub const TIMES: [f32; 6] = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0];
pub const TIME_NAMES: [&str; 6] = ["1/16", "1/8", "3/16", "1/4", "3/8", "1/2"];

const ECHO_SECONDS: f32 = 3.2;
const CHORUS_SECONDS: f32 = 0.03;
const HORN_SECONDS: f32 = 0.004;
const COMBS: [usize; 4] = [1116, 1188, 1277, 1356];
const ALLPASSES: [usize; 2] = [556, 441];
const SPREAD: usize = 23;
const ROOM: f32 = 0.86;
const DAMP: f32 = 0.3;
const REVERB_IN: f32 = 0.03;
const ECHO_FEEDBACK: f32 = 0.45;
const PHASER_STAGES: usize = 4;
const CROSSOVER_HZ: f32 = 800.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fx {
    pub drive: f32,
    pub phaser: f32,
    pub chorus: f32,
    pub rotary: f32,
    pub tremolo: f32,
    pub delay: f32,
    pub time: u8,
    pub reverb: f32,
}

impl Default for Fx {
    fn default() -> Self {
        Self {
            drive: 0.0,
            phaser: 0.0,
            chorus: 0.0,
            rotary: 0.0,
            tremolo: 0.0,
            delay: 0.0,
            time: 2,
            reverb: 0.0,
        }
    }
}

impl Fx {
    pub fn dry(&self) -> bool {
        self.drive <= 0.0
            && self.phaser <= 0.0
            && self.chorus <= 0.0
            && self.rotary <= 0.0
            && self.tremolo <= 0.0
            && self.delay <= 0.0
            && self.reverb <= 0.0
    }

    pub fn beats(&self) -> f32 {
        TIMES[(self.time as usize).min(TIMES.len() - 1)]
    }
}

struct Line {
    buf: Box<[f32]>,
    write: usize,
}

impl Line {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(4)].into_boxed_slice(),
            write: 0,
        }
    }

    fn push(&mut self, x: f32) {
        self.buf[self.write] = x;
        self.write = (self.write + 1) % self.buf.len();
    }

    fn read(&self, delay: f32) -> f32 {
        let n = self.buf.len();
        let d = delay.clamp(1.0, (n - 2) as f32);
        let pos = self.write as f32 - d + n as f32;
        let i = pos.floor() as usize;
        let t = pos - pos.floor();
        let a = self.buf[i % n];
        let b = self.buf[(i + 1) % n];
        a + (b - a) * t
    }
}

struct Comb {
    buf: Box<[f32]>,
    at: usize,
    low: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)].into_boxed_slice(),
            at: 0,
            low: 0.0,
        }
    }

    fn run(&mut self, x: f32) -> f32 {
        let y = self.buf[self.at];
        self.low = y * (1.0 - DAMP) + self.low * DAMP;
        self.buf[self.at] = x + self.low * ROOM;
        self.at = (self.at + 1) % self.buf.len();
        y
    }
}

struct Allpass {
    buf: Box<[f32]>,
    at: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)].into_boxed_slice(),
            at: 0,
        }
    }

    fn run(&mut self, x: f32) -> f32 {
        let held = self.buf[self.at];
        self.buf[self.at] = x + held * 0.5;
        self.at = (self.at + 1) % self.buf.len();
        held - x
    }
}

#[derive(Clone, Copy, Default)]
struct Stage {
    x: f32,
    y: f32,
}

pub struct Rack {
    rate: f32,
    chorus: [Line; 2],
    echo: [Line; 2],
    horn: Line,
    combs: [Vec<Comb>; 2],
    allpasses: [Vec<Allpass>; 2],
    phaser: [[Stage; PHASER_STAGES]; 2],
    phaser_last: [f32; 2],
    crossover: f32,
    lfo: [f32; 5],
}

impl Rack {
    pub fn new(rate: f32) -> Self {
        let scale = rate / 44100.0;
        let len = |n: usize| (n as f32 * scale) as usize;
        let reverb = |offset: usize| -> (Vec<Comb>, Vec<Allpass>) {
            (
                COMBS.iter().map(|n| Comb::new(len(n + offset))).collect(),
                ALLPASSES
                    .iter()
                    .map(|n| Allpass::new(len(n + offset)))
                    .collect(),
            )
        };
        let (cl, al) = reverb(0);
        let (cr, ar) = reverb(SPREAD);
        let line = |seconds: f32| Line::new((seconds * rate) as usize + 4);
        Self {
            rate,
            chorus: [line(CHORUS_SECONDS), line(CHORUS_SECONDS)],
            echo: [line(ECHO_SECONDS), line(ECHO_SECONDS)],
            horn: line(HORN_SECONDS),
            combs: [cl, cr],
            allpasses: [al, ar],
            phaser: [[Stage::default(); PHASER_STAGES]; 2],
            phaser_last: [0.0; 2],
            crossover: 0.0,
            lfo: [0.0, 0.25, 0.0, 0.0, 0.0],
        }
    }

    fn tick(&mut self, i: usize, hz: f32) -> f32 {
        self.lfo[i] = (self.lfo[i] + hz / self.rate).fract();
        (self.lfo[i] * TAU).sin()
    }

    pub fn run(&mut self, fx: &Fx, x: [f32; 2], tempo: f32) -> [f32; 2] {
        let mut s = x;
        if fx.drive > 0.0 {
            let k = 1.0 + fx.drive * 10.0;
            let makeup = (1.0 + fx.drive) / k;
            s = s.map(|v| (v * k).tanh() * makeup);
        }
        if fx.phaser > 0.0 {
            s = self.phaser(fx.phaser, s);
        }
        if fx.chorus > 0.0 {
            s = self.chorus(fx.chorus, s);
        }
        if fx.rotary > 0.0 {
            s = self.rotary(fx.rotary, s);
        }
        if fx.tremolo > 0.0 {
            let m = self.tick(3, 4.5);
            let t = fx.tremolo.min(1.0);
            s = [
                s[0] * (1.0 - t * 0.5 * (1.0 + m)),
                s[1] * (1.0 - t * 0.5 * (1.0 - m)),
            ];
        }
        if fx.delay > 0.0 {
            let samples = 60.0 / tempo.max(1.0) * fx.beats() * self.rate;
            let l = self.echo[0].read(samples);
            let r = self.echo[1].read(samples);
            self.echo[0].push(s[0] + r * ECHO_FEEDBACK);
            self.echo[1].push(s[1] + l * ECHO_FEEDBACK);
            let wet = fx.delay * 0.6;
            s = [s[0] + l * wet, s[1] + r * wet];
        }
        if fx.reverb > 0.0 {
            let input = (s[0] + s[1]) * REVERB_IN;
            let mut wet = [0.0f32; 2];
            for (ch, w) in wet.iter_mut().enumerate() {
                let mut acc: f32 = self.combs[ch].iter_mut().map(|c| c.run(input)).sum();
                for a in self.allpasses[ch].iter_mut() {
                    acc = a.run(acc);
                }
                *w = acc;
            }
            let dry = 1.0 - fx.reverb * 0.3;
            s = [
                s[0] * dry + wet[0] * fx.reverb,
                s[1] * dry + wet[1] * fx.reverb,
            ];
        }
        s
    }

    fn phaser(&mut self, amount: f32, x: [f32; 2]) -> [f32; 2] {
        let m = self.tick(0, 0.35);
        let mut out = x;
        for (ch, o) in out.iter_mut().enumerate() {
            let sweep = if ch == 0 { m } else { -m };
            let hz = 300.0 * 8f32.powf(0.5 + 0.5 * sweep);
            let t = (PI * hz / self.rate).tan();
            let a = (t - 1.0) / (t + 1.0);
            let mut v = x[ch] + self.phaser_last[ch] * 0.5 * amount;
            for st in self.phaser[ch].iter_mut() {
                let y = a * v + st.x - a * st.y;
                st.x = v;
                st.y = y;
                v = y;
            }
            self.phaser_last[ch] = v;
            *o = x[ch] * (1.0 - 0.5 * amount) + v * 0.5 * amount;
        }
        out
    }

    fn chorus(&mut self, amount: f32, x: [f32; 2]) -> [f32; 2] {
        let m = self.tick(1, 0.6);
        let base = 0.007 * self.rate;
        let depth = 0.0035 * self.rate;
        let mut out = x;
        for (ch, o) in out.iter_mut().enumerate() {
            self.chorus[ch].push(x[ch]);
            let sweep = if ch == 0 { m } else { -m };
            let wet = self.chorus[ch].read(base + depth * (0.5 + 0.5 * sweep));
            *o = x[ch] * (1.0 - 0.35 * amount) + wet * 0.7 * amount;
        }
        out
    }

    fn rotary(&mut self, amount: f32, x: [f32; 2]) -> [f32; 2] {
        let mono = (x[0] + x[1]) * 0.5;
        let k = 1.0 - (-TAU * CROSSOVER_HZ / self.rate).exp();
        self.crossover += (mono - self.crossover) * k;
        let low = self.crossover;
        let high = mono - low;
        let speed = 0.8 + 6.0 * amount.clamp(0.0, 1.0).powi(2);
        let horn = self.tick(2, speed);
        let drum = self.tick(4, speed * 0.85);
        self.horn.push(high);
        let doppler = self
            .horn
            .read(1.0 + 0.0012 * self.rate * (0.5 + 0.5 * horn));
        let horn_gain = 1.0 - 0.45 * (0.5 + 0.5 * horn);
        let drum_gain = 1.0 - 0.2 * (0.5 + 0.5 * drum);
        let pan = (horn + 1.0) * PI / 4.0;
        let h = doppler * horn_gain;
        let d = low * drum_gain;
        let wet = [h * pan.cos() * 1.4 + d, h * pan.sin() * 1.4 + d];
        let mix = 0.5 + 0.5 * amount.min(1.0);
        [
            x[0] * (1.0 - mix) + wet[0] * mix,
            x[1] * (1.0 - mix) + wet[1] * mix,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48000.0;

    fn impulse(fx: &Fx, frames: usize) -> Vec<[f32; 2]> {
        let mut rack = Rack::new(RATE);
        (0..frames)
            .map(|i| {
                let x = if i == 0 { 1.0 } else { 0.0 };
                rack.run(fx, [x, x], 120.0)
            })
            .collect()
    }

    fn energy(frames: &[[f32; 2]]) -> f32 {
        frames.iter().map(|f| f[0] * f[0] + f[1] * f[1]).sum()
    }

    mod when_every_effect_is_off {
        use super::*;

        #[test]
        fn passes_the_sound_through() {
            let fx = Fx::default();
            assert!(fx.dry());
            let out = impulse(&fx, 4);
            assert_eq!(out[0], [1.0, 1.0]);
            assert_eq!(out[1], [0.0, 0.0]);
        }
    }

    mod when_the_delay_is_on {
        use super::*;

        #[test]
        fn repeats_the_sound_on_the_beat() {
            let fx = Fx {
                delay: 1.0,
                time: 3,
                ..Fx::default()
            };
            let beat = (RATE * 0.5) as usize;
            let out = impulse(&fx, beat + 10);
            let echo = out[beat - 2..beat + 2]
                .iter()
                .fold(0.0f32, |a, f| a.max(f[0].abs()));
            assert!(echo > 0.3, "{echo}");
            assert!(out[beat / 2][0].abs() < 1e-6);
        }
    }

    mod when_the_reverb_is_on {
        use super::*;

        #[test]
        fn leaves_a_tail() {
            let fx = Fx {
                reverb: 0.8,
                ..Fx::default()
            };
            let out = impulse(&fx, 48000);
            assert!(energy(&out[24000..]) > 1e-4);
        }
    }

    mod when_any_effect_runs_for_a_while {
        use super::*;

        #[test]
        fn stays_finite_and_bounded() {
            let fx = Fx {
                drive: 1.0,
                phaser: 1.0,
                chorus: 1.0,
                rotary: 1.0,
                tremolo: 1.0,
                delay: 1.0,
                time: 0,
                reverb: 1.0,
            };
            let mut rack = Rack::new(RATE);
            let mut peak = 0.0f32;
            for i in 0..96000 {
                let x = (i as f32 * 220.0 * TAU / RATE).sin() * 0.5;
                let y = rack.run(&fx, [x, x], 120.0);
                assert!(y[0].is_finite() && y[1].is_finite());
                peak = peak.max(y[0].abs()).max(y[1].abs());
            }
            assert!(peak < 6.0, "{peak}");
        }
    }
}
