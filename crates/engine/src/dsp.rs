use std::f32::consts::{PI, TAU};

use crate::project::{Filter, Wave};

const MIN_HZ: f32 = 20.0;
const SPAN: f32 = 1000.0;
const SETTLE: f32 = 5.0;
const SILENT: f32 = 1e-4;

pub fn cutoff_hz(c: f32) -> f32 {
    MIN_HZ * SPAN.powf(c.clamp(0.0, 1.0))
}

pub fn semis(st: f32) -> f32 {
    2f32.powf(st / 12.0)
}

pub fn pan(p: f32) -> [f32; 2] {
    let a = (p.clamp(-1.0, 1.0) + 1.0) * PI / 4.0;
    [a.cos(), a.sin()]
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Coefs {
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Coefs {
    pub fn new(hz: f32, resonance: f32, rate: f32) -> Self {
        let g = (PI * hz.min(rate * 0.49) / rate).tan();
        let k = 2.0 - 1.96 * resonance.clamp(0.0, 1.0);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        Self {
            a1,
            a2,
            a3: g * a2,
            k,
        }
    }
}

impl Svf {
    pub fn run(&mut self, x: f32, c: &Coefs, kind: Filter) -> f32 {
        let v3 = x - self.ic2;
        let v1 = c.a1 * self.ic1 + c.a2 * v3;
        let v2 = self.ic2 + c.a2 * self.ic1 + c.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        match kind {
            Filter::Low => v2,
            Filter::High => x - c.k * v1 - v2,
            Filter::Band => v1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Stage {
    Attack,
    Decay,
    Sustain,
    Release,
    #[default]
    Done,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Env {
    pub stage: Stage,
    pub level: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Env {
    pub fn start(&mut self) {
        self.stage = Stage::Attack;
    }

    pub fn release(&mut self) {
        if self.stage != Stage::Done {
            self.stage = Stage::Release;
        }
    }

    pub fn done(&self) -> bool {
        self.stage == Stage::Done
    }

    pub fn next(&mut self, s: &Shape, dt: f32) -> f32 {
        let fall = |time: f32| (-SETTLE * dt / time.max(1e-3)).exp();
        match self.stage {
            Stage::Attack => {
                self.level += dt / s.attack.max(1e-3);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level = s.sustain + (self.level - s.sustain) * fall(s.decay);
                if (self.level - s.sustain).abs() < SILENT {
                    self.level = s.sustain;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => {
                self.level = s.sustain;
                if s.sustain < SILENT {
                    self.stage = Stage::Done;
                }
            }
            Stage::Release => {
                self.level *= fall(s.release);
                if self.level < SILENT {
                    self.level = 0.0;
                    self.stage = Stage::Done;
                }
            }
            Stage::Done => self.level = 0.0,
        }
        self.level
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rng(u32);

impl Default for Rng {
    fn default() -> Self {
        Self(0x9e37_79b9)
    }
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }

    pub fn bits(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    pub fn unit(&mut self) -> f32 {
        (self.bits() >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn noise(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

fn blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        2.0 * t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + 2.0 * t + 1.0
    } else {
        0.0
    }
}

pub fn osc(wave: Wave, phase: f32, dt: f32, rng: &mut Rng) -> f32 {
    match wave {
        Wave::Sine => (phase * TAU).sin(),
        Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
        Wave::Saw => 2.0 * phase - 1.0 - blep(phase, dt),
        Wave::Square => {
            let naive = if phase < 0.5 { 1.0 } else { -1.0 };
            naive + blep(phase, dt) - blep((phase + 0.5).fract(), dt)
        }
        Wave::Noise => rng.noise(),
    }
}

pub fn shape(wave: Wave, phase: f32) -> f32 {
    match wave {
        Wave::Sine => (phase * TAU).sin(),
        Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
        Wave::Saw => 2.0 * phase - 1.0,
        Wave::Square => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Wave::Noise => ((phase * 7919.0).sin() * 43758.547).fract() * 2.0 - 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48000.0;

    fn tone(hz: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (i as f32 * hz * TAU / RATE).sin()).collect()
    }

    fn level(filter: Filter, cutoff: f32, hz: f32) -> f32 {
        let c = Coefs::new(cutoff, 0.0, RATE);
        let mut f = Svf::default();
        let out: Vec<f32> = tone(hz, 9600)
            .into_iter()
            .map(|x| f.run(x, &c, filter))
            .collect();
        out[4800..].iter().fold(0.0, |a, x| a.max(x.abs()))
    }

    mod when_filtering {
        use super::*;

        #[test]
        fn low_pass_cuts_highs() {
            assert!(level(Filter::Low, 500.0, 100.0) > 0.9);
            assert!(level(Filter::Low, 500.0, 8000.0) < 0.05);
        }

        #[test]
        fn high_pass_cuts_lows() {
            assert!(level(Filter::High, 2000.0, 100.0) < 0.05);
            assert!(level(Filter::High, 2000.0, 10000.0) > 0.9);
        }
    }

    mod when_enveloping {
        use super::*;

        const SHAPE: Shape = Shape {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.5,
            release: 0.1,
        };

        #[test]
        fn rises_then_settles_on_sustain() {
            let mut env = Env::default();
            env.start();
            let dt = 1.0 / RATE;
            let peak = (0..480).map(|_| env.next(&SHAPE, dt)).fold(0.0, f32::max);
            assert!(peak > 0.99);
            for _ in 0..48000 {
                env.next(&SHAPE, dt);
            }
            assert_eq!(env.stage, Stage::Sustain);
            assert_eq!(env.level, 0.5);
        }

        #[test]
        fn ends_after_release() {
            let mut env = Env::default();
            env.start();
            let dt = 1.0 / RATE;
            for _ in 0..4800 {
                env.next(&SHAPE, dt);
            }
            env.release();
            for _ in 0..9600 {
                env.next(&SHAPE, dt);
            }
            assert!(env.done());
        }

        #[test]
        fn ends_without_sustain() {
            let mut env = Env::default();
            env.start();
            let shape = Shape {
                sustain: 0.0,
                ..SHAPE
            };
            for _ in 0..48000 {
                env.next(&shape, 1.0 / RATE);
            }
            assert!(env.done());
        }
    }

    mod when_panning {
        use super::*;

        #[test]
        fn keeps_power_in_the_middle() {
            let [l, r] = pan(0.0);
            assert!((l * l + r * r - 1.0).abs() < 1e-5);
            assert!(pan(-1.0)[1].abs() < 1e-6);
        }
    }
}
