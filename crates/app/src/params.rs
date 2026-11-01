use flip_engine::project::{Filter as FilterKind, LENGTHS, Mode as PlayMode, Wave};
use flip_engine::{Pad, Step};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Param {
    Start,
    End,
    Pitch,
    Gain,
    Attack,
    Decay,
    Sustain,
    Release,
    Cutoff,
    Resonance,
    EnvAmount,
    Filter,
    WaveA,
    WaveB,
    Detune,
    Mix,
    Mode,
    Choke,
    Pan,
    Sweep,
    Velocity,
    StepPitch,
    Chance,
    Length,
    Tempo,
    Swing,
    GroupVolume,
    Master,
    Threshold,
    Capture,
    Reverse,
    Source,
}

pub enum Target {
    Pad,
    Step,
    Global,
}

pub struct Spec {
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub steps: Option<usize>,
}

const fn spec(name: &'static str, min: f32, max: f32, default: f32) -> Spec {
    Spec {
        name,
        min,
        max,
        default,
        steps: None,
    }
}

const fn choice(name: &'static str, count: usize, default: f32) -> Spec {
    Spec {
        name,
        min: 0.0,
        max: (count - 1) as f32,
        default,
        steps: Some(count),
    }
}

pub const CAPTURES: [&str; 6] = ["INPUT", "MASTER", "GRP A", "GRP B", "GRP C", "GRP D"];

pub const PAGES: [(&str, [Param; 4]); 8] = [
    (
        "TRIM",
        [Param::Start, Param::End, Param::Pitch, Param::Gain],
    ),
    (
        "ENV",
        [Param::Attack, Param::Decay, Param::Sustain, Param::Release],
    ),
    (
        "FILTER",
        [
            Param::Cutoff,
            Param::Resonance,
            Param::EnvAmount,
            Param::Filter,
        ],
    ),
    (
        "SYNTH",
        [Param::WaveA, Param::WaveB, Param::Detune, Param::Mix],
    ),
    (
        "PLAY",
        [Param::Mode, Param::Choke, Param::Pan, Param::Sweep],
    ),
    (
        "STEP",
        [
            Param::Velocity,
            Param::StepPitch,
            Param::Chance,
            Param::Length,
        ],
    ),
    (
        "MIX",
        [
            Param::Tempo,
            Param::Swing,
            Param::GroupVolume,
            Param::Master,
        ],
    ),
    (
        "REC",
        [
            Param::Threshold,
            Param::Capture,
            Param::Reverse,
            Param::Source,
        ],
    ),
];

impl Param {
    pub fn target(self) -> Target {
        use Param::*;
        match self {
            Velocity | StepPitch | Chance => Target::Step,
            Length | Tempo | Swing | GroupVolume | Master | Threshold | Capture => Target::Global,
            _ => Target::Pad,
        }
    }

