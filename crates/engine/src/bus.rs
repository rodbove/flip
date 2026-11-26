use serde::{Deserialize, Serialize};

const DELAY_SECONDS: f32 = 3.2;
const DELAY_GLIDE: f32 = 0.0005;
const COMBS: [usize; 4] = [1116, 1188, 1277, 1356];
const ALLPASSES: [usize; 2] = [556, 441];
const SPREAD: usize = 23;
const FREEVERB_RATE: f32 = 44100.0;
const REVERB_GAIN: f32 = 0.03;
const CHORUS_SECONDS: f32 = 0.04;
const CHORUS_BASE: f32 = 0.012;
const CHORUS_DEPTH: f32 = 0.008;
const COMP_RATIO: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BusKind {
    #[default]
    Delay,
    Reverb,
    Drive,
    Chorus,
    Comp,
}

impl BusKind {
    pub const ALL: [BusKind; 5] = [
        BusKind::Delay,
        BusKind::Reverb,
        BusKind::Drive,
        BusKind::Chorus,
        BusKind::Comp,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BusFx {
    pub kind: BusKind,
    pub a: f32,
    pub b: f32,
}

impl Default for BusFx {
    fn default() -> Self {
        Self {
            kind: BusKind::Delay,
            a: 0.4,
            b: 0.4,
        }
    }
}

pub const DELAY_STEPS: [f32; 6] = [1.0, 2.0, 3.0, 4.0, 6.0, 8.0];

pub fn delay_steps(a: f32) -> f32 {
    let i = (a.clamp(0.0, 1.0) * (DELAY_STEPS.len() - 1) as f32).round() as usize;
    DELAY_STEPS[i]
}

struct Comb {
    buf: Vec<f32>,
    i: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)],
            i: 0,
            store: 0.0,
        }
    }

    fn run(&mut self, x: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buf[self.i];
        self.store = out * (1.0 - damp) + self.store * damp;
        self.buf[self.i] = x + self.store * feedback;
        self.i += 1;
        if self.i == self.buf.len() {
            self.i = 0;
        }
        out
    }
}

struct Allpass {
    buf: Vec<f32>,
    i: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)],
            i: 0,
        }
    }

    fn run(&mut self, x: f32) -> f32 {
        let held = self.buf[self.i];
        self.buf[self.i] = x + held * 0.5;
        self.i += 1;
        if self.i == self.buf.len() {
            self.i = 0;
        }
        held - x
    }
}

fn read(buf: &[f32], write: usize, back: f32) -> f32 {
    let n = buf.len();
    let back = back.clamp(1.0, (n - 2) as f32);
    let whole = back.floor();
    let t = back - whole;
    let a = buf[(write + n - whole as usize) % n];
    let b = buf[(write + n - whole as usize - 1) % n];
    a + (b - a) * t
}

pub struct Bus {
    rate: f32,
    delay: [Vec<f32>; 2],
    delay_pos: usize,
    delay_time: f32,
    delay_tone: [f32; 2],
    combs: [Vec<Comb>; 2],
    allpasses: [Vec<Allpass>; 2],
    chorus: [Vec<f32>; 2],
    chorus_pos: usize,
    lfo: f32,
    env: f32,
}

impl Bus {
    pub fn new(rate: u32) -> Self {
        let rate = rate as f32;
        let scale = rate / FREEVERB_RATE;
        let tuned = |n: usize, extra: usize| ((n + extra) as f32 * scale) as usize;
        let delay = (rate * DELAY_SECONDS) as usize;
        let chorus = (rate * CHORUS_SECONDS) as usize;
        Self {
            rate,
            delay: [vec![0.0; delay], vec![0.0; delay]],
            delay_pos: 0,
            delay_time: 0.0,
            delay_tone: [0.0; 2],
            combs: std::array::from_fn(|ch| {
                COMBS
                    .iter()
                    .map(|&n| Comb::new(tuned(n, ch * SPREAD)))
                    .collect()
            }),
            allpasses: std::array::from_fn(|ch| {
                ALLPASSES
                    .iter()
                    .map(|&n| Allpass::new(tuned(n, ch * SPREAD)))
                    .collect()
            }),
            chorus: [vec![0.0; chorus], vec![0.0; chorus]],
            chorus_pos: 0,
            lfo: 0.0,
            env: 0.0,
        }
    }

    pub fn run(&mut self, fx: &BusFx, step_samples: f32, x: [f32; 2]) -> [f32; 2] {
        let (a, b) = (fx.a.clamp(0.0, 1.0), fx.b.clamp(0.0, 1.0));
        match fx.kind {
            BusKind::Delay => self.delay(a, b, step_samples, x),
            BusKind::Reverb => self.reverb(a, b, x),
            BusKind::Drive => drive(a, b, x),
            BusKind::Chorus => self.chorus(a, b, x),
            BusKind::Comp => self.comp(a, b, x),
        }
    }

