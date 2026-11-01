use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::sample::Sample;

pub const GROUPS: usize = 4;
pub const PADS: usize = 16;
pub const STEPS: usize = 64;
pub const PATTERNS: usize = 16;
pub const LENGTHS: [usize; 4] = [16, 32, 48, 64];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Source {
    #[default]
    Sample,
    Synth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Mode {
    #[default]
    OneShot,
    Gate,
    Loop,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::OneShot, Mode::Gate, Mode::Loop];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Filter {
    #[default]
    Low,
    High,
    Band,
}

impl Filter {
    pub const ALL: [Filter; 3] = [Filter::Low, Filter::High, Filter::Band];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Wave {
    Sine,
    Triangle,
    #[default]
    Saw,
    Square,
    Noise,
}

impl Wave {
    pub const ALL: [Wave; 5] = [
        Wave::Sine,
        Wave::Triangle,
        Wave::Saw,
        Wave::Square,
        Wave::Noise,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Synth {
    pub wave_a: Wave,
    pub wave_b: Wave,
    pub detune: f32,
    pub mix: f32,
    pub sweep: f32,
}

impl Default for Synth {
    fn default() -> Self {
        Self {
            wave_a: Wave::Saw,
            wave_b: Wave::Square,
            detune: 0.1,
            mix: 0.3,
            sweep: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Pad {
    #[serde(skip)]
    pub sample: Option<Arc<Sample>>,
    pub sample_id: Option<String>,
    pub name: String,
    pub source: Source,
    pub start: f32,
    pub end: f32,
    pub pitch: f32,
    pub gain: f32,
    pub pan: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub filter: Filter,
    pub cutoff: f32,
    pub resonance: f32,
    pub env_amount: f32,
    pub mode: Mode,
    pub choke: u8,
    pub reverse: bool,
    pub synth: Synth,
}

impl Default for Pad {
    fn default() -> Self {
        Self {
            sample: None,
            sample_id: None,
            name: String::new(),
            source: Source::Sample,
            start: 0.0,
            end: 1.0,
            pitch: 0.0,
            gain: 0.8,
            pan: 0.0,
            attack: 0.001,
            decay: 0.4,
            sustain: 1.0,
            release: 0.05,
            filter: Filter::Low,
            cutoff: 1.0,
            resonance: 0.0,
            env_amount: 0.0,
            mode: Mode::OneShot,
            choke: 0,
            reverse: false,
            synth: Synth::default(),
        }
    }
}

impl Pad {
    pub fn silent(&self) -> bool {
        self.source == Source::Sample && self.sample.is_none()
    }

    pub fn label(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        match (self.source, &self.sample) {
            (Source::Synth, _) => "synth".into(),
            (Source::Sample, Some(s)) => s.name.clone(),
            (Source::Sample, None) => String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Step {
    pub on: bool,
    pub velocity: f32,
    pub pitch: i8,
    pub chance: f32,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            on: false,
            velocity: 0.8,
            pitch: 0,
            chance: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub length: usize,
    pub steps: Vec<Step>,
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            length: 16,
            steps: vec![Step::default(); PADS * STEPS],
        }
    }
}

impl Pattern {
    pub fn step(&self, pad: usize, step: usize) -> &Step {
        &self.steps[pad * STEPS + step]
    }

    pub fn step_mut(&mut self, pad: usize, step: usize) -> &mut Step {
        &mut self.steps[pad * STEPS + step]
    }

    pub fn clear_pad(&mut self, pad: usize) {
        for s in &mut self.steps[pad * STEPS..(pad + 1) * STEPS] {
            *s = Step::default();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.steps.iter().all(|s| !s.on)
    }
}

#[derive(Serialize, Deserialize)]
struct PatternFile {
    length: usize,
    notes: Vec<(usize, usize, Step)>,
}

impl Serialize for Pattern {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let notes = (0..PADS)
            .flat_map(|p| (0..STEPS).map(move |s| (p, s)))
            .filter(|&(p, s)| self.step(p, s).on)
            .map(|(p, s)| (p, s, *self.step(p, s)))
            .collect();
        PatternFile {
            length: self.length,
            notes,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let file = PatternFile::deserialize(deserializer)?;
        let mut pattern = Pattern {
            length: file.length.clamp(1, STEPS),
            ..Default::default()
        };
        for (p, s, step) in file.notes {
            if p < PADS && s < STEPS {
                *pattern.step_mut(p, s) = step;
            }
        }
        Ok(pattern)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Group {
    pub pads: Vec<Pad>,
    pub patterns: Vec<Pattern>,
    pub pattern: usize,
    pub volume: f32,
    pub mute: bool,
}

impl Default for Group {
    fn default() -> Self {
        Self {
            pads: vec![Pad::default(); PADS],
            patterns: vec![Pattern::default(); PATTERNS],
            pattern: 0,
            volume: 0.8,
            mute: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub tempo: f32,
    pub swing: f32,
    pub metronome: bool,
    pub volume: f32,
    pub groups: Vec<Group>,
    pub song: Vec<[usize; GROUPS]>,
    pub song_mode: bool,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            tempo: 96.0,
            swing: 0.0,
            metronome: false,
            volume: 0.8,
            groups: vec![Group::default(); GROUPS],
            song: Vec::new(),
            song_mode: false,
        }
    }
}

impl Project {
    pub fn starter() -> Self {
        let mut project = Self::default();
        project.groups[0].pads = crate::kit::drums();
        project.groups[1].pads = crate::kit::keys();
        let beat = &mut project.groups[0].patterns[0];
        for s in [0, 6, 10] {
            beat.step_mut(0, s).on = true;
        }
        for s in [4, 12] {
            beat.step_mut(4, s).on = true;
        }
        for s in (0..16).step_by(2) {
            let step = beat.step_mut(8, s);
            step.on = true;
            step.velocity = if s % 4 == 0 { 0.8 } else { 0.5 };
        }
        project
    }

    pub fn fix(&mut self) {
        self.groups.resize_with(GROUPS, Group::default);
        for g in &mut self.groups {
            g.pads.resize_with(PADS, Pad::default);
            g.patterns.resize_with(PATTERNS, Pattern::default);
            g.pattern = g.pattern.min(PATTERNS - 1);
        }
        for entry in &mut self.song {
            for p in entry.iter_mut() {
                *p = (*p).min(PATTERNS - 1);
            }
        }
        self.tempo = self.tempo.clamp(40.0, 240.0);
    }

    pub fn pattern(&self, group: usize) -> &Pattern {
        let g = &self.groups[group];
        &g.patterns[g.pattern]
    }

    pub fn samples(&self) -> Vec<Arc<Sample>> {
        let mut out: Vec<Arc<Sample>> = Vec::new();
        for pad in self.groups.iter().flat_map(|g| &g.pads) {
            if let Some(s) = &pad.sample
                && !out.iter().any(|o| o.id == s.id)
            {
                out.push(s.clone());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod when_saving_a_pattern {
        use super::*;

        #[test]
        fn keeps_only_the_steps_that_play() {
            let mut pattern = Pattern {
                length: 32,
                ..Pattern::default()
            };
            pattern.step_mut(3, 20).on = true;
            pattern.step_mut(3, 20).pitch = 5;
            let json = serde_json::to_string(&pattern).unwrap();
            assert!(json.len() < 200, "{json}");
            let back: Pattern = serde_json::from_str(&json).unwrap();
            assert_eq!(back, pattern);
        }
    }

    mod when_loading_an_old_project {
        use super::*;

        #[test]
        fn fills_in_missing_groups_and_pads() {
            let mut project: Project = serde_json::from_str(r#"{"groups":[{"pads":[]}]}"#).unwrap();
            project.fix();
            assert_eq!(project.groups.len(), GROUPS);
            assert!(project.groups.iter().all(|g| g.pads.len() == PADS));
            assert!(project.groups.iter().all(|g| g.patterns.len() == PATTERNS));
        }
    }
}
