use crate::project::{Filter, Mode, PADS, Pad, Source, Synth, Wave};

struct Voice {
    a: Wave,
    b: Wave,
    mix: f32,
    pitch: f32,
    sweep: f32,
    decay: f32,
    filter: Filter,
    cutoff: f32,
}

fn hit(v: Voice) -> Pad {
    Pad {
        source: Source::Synth,
        pitch: v.pitch,
        attack: 0.001,
        decay: v.decay,
        sustain: 0.0,
        release: v.decay,
        filter: v.filter,
        cutoff: v.cutoff,
        synth: Synth {
            wave_a: v.a,
            wave_b: v.b,
            detune: 0.0,
            mix: v.mix,
            sweep: v.sweep,
        },
        ..Pad::default()
    }
}

pub fn drums() -> Vec<Pad> {
    use Filter::*;
    use Wave::*;
    let v = |a, b, mix, pitch, sweep, decay, filter, cutoff| {
        hit(Voice {
            a,
            b,
            mix,
            pitch,
            sweep,
            decay,
            filter,
            cutoff,
        })
    };
    let mut pads = vec![
        v(Sine, Sine, 0.0, -26.0, 30.0, 0.35, Low, 1.0),
        v(Sine, Triangle, 0.2, -29.0, 24.0, 0.7, Low, 0.6),
        v(Sine, Sine, 0.0, -14.0, 12.0, 0.3, Low, 1.0),
        v(Sine, Sine, 0.0, -9.0, 12.0, 0.25, Low, 1.0),
        v(Triangle, Noise, 0.65, -5.0, 8.0, 0.18, Band, 0.8),
        v(Noise, Noise, 1.0, 0.0, 0.0, 0.14, Band, 0.68),
        v(Square, Square, 0.0, 14.0, 4.0, 0.03, High, 0.6),
        v(Square, Square, 0.5, 8.0, 0.0, 0.2, Band, 0.72),
        v(Noise, Noise, 1.0, 0.0, 0.0, 0.05, High, 0.86),
        v(Noise, Noise, 1.0, 0.0, 0.0, 0.35, High, 0.84),
        v(Noise, Noise, 1.0, 0.0, 0.0, 0.08, High, 0.92),
        v(Noise, Square, 0.8, 20.0, 0.0, 1.1, High, 0.8),
        v(Saw, Saw, 0.0, 0.0, 36.0, 0.12, Low, 0.9),
        v(Sine, Sine, 0.0, 24.0, 0.0, 0.06, Low, 1.0),
        v(Saw, Square, 0.3, -24.0, 0.0, 0.3, Low, 0.35),
        v(Saw, Saw, 0.5, 0.0, 0.0, 0.4, Low, 0.55),
    ];
    let names = [
        "kick", "boom", "tom lo", "tom hi", "snare", "clap", "rim", "cowbell", "hat", "open hat",
        "shaker", "cymbal", "zap", "blip", "bass hit", "stab",
    ];
    for (pad, name) in pads.iter_mut().zip(names) {
        pad.name = name.into();
    }
    pads[7].synth.detune = 7.0;
    pads[8].choke = 1;
    pads[9].choke = 1;
    pads[10].gain = 0.5;
    pads[14].env_amount = 0.4;
    pads[15].synth.detune = 7.0;
    pads[15].env_amount = 0.3;
    pads
}

pub fn keys() -> Vec<Pad> {
    let notes = [0.0, 3.0, 7.0, 10.0];
    let note_names = ["c", "d#", "g", "a#"];
    let row_names = ["bass", "pluck", "lead", "pad"];
    let rows = [
        (Wave::Saw, Wave::Square, -24.0, 0.002, 0.25, 0.3, 0.35, 0.4),
        (Wave::Triangle, Wave::Saw, -12.0, 0.001, 0.3, 0.0, 0.6, 0.3),
        (Wave::Square, Wave::Square, 0.0, 0.01, 0.2, 0.6, 0.65, 0.2),
        (Wave::Saw, Wave::Saw, 0.0, 0.4, 0.5, 0.8, 0.45, 0.1),
    ];
    (0..PADS)
        .map(|i| {
            let (a, b, base, attack, decay, sustain, cutoff, env) = rows[i / 4];
            Pad {
                name: format!("{} {}", row_names[i / 4], note_names[i % 4]),
                source: Source::Synth,
                pitch: base + notes[i % 4],
                attack,
                decay,
                sustain,
                release: 0.2 + attack,
                cutoff,
                env_amount: env,
                mode: Mode::Gate,
                gain: 0.6,
                synth: Synth {
                    wave_a: a,
                    wave_b: b,
                    detune: if i / 4 == 3 { 0.12 } else { 0.05 },
                    mix: 0.4,
                    sweep: 0.0,
                },
                ..Pad::default()
            }
        })
        .collect()
}
