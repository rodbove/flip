use std::f32::consts::TAU;

use crate::dsp;
use crate::project::{ALGORITHMS, Fm, OPERATORS};

pub const ROUTES: [([u8; OPERATORS], u8); ALGORITHMS] = [
    ([0b0010, 0b0100, 0b1000, 0], 0b0001),
    ([0b0010, 0b1100, 0, 0], 0b0001),
    ([0b0110, 0, 0b1000, 0], 0b0001),
    ([0b0010, 0, 0b1000, 0], 0b0101),
    ([0b1000, 0b1000, 0b1000, 0], 0b0111),
    ([0b0010, 0, 0, 0], 0b1101),
    ([0, 0, 0b1000, 0], 0b0111),
    ([0, 0, 0, 0], 0b1111),
];

pub const HOLD: f32 = 9.9;

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    phase: [f32; OPERATORS],
    env: [f32; OPERATORS],
    feedback: [f32; 2],
}

impl State {
    pub fn start() -> Self {
        Self {
            env: [1.0; OPERATORS],
            ..Self::default()
        }
    }
}

fn route(algorithm: u8) -> ([u8; OPERATORS], u8) {
    ROUTES[(algorithm as usize).min(ALGORITHMS - 1)]
}

pub fn carriers(algorithm: u8) -> u8 {
    route(algorithm).1
}

pub fn next(fm: &Fm, s: &mut State, hz: f32, velocity: f32, dt: f32) -> f32 {
    let (mods, carrier) = route(fm.algorithm);
    let touch = 1.0 - fm.velocity.clamp(0.0, 1.0) * (1.0 - velocity.clamp(0.0, 1.0));
    let mut out = [0.0f32; OPERATORS];
    let mut sum = 0.0;
    let mut count = 0;
    for k in (0..OPERATORS).rev() {
        let op = &fm.ops[k];
        let mut input: f32 = (0..OPERATORS)
            .filter(|j| mods[k] & (1 << j) != 0)
            .map(|j| out[j])
            .sum();
        if k == OPERATORS - 1 {
            input += fm.feedback * 0.5 * (s.feedback[0] + s.feedback[1]);
        }
        let value = ((s.phase[k] + input) * TAU).sin() * op.level * s.env[k];
        if k == OPERATORS - 1 {
            s.feedback = [value, s.feedback[0]];
        }
        if carrier & (1 << k) != 0 {
            out[k] = value;
            sum += value;
            count += 1;
        } else {
            out[k] = value * fm.depth * touch;
        }
        let freq = hz * op.ratio * dsp::semis(op.fine / 100.0);
        s.phase[k] = (s.phase[k] + freq * dt).fract();
        if op.decay < HOLD {
            s.env[k] *= (-dt / op.decay.max(1e-3)).exp();
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / (count as f32).sqrt()
    }
}

pub fn preview(fm: &Fm, n: usize) -> Vec<f32> {
    let mut state = State::start();
    let hz = n as f32 / 2.0;
    let dt = 1.0 / (n as f32 * n as f32);
    let mut values: Vec<f32> = (0..n).map(|_| next(fm, &mut state, hz, 1.0, dt)).collect();
    super::peak_normalize(&mut values);
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Op;

    const RATE: f32 = 48000.0;

    fn render(fm: &Fm, frames: usize) -> Vec<f32> {
        let mut s = State::start();
        (0..frames)
            .map(|_| next(fm, &mut s, 220.0, 1.0, 1.0 / RATE))
            .collect()
    }

    fn crossings(values: &[f32]) -> usize {
        values
            .windows(2)
            .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
            .count()
    }

    mod when_routing_operators {
        use super::*;

        #[test]
        fn every_algorithm_has_a_carrier() {
            for a in 0..ALGORITHMS as u8 {
                assert_ne!(carriers(a), 0, "algorithm {a}");
            }
        }

        #[test]
        fn modulators_always_sit_above_their_targets() {
            for (mods, _) in ROUTES {
                for (k, mask) in mods.iter().enumerate() {
                    assert_eq!(mask & ((1 << (k + 1)) - 1), 0);
                }
            }
        }
    }

    mod when_a_modulator_is_silent {
        use super::*;

        #[test]
        fn the_carrier_is_a_plain_sine() {
            let fm = Fm {
                ops: [
                    Op {
                        level: 1.0,
                        decay: HOLD,
                        ..Op::default()
                    },
                    Op::default(),
                    Op::default(),
                    Op::default(),
                ],
                ..Fm::default()
            };
            let out = render(&fm, 48000);
            assert!((crossings(&out) as i32 - 440).abs() <= 2);
        }
    }

    mod when_a_modulator_plays {
        use super::*;

        #[test]
        fn adds_harmonics() {
            let plain = Fm {
                ops: [
                    Op {
                        level: 1.0,
                        decay: HOLD,
                        ..Op::default()
                    },
                    Op::default(),
                    Op::default(),
                    Op::default(),
                ],
                ..Fm::default()
            };
            let mut rich = plain;
            rich.ops[1] = Op {
                ratio: 3.0,
                level: 1.0,
                decay: HOLD,
                fine: 0.0,
            };
            assert!(crossings(&render(&rich, 48000)) > crossings(&render(&plain, 48000)) + 100);
        }
    }
}
