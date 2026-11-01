use std::f32::consts::TAU;

use crate::dsp::{self, Rng};
use crate::project::{Morph, Wave};

const WAVES: [Wave; 4] = [Wave::Sine, Wave::Triangle, Wave::Saw, Wave::Square];
const UNISON: f32 = 1e-3;

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    phase: [f32; 3],
    sub: f32,
}

impl State {
    pub fn start() -> Self {
        Self {
            phase: [0.0, 0.07, 0.13],
            sub: 0.0,
        }
    }
}

fn blend(position: f32) -> (usize, f32) {
    let p = position.clamp(0.0, 3.0);
    let i = (p.floor() as usize).min(2);
    (i, p - i as f32)
}

fn drive(x: f32, amount: f32) -> f32 {
    if amount <= 0.0 {
        return x;
    }
    let k = 1.0 + amount * 6.0;
    (x * k).tanh() / k.tanh()
}

pub fn next(m: &Morph, s: &mut State, hz: f32, rate: f32, rng: &mut Rng) -> f32 {
    let (i, t) = blend(m.position);
    let voices = if m.spread > UNISON { 3 } else { 1 };
    let detune = [0.0, m.spread, -m.spread];
    let mut sum = 0.0;
    for (phase, semis) in s.phase.iter_mut().zip(detune).take(voices) {
        let dt = (hz * dsp::semis(semis) / rate).min(0.5);
        let a = dsp::osc(WAVES[i], *phase, dt, rng);
        let b = dsp::osc(WAVES[i + 1], *phase, dt, rng);
        sum += a + (b - a) * t;
        *phase = (*phase + dt).fract();
    }
    let mut x = sum / (voices as f32).sqrt();
    if m.sub > 0.0 {
        x += (s.sub * TAU).sin() * m.sub;
        s.sub = (s.sub + 0.5 * hz / rate).fract();
    }
    drive(x, m.drive) * 0.7
}

pub fn shape(m: &Morph, t: f32) -> f32 {
    let (i, blend_t) = blend(m.position);
    let phase = t.fract();
    let a = dsp::shape(WAVES[i], phase);
    let b = dsp::shape(WAVES[i + 1], phase);
    let sub = (phase * 0.5 * TAU).sin() * m.sub;
    drive(a + (b - a) * blend_t + sub, m.drive)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod when_moving_the_position {
        use super::*;

        #[test]
        fn lands_on_each_wave() {
            assert_eq!(blend(0.0), (0, 0.0));
            assert_eq!(blend(2.0), (2, 0.0));
            assert_eq!(blend(3.0), (2, 1.0));
            assert_eq!(blend(9.0), (2, 1.0));
        }

        #[test]
        fn makes_sound_between_waves() {
            let m = Morph {
                position: 1.5,
                ..Morph::default()
            };
            let mut s = State::start();
            let mut rng = Rng::default();
            let peak = (0..4800)
                .map(|_| next(&m, &mut s, 220.0, 48000.0, &mut rng).abs())
                .fold(0.0, f32::max);
            assert!(peak > 0.3);
        }
    }
}
