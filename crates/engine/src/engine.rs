use std::sync::Arc;

use crate::dsp::{self, Coefs, Env, Rng, Shape, Stage, Svf};
use crate::project::{Filter, GROUPS, Mode, PADS, Pad, Pattern, Project, Source, Step, TICKS};
use crate::sample::Sample;
use crate::synth::{fm, morph, string};

const VOICES: usize = 48;
const KILL: u32 = 96;
const TRASH: usize = 256;
const PENDING: usize = 256;
const COEF_EVERY: u32 = 16;
const MIDDLE_C: f32 = 261.626;
const GATE_SHARE: f64 = 0.9;
const SWING_SPAN: f64 = 0.5;
const CLICK_DECAY: f32 = 0.015;
const CLICK_GAIN: f32 = 0.25;
const SWEEP_SHARE: f32 = 0.25;
const SWEEP_MIN: f32 = 0.008;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tap {
    Master,
    Group(usize),
}

pub enum Command {
    Note {
        group: usize,
        pad: usize,
        note: i8,
        velocity: f32,
        on: bool,
    },
    Pad {
        group: usize,
        pad: usize,
        data: Box<Pad>,
    },
    Pattern {
        group: usize,
        index: usize,
        data: Box<Pattern>,
    },
    Write {
        group: usize,
        index: usize,
        pad: usize,
        step: usize,
        data: Step,
    },
    Select {
        group: usize,
        pattern: usize,
    },
    Song(Box<Vec<[usize; GROUPS]>>),
    SongMode(bool),
    Play(bool),
    Tempo(f32),
    Swing(f32),
    Metronome(bool),
    Mix {
        group: usize,
        volume: f32,
        mute: bool,
    },
    Volume(f32),
    Tap(Option<Tap>),
    Load(Box<Project>),
    Panic,
}

pub enum Trash {
    Pad(Box<Pad>),
    Pattern(Box<Pattern>),
    Song(Box<Vec<[usize; GROUPS]>>),
    Project(Box<Project>),
    Sample(Arc<Sample>),
}

#[derive(Default)]
struct Voice {
    active: bool,
    group: usize,
    pad: usize,
    note: i8,
    velocity: f32,
    env: Env,
    gate: bool,
    gate_left: Option<u64>,
    sample: Option<Arc<Sample>>,
    pos: f64,
    kill: u32,
    age: u64,
    phase_a: f32,
    phase_b: f32,
    sweep: f32,
    filters: [Svf; 2],
    coefs: Option<Coefs>,
    tick: u32,
    fm: fm::State,
    morph: morph::State,
    string: string::State,
}

#[derive(Clone, Copy)]
struct Pending {
    group: usize,
    pad: usize,
    note: i8,
    velocity: f32,
    at: f64,
    gate: u64,
}

