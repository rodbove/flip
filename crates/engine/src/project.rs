use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::sample::Sample;

pub const GROUPS: usize = 4;
pub const PADS: usize = 16;
pub const STEPS: usize = 256;
pub const STRIDE: usize = 64;
pub const PATTERNS: usize = 16;
pub const LENGTHS: [usize; 11] = [8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256];
pub const TICKS: u32 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Source {
    #[default]
    Sample,
    Synth,
    Fm,
    String,
    Morph,
}

impl Source {
    pub const ALL: [Source; 5] = [
        Source::Sample,
        Source::Synth,
        Source::Fm,
        Source::String,
        Source::Morph,
    ];

    pub fn is_synth(self) -> bool {
        self != Source::Sample
    }
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

pub const OPERATORS: usize = 4;
pub const ALGORITHMS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Op {
    pub ratio: f32,
    pub level: f32,
    pub decay: f32,
    pub fine: f32,
}

impl Default for Op {
    fn default() -> Self {
        Self {
            ratio: 1.0,
            level: 0.0,
            decay: 1.0,
            fine: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fm {
    pub ops: [Op; OPERATORS],
    pub algorithm: u8,
    pub feedback: f32,
    pub depth: f32,
    pub velocity: f32,
}

impl Default for Fm {
    fn default() -> Self {
        let op = |ratio, level, decay| Op {
            ratio,
            level,
            decay,
            fine: 0.0,
        };
        Self {
            ops: [
                op(1.0, 1.0, 10.0),
                op(1.0, 0.6, 0.8),
                Op::default(),
                Op::default(),
            ],
            algorithm: 0,
            feedback: 0.0,
            depth: 1.0,
            velocity: 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pluck {
    pub damping: f32,
    pub brightness: f32,
    pub body: f32,
    pub bow: f32,
}

impl Default for Pluck {
    fn default() -> Self {
        Self {
            damping: 0.5,
            brightness: 0.6,
            body: 0.3,
            bow: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Morph {
    pub position: f32,
    pub spread: f32,
    pub sub: f32,
    pub drive: f32,
}

impl Default for Morph {
    fn default() -> Self {
        Self {
            position: 1.0,
            spread: 0.1,
            sub: 0.0,
            drive: 0.0,
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
    pub fm: Fm,
    pub pluck: Pluck,
    pub morph: Morph,
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
            fm: Fm::default(),
            pluck: Pluck::default(),
            morph: Morph::default(),
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
            (Source::Fm, _) => "fm".into(),
            (Source::String, _) => "string".into(),
            (Source::Morph, _) => "morph".into(),
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub pad: u8,
    pub start: u32,
    pub length: u32,
    pub pitch: i8,
    pub velocity: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub length: usize,
    pub steps: Vec<Step>,
    pub notes: Vec<Note>,
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            length: 16,
            steps: vec![Step::default(); PADS * STRIDE],
            notes: Vec::new(),
        }
    }
}

impl Pattern {
    pub fn stride(&self) -> usize {
        self.steps.len() / PADS
    }

    pub fn step(&self, pad: usize, step: usize) -> &Step {
        static OFF: Step = Step {
            on: false,
            velocity: 0.8,
            pitch: 0,
            chance: 1.0,
        };
        let stride = self.stride();
        if step < stride {
            &self.steps[pad * stride + step]
        } else {
            &OFF
        }
    }

    pub fn step_mut(&mut self, pad: usize, step: usize) -> &mut Step {
        self.reserve(step + 1);
        let stride = self.stride();
        &mut self.steps[pad * stride + step]
    }

    fn reserve(&mut self, steps: usize) {
        let old = self.stride();
        if steps <= old {
            return;
        }
        let new = steps.next_multiple_of(STRIDE).min(STEPS);
        let mut grown = vec![Step::default(); PADS * new];
        for pad in 0..PADS {
            grown[pad * new..pad * new + old]
                .copy_from_slice(&self.steps[pad * old..(pad + 1) * old]);
        }
        self.steps = grown;
    }

    pub fn resize(&mut self, length: usize) {
        let length = length.clamp(1, STEPS);
        let old = self.length;
        self.reserve(length);
        if length > old && self.region_empty(old, length) {
            for pad in 0..PADS {
                for s in old..length {
                    *self.step_mut(pad, s) = *self.step(pad, s % old);
                }
            }
            let span = old as u32 * TICKS;
            let end = length as u32 * TICKS;
            let source: Vec<Note> = self
                .notes
                .iter()
                .filter(|n| n.start < span)
                .copied()
                .collect();
            let mut shift = span;
            while shift < end {
                for n in &source {
                    if n.start + shift < end {
                        self.notes.push(Note {
                            start: n.start + shift,
                            ..*n
                        });
                    }
                }
                shift += span;
            }
            self.notes.sort_by_key(|n| n.start);
        }
        self.length = length;
    }

    fn region_empty(&self, from: usize, to: usize) -> bool {
        let steps = (0..PADS).all(|p| (from..to).all(|s| !self.step(p, s).on));
        let (a, b) = (from as u32 * TICKS, to as u32 * TICKS);
        steps && !self.notes.iter().any(|n| n.start >= a && n.start < b)
    }

    pub fn clear_pad(&mut self, pad: usize) {
        let stride = self.stride();
        for s in &mut self.steps[pad * stride..(pad + 1) * stride] {
            *s = Step::default();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.steps.iter().all(|s| !s.on) && self.notes.is_empty()
    }

    pub fn ticks(&self) -> u32 {
        self.length as u32 * TICKS
    }

    pub fn add_note(&mut self, note: Note) {
        let at = self.notes.partition_point(|n| n.start <= note.start);
        self.notes.insert(at, note);
    }

    pub fn sort_notes(&mut self) {
        self.notes.sort_by_key(|n| n.start);
    }

    pub fn clear_notes(&mut self, pad: usize) {
        self.notes.retain(|n| n.pad as usize != pad);
    }

    pub fn notes_in(&self, from: u32, to: u32) -> &[Note] {
        let a = self.notes.partition_point(|n| n.start < from);
        let b = self.notes.partition_point(|n| n.start < to);
        &self.notes[a..b.max(a)]
    }
}

#[derive(Serialize, Deserialize)]
struct PatternFile {
    length: usize,
    notes: Vec<(usize, usize, Step)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    clip: Vec<Note>,
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
            clip: self.notes.clone(),
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
        let ticks = pattern.ticks();
        pattern.notes = file
            .clip
            .into_iter()
            .filter(|n| (n.pad as usize) < PADS && n.start < ticks)
            .map(|n| Note {
                length: n.length.max(1),
                ..n
            })
            .collect();
        pattern.sort_notes();
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
    pub mood: String,
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
            mood: String::new(),
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

        #[test]
        fn keeps_recorded_notes_in_order() {
            let mut pattern = Pattern::default();
            let note = |start, pitch| Note {
                pad: 2,
                start,
                length: 30,
                pitch,
                velocity: 0.7,
            };
            pattern.add_note(note(48, 4));
            pattern.add_note(note(0, 0));
            pattern.add_note(note(48, 7));
            let json = serde_json::to_string(&pattern).unwrap();
            let back: Pattern = serde_json::from_str(&json).unwrap();
            assert_eq!(back, pattern);
            let starts: Vec<u32> = back.notes.iter().map(|n| n.start).collect();
            assert_eq!(starts, vec![0, 48, 48]);
            assert_eq!(back.notes_in(24, 72).len(), 2);
        }

        #[test]
        fn drops_notes_past_the_end() {
            let json = r#"{"length":16,"notes":[],"clip":[{"pad":0,"start":9999,"length":5,"pitch":0,"velocity":1.0}]}"#;
            let back: Pattern = serde_json::from_str(json).unwrap();
            assert!(back.notes.is_empty());
        }
    }

    mod when_making_a_pattern_longer {
        use super::*;

        fn beat() -> Pattern {
            let mut p = Pattern::default();
            p.step_mut(0, 0).on = true;
            p.step_mut(2, 12).on = true;
            p.notes.push(Note {
                pad: 1,
                start: 4 * TICKS,
                length: TICKS,
                pitch: 3,
                velocity: 0.8,
            });
            p
        }

        #[test]
        fn repeats_the_loop_into_the_new_bars() {
            let mut p = beat();
            p.resize(64);
            assert_eq!(p.length, 64);
            assert!(p.step(0, 48).on);
            assert!(p.step(2, 60).on);
            assert_eq!(p.notes.len(), 4);
            assert_eq!(p.notes[3].start, (48 + 4) * TICKS);
        }

        #[test]
        fn grows_past_the_first_storage_block() {
            let mut p = beat();
            p.resize(256);
            assert!(p.stride() >= 256);
            assert!(p.step(0, 240).on);
            assert!(p.step(2, 252).on);
        }

        #[test]
        fn keeps_hidden_steps_when_shrunk_and_grown_again() {
            let mut p = beat();
            p.resize(32);
            p.step_mut(0, 16).on = false;
            p.step_mut(3, 20).on = true;
            p.resize(16);
            p.resize(32);
            assert!(p.step(3, 20).on);
            assert!(!p.step(0, 16).on);
        }

        #[test]
        fn saves_and_loads_long_patterns() {
            let mut p = beat();
            p.resize(200);
            let back: Pattern = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
            assert_eq!(back.length, 200);
            assert!(back.step(2, 188).on);
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

        #[test]
        fn gives_old_synth_pads_default_engine_settings() {
            let pad: Pad = serde_json::from_str(r#"{"source":"Synth","pitch":3.0}"#).unwrap();
            assert_eq!(pad.source, Source::Synth);
            assert_eq!(pad.pitch, 3.0);
            assert_eq!(pad.fm, Fm::default());
            assert_eq!(pad.pluck, Pluck::default());
        }
    }
}