    pub fn spec(self) -> Spec {
        use Param::*;
        match self {
            Start => spec("START", 0.0, 1.0, 0.0),
            End => spec("END", 0.0, 1.0, 1.0),
            Pitch => spec("PITCH", -24.0, 24.0, 0.0),
            Gain => spec("GAIN", 0.0, 1.5, 0.8),
            Attack => spec("ATTACK", 0.001, 2.0, 0.001),
            Decay => spec("DECAY", 0.01, 4.0, 0.4),
            Sustain => spec("SUSTAIN", 0.0, 1.0, 1.0),
            Release => spec("RELEASE", 0.005, 4.0, 0.05),
            Cutoff => spec("CUTOFF", 0.0, 1.0, 1.0),
            Resonance => spec("RES", 0.0, 1.0, 0.0),
            EnvAmount => spec("ENV AMT", -1.0, 1.0, 0.0),
            Filter => choice("TYPE", 3, 0.0),
            WaveA => choice("OSC A", 5, 2.0),
            WaveB => choice("OSC B", 5, 3.0),
            Detune => spec("DETUNE", -12.0, 12.0, 0.1),
            Mix => spec("MIX", 0.0, 1.0, 0.3),
            Mode => choice("MODE", 3, 0.0),
            Choke => choice("CHOKE", 5, 0.0),
            Pan => spec("PAN", -1.0, 1.0, 0.0),
            Sweep => spec("SWEEP", -48.0, 48.0, 0.0),
            Velocity => spec("VELOCITY", 0.0, 1.0, 0.8),
            StepPitch => spec("NOTE", -24.0, 24.0, 0.0),
            Chance => spec("CHANCE", 0.0, 1.0, 1.0),
            Length => choice("LENGTH", 4, 0.0),
            Tempo => spec("TEMPO", 40.0, 240.0, 96.0),
            Swing => spec("SWING", 0.0, 1.0, 0.0),
            GroupVolume => spec("GROUP", 0.0, 1.0, 0.8),
            Master => spec("MASTER", 0.0, 1.0, 0.8),
            Threshold => spec("THRESH", 0.0, 0.5, 0.05),
            Capture => choice("SOURCE", CAPTURES.len(), 0.0),
            Reverse => choice("REVERSE", 2, 0.0),
            Source => choice("ENGINE", 2, 0.0),
        }
    }

    pub fn curve(self) -> f32 {
        use Param::*;
        match self {
            Attack | Decay | Release => 3.0,
            _ => 1.0,
        }
    }

    pub fn unit_of(self, v: f32) -> f32 {
        let s = self.spec();
        let t = ((v - s.min) / (s.max - s.min)).clamp(0.0, 1.0);
        t.powf(1.0 / self.curve())
    }

    pub fn value_at(self, t: f32) -> f32 {
        let s = self.spec();
        let v = s.min + t.clamp(0.0, 1.0).powf(self.curve()) * (s.max - s.min);
        if s.steps.is_some() { v.round() } else { v }
    }

    pub fn pad_get(self, p: &Pad) -> f32 {
        use Param::*;
        match self {
            Start => p.start,
            End => p.end,
            Pitch => p.pitch,
            Gain => p.gain,
            Attack => p.attack,
            Decay => p.decay,
            Sustain => p.sustain,
            Release => p.release,
            Cutoff => p.cutoff,
            Resonance => p.resonance,
            EnvAmount => p.env_amount,
            Filter => index(&FilterKind::ALL, p.filter),
            WaveA => index(&Wave::ALL, p.synth.wave_a),
            WaveB => index(&Wave::ALL, p.synth.wave_b),
            Detune => p.synth.detune,
            Mix => p.synth.mix,
            Mode => index(&PlayMode::ALL, p.mode),
            Choke => p.choke as f32,
            Pan => p.pan,
            Sweep => p.synth.sweep,
            Reverse => p.reverse as u8 as f32,
            Source => (p.source == flip_engine::Source::Synth) as u8 as f32,
            _ => 0.0,
        }
    }

    pub fn pad_set(self, p: &mut Pad, v: f32) {
        use Param::*;
        let i = v.round().max(0.0) as usize;
        match self {
            Start => p.start = v,
            End => p.end = v,
            Pitch => p.pitch = v,
            Gain => p.gain = v,
            Attack => p.attack = v,
            Decay => p.decay = v,
            Sustain => p.sustain = v,
            Release => p.release = v,
            Cutoff => p.cutoff = v,
            Resonance => p.resonance = v,
            EnvAmount => p.env_amount = v,
            Filter => p.filter = pick(&FilterKind::ALL, i),
            WaveA => p.synth.wave_a = pick(&Wave::ALL, i),
            WaveB => p.synth.wave_b = pick(&Wave::ALL, i),
            Detune => p.synth.detune = v,
            Mix => p.synth.mix = v,
            Mode => p.mode = pick(&PlayMode::ALL, i),
            Choke => p.choke = i as u8,
            Pan => p.pan = v,
            Sweep => p.synth.sweep = v,
            Reverse => p.reverse = i > 0,
            Source => {
                p.source = if i > 0 {
                    flip_engine::Source::Synth
                } else {
                    flip_engine::Source::Sample
                }
            }
            _ => {}
        }
    }