    fn delay(&mut self, a: f32, b: f32, step_samples: f32, x: [f32; 2]) -> [f32; 2] {
        let target = delay_steps(a) * step_samples;
        if self.delay_time <= 0.0 {
            self.delay_time = target;
        }
        self.delay_time += (target - self.delay_time) * DELAY_GLIDE;
        let feedback = b * 0.92;
        let n = self.delay[0].len();
        let left = read(&self.delay[0], self.delay_pos, self.delay_time);
        let right = read(&self.delay[1], self.delay_pos, self.delay_time);
        for (tone, v) in self.delay_tone.iter_mut().zip([left, right]) {
            *tone += (v - *tone) * 0.6;
        }
        let input = (x[0] + x[1]) * 0.5;
        self.delay[0][self.delay_pos] = input + self.delay_tone[1] * feedback;
        self.delay[1][self.delay_pos] = self.delay_tone[0] * feedback;
        self.delay_pos += 1;
        if self.delay_pos == n {
            self.delay_pos = 0;
        }
        [left, right]
    }

    fn reverb(&mut self, a: f32, b: f32, x: [f32; 2]) -> [f32; 2] {
        let feedback = 0.7 + a * 0.28;
        let damp = 0.05 + (1.0 - b) * 0.6;
        let input = (x[0] + x[1]) * REVERB_GAIN;
        std::array::from_fn(|ch| {
            let mut sum = 0.0;
            for c in &mut self.combs[ch] {
                sum += c.run(input, feedback, damp);
            }
            for p in &mut self.allpasses[ch] {
                sum = p.run(sum);
            }
            sum
        })
    }

    fn chorus(&mut self, a: f32, b: f32, x: [f32; 2]) -> [f32; 2] {
        let hz = 0.1 + a * a * 5.0;
        self.lfo = (self.lfo + hz / self.rate).fract();
        let n = self.chorus[0].len();
        for (line, v) in self.chorus.iter_mut().zip(x) {
            line[self.chorus_pos] = v;
        }
        let out = std::array::from_fn(|ch| {
            let phase = self.lfo + ch as f32 * 0.25;
            let wobble = (phase * std::f32::consts::TAU).sin();
            let back = (CHORUS_BASE + CHORUS_DEPTH * b * wobble) * self.rate;
            read(&self.chorus[ch], self.chorus_pos, back)
        });
        self.chorus_pos += 1;
        if self.chorus_pos == n {
            self.chorus_pos = 0;
        }
        out
    }

    fn comp(&mut self, a: f32, b: f32, x: [f32; 2]) -> [f32; 2] {
        let level = x[0].abs().max(x[1].abs());
        let attack = (-1.0 / (0.001 * self.rate)).exp();
        let release = (-1.0 / ((0.05 + b * 0.45) * self.rate)).exp();
        let k = if level > self.env { attack } else { release };
        self.env = level + (self.env - level) * k;
        let threshold = 0.5 * (1.0 - a * 0.95);
        let gain = if self.env > threshold {
            (threshold / self.env).powf(1.0 - 1.0 / COMP_RATIO)
        } else {
            1.0
        };
        let makeup = (1.0 / threshold).powf(0.6).min(8.0);
        [x[0] * gain * makeup, x[1] * gain * makeup]
    }
}

fn drive(a: f32, b: f32, x: [f32; 2]) -> [f32; 2] {
    let gain = 1.0 + a * 30.0;
    let levels = 2f32.powf(16.0 - b * 13.0);
    x.map(|v| ((v * gain).tanh() * levels).round() / levels * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48000;

    fn run(fx: BusFx, frames: usize, input: impl Fn(usize) -> f32) -> Vec<[f32; 2]> {
        let mut bus = Bus::new(RATE);
        (0..frames)
            .map(|i| {
                let v = input(i);
                bus.run(&fx, 6000.0, [v, v])
            })
            .collect()
    }

    mod when_delaying {
        use super::*;

        #[test]
        fn repeats_a_hit_after_the_synced_time() {
            let fx = BusFx {
                kind: BusKind::Delay,
                a: 0.0,
                b: 0.5,
            };
            let out = run(fx, 7000, |i| if i == 0 { 1.0 } else { 0.0 });
            let loudest = (0..out.len())
                .max_by(|&x, &y| out[x][0].abs().total_cmp(&out[y][0].abs()))
                .unwrap_or(0);
            assert!((5990..=6010).contains(&loudest));
        }

        #[test]
        fn picks_the_time_from_the_knob() {
            assert_eq!(delay_steps(0.0), 1.0);
            assert_eq!(delay_steps(1.0), 8.0);
            assert_eq!(delay_steps(0.4), 3.0);
        }
    }

    mod when_reverberating {
        use super::*;

        #[test]
        fn rings_after_the_input_stops() {
            let fx = BusFx {
                kind: BusKind::Reverb,
                a: 0.8,
                b: 0.5,
            };
            let out = run(fx, 24000, |i| if i < 100 { 0.5 } else { 0.0 });
            assert!(out[12000..].iter().any(|f| f[0].abs() > 1e-3));
        }
    }

    mod when_compressing {
        use super::*;

        #[test]
        fn evens_out_loud_and_soft_input() {
            let fx = BusFx {
                kind: BusKind::Comp,
                a: 0.8,
                b: 0.2,
            };
            let loud = run(fx, 4800, |_| 0.8);
            let soft = run(fx, 4800, |_| 0.1);
            let ratio = loud[4799][0] / soft[4799][0];
            assert!(ratio < 4.0);
        }
    }

    mod when_driving {
        use super::*;

        #[test]
        fn stays_within_range() {
            let out = drive(1.0, 1.0, [5.0, -5.0]);
            assert!(out.iter().all(|v| v.abs() <= 0.5));
        }
    }
}