#[derive(Default)]
struct Click {
    phase: f32,
    freq: f32,
    level: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Report {
    pub playing: bool,
    pub phase: f64,
    pub next_time: f64,
    pub pos: [usize; GROUPS],
    pub current: [usize; GROUPS],
    pub entry: usize,
    pub hits: [u32; GROUPS],
    pub peak: [f32; 2],
    pub voices: usize,
}

pub struct Engine {
    rate: f32,
    project: Box<Project>,
    voices: Vec<Voice>,
    strings: Vec<Box<[f32]>>,
    rng: Rng,
    playing: bool,
    phase: f64,
    next: u64,
    pos: [usize; GROUPS],
    current: [usize; GROUPS],
    queued: [Option<usize>; GROUPS],
    entry: usize,
    entry_pos: usize,
    suppress: [u32; GROUPS],
    pending: Vec<Pending>,
    click: Click,
    tap: Option<Tap>,
    age: u64,
    hits: [u32; GROUPS],
    peak: [f32; 2],
    trash: Vec<Trash>,
}

impl Engine {
    pub fn new(project: Project, rate: u32) -> Self {
        let current = std::array::from_fn(|g| project.groups[g].pattern);
        Self {
            rate: rate as f32,
            project: Box::new(project),
            voices: (0..VOICES).map(|_| Voice::default()).collect(),
            strings: (0..VOICES)
                .map(|_| vec![0.0; string::buffer_len(rate as f32)].into_boxed_slice())
                .collect(),
            rng: Rng::default(),
            playing: false,
            phase: 0.0,
            next: 0,
            pos: [0; GROUPS],
            current,
            queued: [None; GROUPS],
            entry: 0,
            entry_pos: 0,
            suppress: [0; GROUPS],
            pending: Vec::with_capacity(PENDING),
            click: Click::default(),
            tap: None,
            age: 0,
            hits: [0; GROUPS],
            peak: [0.0; 2],
            trash: Vec::with_capacity(TRASH),
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn take_trash(&mut self) -> impl Iterator<Item = Trash> + '_ {
        self.trash.drain(..)
    }

    fn discard(&mut self, t: Trash) {
        if self.trash.len() < self.trash.capacity() {
            self.trash.push(t);
        }
    }

    pub fn report(&mut self) -> Report {
        let report = Report {
            playing: self.playing,
            phase: self.phase,
            next_time: self.step_time(self.next),
            pos: self.pos,
            current: self.current,
            entry: self.entry,
            hits: self.hits,
            peak: self.peak,
            voices: self.voices.iter().filter(|v| v.active).count(),
        };
        self.hits = [0; GROUPS];
        self.peak = [0.0; 2];
        report
    }

    pub fn handle(&mut self, command: Command) {
        match command {
            Command::Note {
                group,
                pad,
                note,
                velocity,
                on,
            } => {
                if group < GROUPS && pad < PADS {
                    if on {
                        self.note_on(group, pad, note, velocity, None);
                    } else {
                        self.note_off(group, pad, note);
                    }
                }
            }
            Command::Pad {
                group,
                pad,
                mut data,
            } => {
                std::mem::swap(&mut self.project.groups[group].pads[pad], &mut data);
                self.discard(Trash::Pad(data));
            }
            Command::Pattern {
                group,
                index,
                mut data,
            } => {
                std::mem::swap(&mut self.project.groups[group].patterns[index], &mut data);
                self.discard(Trash::Pattern(data));
            }
            Command::Write {
                group,
                index,
                pad,
                step,
                data,
            } => {
                *self.project.groups[group].patterns[index].step_mut(pad, step) = data;
                if self.playing && self.current[group] == index && self.pos[group] == step {
                    self.suppress[group] |= 1 << pad;
                }
            }
            Command::Select { group, pattern } => {
                self.project.groups[group].pattern = pattern;
                if self.playing && !self.song_active() {
                    self.queued[group] = Some(pattern);
                } else if !self.playing {
                    self.current[group] = pattern;
                }
            }
            Command::Song(mut data) => {
                std::mem::swap(&mut self.project.song, &mut data);
                if self.entry >= self.project.song.len() {
                    self.entry = 0;
                    self.entry_pos = 0;
                }
                self.discard(Trash::Song(data));
            }
            Command::SongMode(on) => self.project.song_mode = on,
            Command::Play(on) => self.transport(on),
            Command::Tempo(t) => self.project.tempo = t.clamp(40.0, 240.0),
            Command::Swing(s) => self.project.swing = s.clamp(0.0, 1.0),
            Command::Metronome(on) => self.project.metronome = on,
            Command::Mix {
                group,
                volume,
                mute,
            } => {
                self.project.groups[group].volume = volume;
                self.project.groups[group].mute = mute;
            }
            Command::Volume(v) => self.project.volume = v,
            Command::Tap(t) => self.tap = t,
            Command::Load(mut data) => {
                std::mem::swap(&mut self.project, &mut data);
                self.discard(Trash::Project(data));
                for g in 0..GROUPS {
                    self.queued[g] = None;
                    if !self.playing {
                        self.current[g] = self.project.groups[g].pattern;
                    }
                }
                self.entry = self.entry.min(self.project.song.len().saturating_sub(1));
            }
            Command::Panic => {
                for i in 0..VOICES {
                    self.finish(i);
                }
            }
        }
    }

    fn song_active(&self) -> bool {
        self.project.song_mode && !self.project.song.is_empty()
    }

    fn transport(&mut self, on: bool) {
        self.playing = on;
        self.pending.clear();
        if on {
            self.phase = 0.0;
            self.next = 0;
            self.pos = [0; GROUPS];
            self.entry = 0;
            self.entry_pos = 0;
            self.queued = [None; GROUPS];
            self.suppress = [0; GROUPS];
            self.current = std::array::from_fn(|g| self.project.groups[g].pattern);
        }
    }

    fn step_time(&self, n: u64) -> f64 {
        let swing = if n % 2 == 1 {
            self.project.swing as f64 * SWING_SPAN
        } else {
            0.0
        };
        n as f64 + swing
    }

    fn step_samples(&self) -> f64 {
        self.rate as f64 * 60.0 / (self.project.tempo as f64 * 4.0)
    }

    fn entry_length(&self, entry: usize) -> usize {
        let e = self.project.song[entry];
        (0..GROUPS)
            .map(|g| self.project.groups[g].patterns[e[g]].length)
            .max()
            .unwrap_or(16)
    }

    pub fn loop_steps(&self) -> u64 {
        if self.song_active() {
            (0..self.project.song.len())
                .map(|e| self.entry_length(e) as u64)
                .sum()
        } else {
            (0..GROUPS)
                .map(|g| self.project.pattern(g).length as u64)
                .max()
                .unwrap_or(16)
        }
    }

    fn fire(&mut self) {
        let n = self.next;
        if self.project.metronome && n.is_multiple_of(4) {
            self.click = Click {
                phase: 0.0,
                freq: if n.is_multiple_of(16) { 1760.0 } else { 1320.0 },
                level: 1.0,
            };
        }
        let song = self.song_active();
        if song && self.entry_pos == 0 {
            let e = self.project.song[self.entry];
            self.current = e;
            self.pos = [0; GROUPS];
        }
        let gate = (self.step_samples() * GATE_SHARE) as u64;
        for g in 0..GROUPS {
            if !song
                && self.pos[g] == 0
                && let Some(q) = self.queued[g].take()
            {
                self.current[g] = q;
            }
            let length = self.project.groups[g].patterns[self.current[g]].length;
            if self.pos[g] >= length {
                self.pos[g] = 0;
            }
            let at = self.pos[g];
            let skip = std::mem::take(&mut self.suppress[g]);
            for pad in 0..PADS {
                let step = *self.project.groups[g].patterns[self.current[g]].step(pad, at);
                if !step.on || skip & (1 << pad) != 0 {
                    continue;
                }
                if step.chance < 1.0 && self.rng.unit() >= step.chance {
                    continue;
                }
                self.note_on(g, pad, step.pitch, step.velocity, Some(gate));
            }
            self.queue_notes(g, at);
            self.pos[g] = (at + 1) % length;
        }
        if song {
            self.entry_pos += 1;
            if self.entry_pos >= self.entry_length(self.entry) {
                self.entry_pos = 0;
                self.entry = (self.entry + 1) % self.project.song.len();
            }
        }
        self.next += 1;
    }

    fn queue_notes(&mut self, group: usize, at: usize) {
        let start = self.step_time(self.next);
        let per_tick = self.step_samples() / TICKS as f64;
        let pattern = &self.project.groups[group].patterns[self.current[group]];
        let from = at as u32 * TICKS;
        for n in pattern.notes_in(from, from + TICKS) {
            if self.pending.len() >= self.pending.capacity() {
                break;
            }
            self.pending.push(Pending {
                group,
                pad: n.pad as usize,
                note: n.pitch,
                velocity: n.velocity,
                at: start + (n.start - from) as f64 / TICKS as f64,
                gate: (n.length as f64 * per_tick) as u64,
            });
        }
    }

    fn play_pending(&mut self) {
        let mut i = 0;
        while i < self.pending.len() {
            if self.pending[i].at <= self.phase {
                let p = self.pending.swap_remove(i);
                if p.pad < PADS {
                    self.note_on(p.group, p.pad, p.note, p.velocity, Some(p.gate));
                }
            } else {
                i += 1;
            }
        }
    }

    fn note_on(&mut self, group: usize, pad: usize, note: i8, velocity: f32, gate: Option<u64>) {
        let p = &self.project.groups[group].pads[pad];
        if p.silent() {
            return;
        }
        let choke = p.choke;
        let source = p.source;
        let sample = match source {
            Source::Sample => p.sample.clone(),
            _ => None,
        };
        let pos = match &sample {
            Some(s) => {
                let (a, b) = bounds(p, s.len());
                if p.reverse { b - 1.0 } else { a }
            }
            None => 0.0,
        };
        for i in 0..VOICES {
            let v = &self.voices[i];
            if !v.active || v.kill > 0 || v.group != group {
                continue;
            }
            let same = v.pad == pad && v.note == note;
            let choked =
                choke > 0 && v.pad != pad && self.project.groups[group].pads[v.pad].choke == choke;
            if same || choked {
                self.voices[i].kill = KILL;
            }
        }
        let slot = self
            .voices
            .iter()
            .position(|v| !v.active)
            .unwrap_or_else(|| (0..VOICES).min_by_key(|&i| self.voices[i].age).unwrap_or(0));
        self.finish(slot);
        self.age += 1;
        self.hits[group] |= 1 << pad;
        let mut env = Env::default();
        env.start();
        self.voices[slot] = Voice {
            active: true,
            group,
            pad,
            note,
            velocity: velocity.clamp(0.0, 1.0),
            env,
            gate: true,
            gate_left: gate,
            sample,
            pos,
            kill: 0,
            age: self.age,
            phase_a: 0.0,
            phase_b: 0.0,
            sweep: 1.0,
            filters: [Svf::default(); 2],
            coefs: None,
            tick: 0,
            fm: fm::State::start(),
            morph: morph::State::start(),
            string: string::State::default(),
        };
        if source == Source::String {
            let pad = &self.project.groups[group].pads[pad];
            let hz = MIDDLE_C * dsp::semis(pad.pitch + note as f32);
            self.voices[slot].string = string::start(
                &pad.pluck,
                &mut self.strings[slot],
                hz,
                self.rate,
                &mut self.rng,
            );
        }
    }

    fn note_off(&mut self, group: usize, pad: usize, note: i8) {
        for v in &mut self.voices {
            if v.active && v.group == group && v.pad == pad && v.note == note && v.gate {
                v.gate = false;
                if self.project.groups[group].pads[pad].mode != Mode::OneShot {
                    v.env.release();
                }
            }
        }
    }

    fn finish(&mut self, i: usize) {
        let v = &mut self.voices[i];
        v.active = false;
        if let Some(s) = v.sample.take() {
            self.discard(Trash::Sample(s));
        }
    }

    pub fn render(&mut self, out: &mut [[f32; 2]], tap: &mut dyn FnMut([f32; 2])) {
        let dt = 1.0 / self.rate;
        for frame in out.iter_mut() {
            if self.playing {
                let inc = 1.0 / self.step_samples();
                while self.phase >= self.step_time(self.next) {
                    self.fire();
                }
                if !self.pending.is_empty() {
                    self.play_pending();
                }
                self.phase += inc;
            }
            let mut buses = [[0.0f32; 2]; GROUPS];
            for i in 0..VOICES {
                if !self.voices[i].active {
                    continue;
                }
                match self.voice(i, dt) {
                    Some(s) => {
                        let bus = &mut buses[self.voices[i].group];
                        bus[0] += s[0];
                        bus[1] += s[1];
                    }
                    None => self.finish(i),
                }
            }
            let mut master = [0.0f32; 2];
            for (g, bus) in buses.iter_mut().enumerate() {
                let group = &self.project.groups[g];
                let gain = if group.mute { 0.0 } else { group.volume };
                bus[0] *= gain;
                bus[1] *= gain;
                master[0] += bus[0];
                master[1] += bus[1];
            }
            let volume = self.project.volume;
            master = [soft(master[0] * volume), soft(master[1] * volume)];
            match self.tap {
                Some(Tap::Master) => tap(master),
                Some(Tap::Group(g)) if g < GROUPS => tap(buses[g]),
                _ => {}
            }
            let click = self.click_next(dt);
            self.peak[0] = self.peak[0].max(master[0].abs());
            self.peak[1] = self.peak[1].max(master[1].abs());
            *frame = [master[0] + click, master[1] + click];
        }
    }

    fn click_next(&mut self, dt: f32) -> f32 {
        let c = &mut self.click;
        if c.level < 1e-4 {
            return 0.0;
        }
        let s = (c.phase * std::f32::consts::TAU).sin() * c.level * CLICK_GAIN;
        c.phase = (c.phase + c.freq * dt).fract();
        c.level *= (-dt / CLICK_DECAY).exp();
        s
    }

    fn voice(&mut self, i: usize, dt: f32) -> Option<[f32; 2]> {
        let rate = self.rate;
        let v = &mut self.voices[i];
        let pad = &self.project.groups[v.group].pads[v.pad];
        if let Some(left) = v.gate_left.as_mut() {
            if *left == 0 {
                v.gate_left = None;
                v.gate = false;
                if pad.mode != Mode::OneShot {
                    v.env.release();
                }
            } else {
                *left -= 1;
            }
        }
        let shape = Shape {
            attack: pad.attack,
            decay: pad.decay,
            sustain: pad.sustain,
            release: pad.release,
        };
        if pad.source.is_synth() && pad.mode == Mode::OneShot && v.env.stage == Stage::Sustain {
            v.env.release();
        }
        let level = v.env.next(&shape, dt);
        if v.env.done() {
            return None;
        }
        let tune = pad.pitch + v.note as f32;
        let raw = match (&v.sample, pad.source) {
            (Some(s), Source::Sample) => {
                let (a, b) = bounds(pad, s.len());
                let value = s.read(v.pos);
                let step = s.rate as f64 / rate as f64 * dsp::semis(tune) as f64;
                let span = b - a;
                let left = if pad.reverse { v.pos - a } else { b - v.pos };
                if pad.mode == Mode::OneShot
                    && v.env.stage != Stage::Release
                    && left / step < (pad.release * rate) as f64
                {
                    v.env.release();
                }
                if pad.reverse {
                    v.pos -= step;
                    if v.pos < a {
                        if pad.mode == Mode::Loop && span > 1.0 {
                            v.pos += span;
                        } else {
                            return None;
                        }
                    }
                } else {
                    v.pos += step;
                    if v.pos >= b {
                        if pad.mode == Mode::Loop && span > 1.0 {
                            v.pos -= span;
                        } else {
                            return None;
                        }
                    }
                }
                value
            }
            (_, Source::Sample) => return None,
            (_, source) => {
                let synth = &pad.synth;
                let sweep_time = (pad.decay * SWEEP_SHARE).max(SWEEP_MIN);
                v.sweep *= (-dt / sweep_time).exp();
                let hz = MIDDLE_C * dsp::semis(tune + synth.sweep * v.sweep);
                let s = match source {
                    Source::Fm => fm::next(&pad.fm, &mut v.fm, hz, v.velocity, dt),
                    Source::Morph => morph::next(&pad.morph, &mut v.morph, hz, rate, &mut self.rng),
                    Source::String => string::next(
                        &pad.pluck,
                        &mut self.strings[i],
                        &mut v.string,
                        &mut self.rng,
                    ),
                    _ => analog(synth, v, hz, rate, &mut self.rng),
                };
                [s * 0.5, s * 0.5]
            }
        };

        let bypass = pad.filter == Filter::Low && pad.cutoff >= 1.0 && pad.env_amount <= 0.0;
        let mut s = raw;
        if !bypass {
            if v.tick == 0 || v.coefs.is_none() {
                let c = pad.cutoff + pad.env_amount * level;
                v.coefs = Some(Coefs::new(dsp::cutoff_hz(c), pad.resonance, rate));
            }
            v.tick = (v.tick + 1) % COEF_EVERY;
            if let Some(c) = &v.coefs {
                s = [
                    v.filters[0].run(s[0], c, pad.filter),
                    v.filters[1].run(s[1], c, pad.filter),
                ];
            }
        }
        let mut gain = level * v.velocity * pad.gain;
        if v.kill > 0 {
            v.kill -= 1;
            if v.kill == 0 {
                return None;
            }
            gain *= v.kill as f32 / KILL as f32;
        }
        let [l, r] = dsp::pan(pad.pan);
        Some([s[0] * gain * l, s[1] * gain * r])
    }

    pub fn playing(&self) -> bool {
        self.playing
    }
}

fn analog(synth: &crate::project::Synth, v: &mut Voice, hz: f32, rate: f32, rng: &mut Rng) -> f32 {
    let dt_a = (hz / rate).min(0.5);
    let dt_b = (hz * dsp::semis(synth.detune) / rate).min(0.5);
    let a = dsp::osc(synth.wave_a, v.phase_a, dt_a, rng);
    let b = dsp::osc(synth.wave_b, v.phase_b, dt_b, rng);
    v.phase_a = (v.phase_a + dt_a).fract();
    v.phase_b = (v.phase_b + dt_b).fract();
    a * (1.0 - synth.mix) + b * synth.mix
}

fn soft(x: f32) -> f32 {
    if x.abs() < 0.8 {
        x
    } else {
        x.signum() * (0.8 + 0.2 * ((x.abs() - 0.8) / 0.2).tanh())
    }
}

fn bounds(pad: &Pad, len: usize) -> (f64, f64) {
    let n = len as f64;
    let (a, b) = (pad.start.min(pad.end), pad.start.max(pad.end));
    let a = (a.clamp(0.0, 1.0) as f64 * n).floor();
    let b = (b.clamp(0.0, 1.0) as f64 * n).ceil().max(a + 1.0).min(n);
    (a, b)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bounce {
    Pattern,
    Song,
    Group(usize),
}

pub fn bounce(project: &Project, rate: u32, what: Bounce, tail: f32) -> Vec<[f32; 2]> {
    let mut project = project.clone();
    project.metronome = false;
    project.song_mode = what == Bounce::Song;
    if let Bounce::Group(solo) = what {
        for (g, group) in project.groups.iter_mut().enumerate() {
            group.mute = g != solo;
            if g == solo {
                group.volume = 1.0;
            }
        }
    }
    let mut engine = Engine::new(project, rate);
    engine.transport(true);
    let steps = engine.loop_steps();
    let length = (steps as f64 * engine.step_samples()).round() as usize;
    let mut out = vec![[0.0f32; 2]; length];
    let tap = matches!(what, Bounce::Group(_));
    let mut solo = Vec::with_capacity(if tap { length } else { 0 });
    if let Bounce::Group(g) = what {
        engine.tap = Some(Tap::Group(g));
    }
    engine.render(&mut out, &mut |f| solo.push(f));
    engine.trash.clear();
    engine.playing = false;
    let tail_frames = (tail.max(0.0) * rate as f32) as usize;
    if tail_frames > 0 {
        let mut rest = vec![[0.0f32; 2]; tail_frames];
        engine.render(&mut rest, &mut |f| solo.push(f));
        out.extend(rest);
    }
    if tap { solo } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48000;

    fn engine() -> Engine {
        Engine::new(Project::starter(), RATE)
    }

    fn run(e: &mut Engine, frames: usize) -> Vec<[f32; 2]> {
        let mut out = vec![[0.0; 2]; frames];
        e.render(&mut out, &mut |_| {});
        e.trash.clear();
        out
    }

    fn steps(e: &mut Engine, n: f64) {
        let frames = (e.step_samples() * n) as usize;
        run(e, frames);
    }

    fn loud(frames: &[[f32; 2]]) -> f32 {
        crate::sample::peak(frames)
    }

    mod when_a_pad_is_hit {
        use super::*;

        #[test]
        fn makes_sound() {
            let mut e = engine();
            e.handle(Command::Note {
                group: 0,
                pad: 0,
                note: 0,
                velocity: 1.0,
                on: true,
            });
            assert!(loud(&run(&mut e, 4800)) > 0.1);
        }

        #[test]
        fn falls_silent_after_the_decay() {
            let mut e = engine();
            e.handle(Command::Note {
                group: 0,
                pad: 8,
                note: 0,
                velocity: 1.0,
                on: true,
            });
            run(&mut e, RATE as usize);
            assert_eq!(e.report().voices, 0);
        }
    }

    mod when_a_synth_engine_plays {
        use super::*;

        fn with_engine(source: Source) -> Engine {
            let mut project = Project::default();
            let pad = &mut project.groups[0].pads[0];
            pad.source = source;
            pad.mode = Mode::Gate;
            pad.sustain = 1.0;
            pad.release = 0.05;
            Engine::new(project, RATE)
        }

        fn note(e: &mut Engine, on: bool) {
            e.handle(Command::Note {
                group: 0,
                pad: 0,
                note: 0,
                velocity: 1.0,
                on,
            });
        }

        #[test]
        fn each_engine_sounds_and_stops_after_release() {
            for source in [Source::Synth, Source::Fm, Source::String, Source::Morph] {
                let mut e = with_engine(source);
                note(&mut e, true);
                assert!(loud(&run(&mut e, 4800)) > 0.05, "{source:?}");
                note(&mut e, false);
                run(&mut e, RATE as usize);
                assert_eq!(e.report().voices, 0, "{source:?}");
            }
        }

        #[test]
        fn plays_chords_on_one_pad() {
            let mut e = with_engine(Source::Fm);
            for n in [0, 4, 7] {
                e.handle(Command::Note {
                    group: 0,
                    pad: 0,
                    note: n,
                    velocity: 1.0,
                    on: true,
                });
            }
            run(&mut e, 480);
            assert_eq!(e.report().voices, 3);
        }
    }

    mod when_a_pad_has_no_sound {
        use super::*;

        #[test]
        fn stays_silent() {
            let mut e = engine();
            e.handle(Command::Note {
                group: 2,
                pad: 0,
                note: 0,
                velocity: 1.0,
                on: true,
            });
            assert_eq!(loud(&run(&mut e, 4800)), 0.0);
            assert_eq!(e.report().voices, 0);
        }
    }

    mod when_a_sample_plays {
        use super::*;

        fn with_sample(mode: Mode) -> Engine {
            let mut project = Project::default();
            let pad = &mut project.groups[0].pads[0];
            pad.sample = Some(Arc::new(Sample::new("dc", RATE, vec![[0.5, 0.5]; 4800])));
            pad.mode = mode;
            pad.gain = 1.0;
            Engine::new(project, RATE)
        }

        fn hit(e: &mut Engine, on: bool) {
            e.handle(Command::Note {
                group: 0,
                pad: 0,
                note: 0,
                velocity: 1.0,
                on,
            });
        }

        #[test]
        fn stops_at_the_end_of_the_sample() {
            let mut e = with_sample(Mode::OneShot);
            hit(&mut e, true);
            let out = run(&mut e, 9600);
            assert!(out[2400][0] > 0.15);
            assert_eq!(out[6000][0], 0.0);
        }

        #[test]
        fn hands_the_sample_back_when_done() {
            let mut e = with_sample(Mode::OneShot);
            hit(&mut e, true);
            let mut out = vec![[0.0; 2]; 9600];
            e.render(&mut out, &mut |_| {});
            assert!(e.take_trash().any(|t| matches!(t, Trash::Sample(_))));
        }

        #[test]
        fn fades_out_over_the_release() {
            let mut e = with_sample(Mode::OneShot);
            e.project.groups[0].pads[0].release = 0.05;
            hit(&mut e, true);
            let out = run(&mut e, 4800);
            let early = out[4800 - 2400 - 1][0];
            let late = out[4800 - 300][0];
            assert!(late < early * 0.5, "{late} vs {early}");
        }

        #[test]
        fn loops_while_held() {
            let mut e = with_sample(Mode::Loop);
            hit(&mut e, true);
            let out = run(&mut e, 24000);
            assert!(out[20000][0] > 0.15);
            hit(&mut e, false);
            run(&mut e, 9600);
            assert_eq!(e.report().voices, 0);
        }
    }

    mod when_choking {
        use super::*;

        #[test]
        fn cuts_the_other_pad() {
            let mut e = engine();
            for pad in [9, 8] {
                e.handle(Command::Note {
                    group: 0,
                    pad,
                    note: 0,
                    velocity: 1.0,
                    on: true,
                });
                run(&mut e, 10);
            }
            run(&mut e, KILL as usize + 10);
            let active: Vec<usize> = e
                .voices
                .iter()
                .filter(|v| v.active)
                .map(|v| v.pad)
                .collect();
            assert_eq!(active, vec![8]);
        }
    }

    mod when_playing_the_sequence {
        use super::*;

        #[test]
        fn hits_the_programmed_steps() {
            let mut e = engine();
            e.handle(Command::Play(true));
            let steps = 16;
            let frames = (e.step_samples() * steps as f64) as usize + 10;
            let mut hits = 0;
            for _ in 0..frames / 64 {
                run(&mut e, 64);
                hits += e.report().hits[0].count_ones();
            }
            let expected: usize = (0..PADS)
                .map(|p| {
                    (0..16)
                        .filter(|&s| e.project.pattern(0).step(p, s).on)
                        .count()
                })
                .sum();
            assert!(hits as usize >= expected - 2, "{hits} of {expected}");
        }

        #[test]
        fn loops_the_pattern() {
            let mut e = engine();
            e.handle(Command::Play(true));
            steps(&mut e, 17.5);
            assert_eq!(e.next, 18);
            assert_eq!(e.pos[0], 2);
        }

        #[test]
        fn switches_pattern_at_the_end_of_the_loop() {
            let mut e = engine();
            e.handle(Command::Play(true));
            steps(&mut e, 4.5);
            e.handle(Command::Select {
                group: 0,
                pattern: 3,
            });
            assert_eq!(e.current[0], 0);
            steps(&mut e, 12.0);
            assert_eq!(e.current[0], 3);
        }

        #[test]
        fn delays_the_off_beats_with_swing() {
            let mut e = engine();
            e.handle(Command::Swing(1.0));
            assert_eq!(e.step_time(1), 1.5);
            assert_eq!(e.step_time(2), 2.0);
        }
    }

    mod when_recording_live {
        use super::*;

        #[test]
        fn does_not_double_the_coming_step() {
            let mut e = engine();
            e.handle(Command::Play(true));
            steps(&mut e, 0.5);
            assert_eq!(e.pos[2], 1);
            e.handle(Command::Write {
                group: 2,
                index: 0,
                pad: 0,
                step: 1,
                data: Step {
                    on: true,
                    ..Step::default()
                },
            });
            assert_eq!(e.suppress[2], 1);
            steps(&mut e, 1.0);
            assert_eq!(e.suppress[2], 0);
            assert!(e.project.pattern(2).step(0, 1).on);
        }
    }

    mod when_playing_recorded_notes {
        use super::*;
        use crate::project::Note;

        fn with_chord(start: u32, length: u32) -> Engine {
            let mut project = Project::default();
            let pad = &mut project.groups[1].pads[0];
            pad.source = Source::Fm;
            pad.mode = Mode::Gate;
            pad.sustain = 1.0;
            pad.release = 0.01;
            for pitch in [0, 4, 7] {
                project.groups[1].patterns[0].add_note(Note {
                    pad: 0,
                    start,
                    length,
                    pitch,
                    velocity: 0.9,
                });
            }
            Engine::new(project, RATE)
        }

        #[test]
        fn starts_them_at_their_tick() {
            let mut e = with_chord(TICKS * 2 + TICKS / 2, TICKS * 4);
            e.handle(Command::Play(true));
            steps(&mut e, 2.4);
            assert_eq!(e.report().voices, 0);
            steps(&mut e, 0.2);
            assert_eq!(e.report().voices, 3);
        }

        #[test]
        fn releases_them_after_their_length() {
            let mut e = with_chord(0, TICKS * 2);
            e.handle(Command::Play(true));
            steps(&mut e, 1.5);
            assert_eq!(e.report().voices, 3);
            steps(&mut e, 2.0);
            assert_eq!(e.report().voices, 0);
        }

        #[test]
        fn bounces_them_into_the_file() {
            let e = with_chord(0, TICKS * 8);
            let out = bounce(e.project(), RATE, Bounce::Group(1), 0.0);
            assert!(loud(&out) > 0.05);
        }
    }

    mod when_playing_a_song {
        use super::*;

        #[test]
        fn walks_through_the_entries() {
            let mut project = Project::starter();
            project.song = vec![[0, 0, 0, 0], [1, 2, 0, 0]];
            project.song_mode = true;
            project.groups[0].patterns[1].length = 32;
            let mut e = Engine::new(project, RATE);
            assert_eq!(e.loop_steps(), 48);
            e.handle(Command::Play(true));
            steps(&mut e, 16.5);
            assert_eq!(e.entry, 1);
            assert_eq!(e.current, [1, 2, 0, 0]);
            steps(&mut e, 32.0);
            assert_eq!(e.entry, 0);
        }
    }

    mod when_swapping_data {
        use super::*;

        #[test]
        fn returns_the_old_data_for_cleanup() {
            let mut e = engine();
            e.handle(Command::Pad {
                group: 0,
                pad: 0,
                data: Box::new(Pad::default()),
            });
            assert!(e.project.groups[0].pads[0].source == Source::Sample);
            let trash: Vec<_> = e.take_trash().collect();
            assert!(matches!(&trash[0], Trash::Pad(p) if p.source == Source::Synth));
        }
    }

    mod when_bouncing {
        use super::*;

        #[test]
        fn renders_one_loop_plus_the_tail() {
            let project = Project::starter();
            let out = bounce(&project, RATE, Bounce::Pattern, 1.0);
            let step = RATE as f64 * 60.0 / (project.tempo as f64 * 4.0);
            let expected = (16.0 * step).round() as usize + RATE as usize;
            assert_eq!(out.len(), expected);
            assert!(loud(&out) > 0.1);
        }

        #[test]
        fn renders_a_group_alone() {
            let project = Project::starter();
            let out = bounce(&project, RATE, Bounce::Group(1), 0.0);
            assert_eq!(loud(&out), 0.0);
            let drums = bounce(&project, RATE, Bounce::Group(0), 0.0);
            assert!(loud(&drums) > 0.1);
        }
    }
}
