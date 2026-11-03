use crate::dsp::{self, Coefs, Svf};
use crate::project::Filter;

pub const PUNCHES: [&str; 8] = [
    "LOOP 1/4",
    "LOOP 1/8",
    "LOOP 1/16",
    "LOOP 1/32",
    "ECHO",
    "LOW SWEEP",
    "HIGH SWEEP",
    "REVERSE",
];

const HISTORY_SECONDS: f32 = 2.0;
const ECHO_SECONDS: f32 = 1.5;
const ECHO_STEPS: f32 = 3.0;
const ECHO_FEEDBACK: f32 = 0.55;
const FADE_SECONDS: f32 = 0.004;
const SWEEP_STEPS: f32 = 16.0;
const COEF_EVERY: usize = 32;

const LOOP: usize = 0;
const ECHO: usize = 4;
const LOW: usize = 5;
const HIGH: usize = 6;
const REVERSE: usize = 7;

pub struct Punch {
    rate: f32,
    history: [Vec<f32>; 2],
    write: usize,
    held: Option<usize>,
    kind: usize,
    n: usize,
    anchor: usize,
    span: usize,
    mix: f32,
    filters: [Svf; 2],
    coefs: Option<Coefs>,
    echo: [Vec<f32>; 2],
    echo_pos: usize,
}

impl Punch {
    pub fn new(rate: u32) -> Self {
        let rate = rate as f32;
        let history = (rate * HISTORY_SECONDS) as usize;
        let echo = (rate * ECHO_SECONDS) as usize;
        Self {
            rate,
            history: [vec![0.0; history], vec![0.0; history]],
            write: 0,
            held: None,
            kind: 0,
            n: 0,
            anchor: 0,
            span: 1,
            mix: 0.0,
            filters: [Svf::default(); 2],
            coefs: None,
            echo: [vec![0.0; echo], vec![0.0; echo]],
            echo_pos: 0,
        }
    }

    pub fn held(&self) -> Option<usize> {
        self.held
    }

    pub fn set(&mut self, punch: Option<usize>, step_samples: f32) {
        let punch = punch.filter(|&p| p < PUNCHES.len());
        if punch == self.held {
            return;
        }
        self.held = punch;
        let Some(kind) = punch else {
            return;
        };
        self.kind = kind;
        self.n = 0;
        self.anchor = self.write;
        self.filters = [Svf::default(); 2];
        self.coefs = None;
        let steps = match kind {
            k if k < ECHO => 4.0 / (1 << (k - LOOP)) as f32,
            REVERSE => 4.0,
            _ => 1.0,
        };
        let most = self.history[0].len() - 1;
        self.span = ((steps * step_samples) as usize).clamp(1, most);
    }

    fn at(&self, offset: isize) -> [f32; 2] {
        let n = self.history[0].len() as isize;
        let i = (self.anchor as isize + offset).rem_euclid(n) as usize;
        [self.history[0][i], self.history[1][i]]
    }

    fn window(&self, pos: usize) -> f32 {
        let fade = (FADE_SECONDS * self.rate).max(1.0);
        let edge = pos.min(self.span - pos) as f32;
        (edge / fade).min(1.0)
    }

    fn wet(&mut self, x: [f32; 2], step_samples: f32) -> [f32; 2] {
        match self.kind {
            k if k < ECHO => {
                if self.n < self.span {
                    return x;
                }
                let pos = self.n % self.span;
                let g = self.window(pos);
                self.at(pos as isize).map(|v| v * g)
            }
            REVERSE => {
                let pos = self.n % self.span;
                let g = self.window(pos);
                self.at(-1 - pos as isize).map(|v| v * g)
            }
            LOW | HIGH => {
                if self.coefs.is_none() || self.n.is_multiple_of(COEF_EVERY) {
                    let t = (self.n as f32 / (SWEEP_STEPS * step_samples)).min(1.0);
                    let c = if self.kind == LOW {
                        1.0 - t * 0.8
                    } else {
                        t * 0.7
                    };
                    self.coefs = Some(Coefs::new(dsp::cutoff_hz(c), 0.5, self.rate));
                }
                let kind = if self.kind == LOW {
                    Filter::Low
                } else {
                    Filter::High
                };
                match &self.coefs {
                    Some(c) => [
                        self.filters[0].run(x[0], c, kind),
                        self.filters[1].run(x[1], c, kind),
                    ],
                    None => x,
                }
            }
            _ => x,
        }
    }

