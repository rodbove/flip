pub mod fm;
pub mod morph;
pub mod string;

use crate::dsp::{self, Rng};
use crate::project::{Pad, Source};

pub fn preview(pad: &Pad, n: usize) -> Vec<f32> {
    let n = n.max(2);
    match pad.source {
        Source::Fm => fm::preview(&pad.fm, n),
        Source::Morph => (0..n)
            .map(|i| morph::shape(&pad.morph, i as f32 / n as f32 * 2.0))
            .collect(),
        Source::String => string::preview(&pad.pluck, n),
        Source::Synth | Source::Sample => {
            let synth = &pad.synth;
            let ratio = dsp::semis(synth.detune);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32 * 2.0;
                    let a = dsp::shape(synth.wave_a, t.fract());
                    let b = dsp::shape(synth.wave_b, (t * ratio).fract());
                    a * (1.0 - synth.mix) + b * synth.mix
                })
                .collect()
        }
    }
}

pub(crate) fn peak_normalize(values: &mut [f32]) {
    let peak = values.iter().fold(0.0f32, |a, v| a.max(v.abs()));
    if peak > 1e-6 {
        for v in values {
            *v /= peak;
        }
    }
}

pub(crate) fn noise_rng() -> Rng {
    Rng::new(0x5eed)
}