    pub fn step_get(self, s: &Step) -> f32 {
        match self {
            Param::Velocity => s.velocity,
            Param::StepPitch => s.pitch as f32,
            Param::Chance => s.chance,
            _ => 0.0,
        }
    }

    pub fn step_set(self, s: &mut Step, v: f32) {
        match self {
            Param::Velocity => s.velocity = v,
            Param::StepPitch => s.pitch = v.round() as i8,
            Param::Chance => s.chance = v,
            _ => {}
        }
    }

    pub fn text(self, v: f32) -> String {
        use Param::*;
        let i = v.round().max(0.0) as usize;
        match self {
            Start | End | Mix | Sustain | Chance | Velocity | Swing | GroupVolume | Master
            | Cutoff | Resonance => format!("{:.0}%", v * 100.0),
            Gain => format!("{:.0}%", v * 100.0),
            Pitch | StepPitch | Sweep => format!("{v:+.0} st"),
            Detune => format!("{v:+.2}"),
            Attack | Decay | Release => {
                if v < 1.0 {
                    format!("{:.0} ms", v * 1000.0)
                } else {
                    format!("{v:.2} s")
                }
            }
            EnvAmount | Pan => format!("{:+.0}", v * 100.0),
            Filter => ["LOW", "HIGH", "BAND"][i.min(2)].into(),
            WaveA | WaveB => ["SINE", "TRI", "SAW", "SQR", "NOISE"][i.min(4)].into(),
            Mode => ["1-SHOT", "GATE", "LOOP"][i.min(2)].into(),
            Choke => {
                if i == 0 {
                    "OFF".into()
                } else {
                    format!("{i}")
                }
            }
            Length => format!("{}", LENGTHS[i.min(3)]),
            Tempo => format!("{v:.1}"),
            Threshold => format!("{:.0}%", v * 200.0),
            Capture => CAPTURES[i.min(CAPTURES.len() - 1)].into(),
            Reverse => if i > 0 { "ON" } else { "OFF" }.into(),
            Source => if i > 0 { "SYNTH" } else { "SAMPLE" }.into(),
        }
    }
}

fn index<T: PartialEq + Copy>(all: &[T], v: T) -> f32 {
    all.iter().position(|x| *x == v).unwrap_or(0) as f32
}

fn pick<T: Copy>(all: &[T], i: usize) -> T {
    all[i.min(all.len() - 1)]
}

pub fn length_index(length: usize) -> f32 {
    LENGTHS.iter().position(|l| *l >= length).unwrap_or(0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use flip_engine::Source;

    mod when_turning_a_knob {
        use super::*;

        #[test]
        fn round_trips_through_the_unit_range() {
            for p in [Param::Pitch, Param::Attack, Param::Cutoff, Param::Tempo] {
                let s = p.spec();
                let back = p.value_at(p.unit_of(s.default));
                assert!((back - s.default).abs() < 1e-3, "{p:?}");
            }
        }

        #[test]
        fn snaps_choices_to_whole_values() {
            assert_eq!(Param::WaveA.value_at(0.6), 2.0);
            assert_eq!(Param::Mode.value_at(1.0), 2.0);
        }

        #[test]
        fn writes_back_to_the_pad() {
            let mut pad = Pad::default();
            Param::WaveB.pad_set(&mut pad, 4.0);
            assert_eq!(pad.synth.wave_b, Wave::Noise);
            assert_eq!(Param::WaveB.pad_get(&pad), 4.0);
            Param::Source.pad_set(&mut pad, 1.0);
            assert_eq!(pad.source, Source::Synth);
        }
    }
}