    pub fn run(&mut self, x: [f32; 2], step_samples: f32) -> [f32; 2] {
        let len = self.history[0].len();
        self.history[0][self.write] = x[0];
        self.history[1][self.write] = x[1];
        self.write = (self.write + 1) % len;
        let target = if self.held.is_some() { 1.0 } else { 0.0 };
        let fade = 1.0 / (FADE_SECONDS * self.rate).max(1.0);
        self.mix = if self.mix < target {
            (self.mix + fade).min(target)
        } else {
            (self.mix - fade).max(target)
        };
        let out = if self.mix > 0.0 {
            let wet = self.wet(x, step_samples);
            self.n += 1;
            [
                x[0] + (wet[0] - x[0]) * self.mix,
                x[1] + (wet[1] - x[1]) * self.mix,
            ]
        } else {
            x
        };
        let echo = self.echo(x, step_samples);
        [out[0] + echo[0], out[1] + echo[1]]
    }

    fn echo(&mut self, x: [f32; 2], step_samples: f32) -> [f32; 2] {
        let len = self.echo[0].len();
        let back = ((ECHO_STEPS * step_samples) as usize).clamp(1, len - 1);
        let read = (self.echo_pos + len - back) % len;
        let delayed = [self.echo[0][read], self.echo[1][read]];
        let input = if self.held == Some(ECHO) { x } else { [0.0; 2] };
        self.echo[0][self.echo_pos] = input[0] + delayed[1] * ECHO_FEEDBACK;
        self.echo[1][self.echo_pos] = input[1] + delayed[0] * ECHO_FEEDBACK;
        self.echo_pos = (self.echo_pos + 1) % len;
        delayed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48000;
    const STEP: f32 = 1000.0;

    fn ramp(i: usize) -> [f32; 2] {
        let v = (i % 5000) as f32 / 5000.0;
        [v, v]
    }

    mod when_looping {
        use super::*;

        #[test]
        fn repeats_the_slice_from_the_press() {
            let mut p = Punch::new(RATE);
            for i in 0..3000 {
                p.run(ramp(i), STEP);
            }
            p.set(Some(2), STEP);
            let out: Vec<[f32; 2]> = (3000..6000).map(|i| p.run(ramp(i), STEP)).collect();
            assert!((out[1500][0] - ramp(3500)[0]).abs() < 1e-4);
            assert!((out[2500][0] - ramp(3500)[0]).abs() < 1e-4);
        }

        #[test]
        fn returns_to_the_live_sound_on_release() {
            let mut p = Punch::new(RATE);
            p.set(Some(2), STEP);
            for i in 0..3000 {
                p.run(ramp(i), STEP);
            }
            p.set(None, STEP);
            let out: Vec<[f32; 2]> = (3000..4000).map(|i| p.run(ramp(i), STEP)).collect();
            assert!((out[999][0] - ramp(3999)[0]).abs() < 1e-4);
        }
    }

    mod when_reversing {
        use super::*;

        #[test]
        fn plays_the_last_beat_backwards() {
            let mut p = Punch::new(RATE);
            for i in 0..4000 {
                p.run(ramp(i), STEP);
            }
            p.set(Some(REVERSE), STEP);
            let out: Vec<[f32; 2]> = (4000..5000).map(|i| p.run(ramp(i), STEP)).collect();
            assert!(out[600][0] < out[400][0]);
        }
    }

    mod when_echoing {
        use super::*;

        #[test]
        fn keeps_the_tail_after_release() {
            let mut p = Punch::new(RATE);
            p.set(Some(ECHO), STEP);
            p.run([1.0, 1.0], STEP);
            p.set(None, STEP);
            let out: Vec<[f32; 2]> = (0..4000).map(|_| p.run([0.0; 2], STEP)).collect();
            assert!(out[2999][0] > 0.5);
        }
    }
}
