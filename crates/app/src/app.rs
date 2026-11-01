use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use eframe::egui;
use flip_engine::project::LENGTHS;
use flip_engine::{
    Bounce, Command, GROUPS, PADS, Pad, Project, Report, STEPS, Sample, Source, Step, Tap, chop,
    sample, store,
};

use crate::audio::{self, Input, Output, Sender};
use crate::midi::{self, Midi, Route};
use crate::params::{self, PAGES, Param, Target};
use crate::settings::Settings;

const UNDO: usize = 50;
const PREROLL: f32 = 0.02;
const MAX_TAKE: f32 = 120.0;
const TAP_FLOOR: f32 = 1e-4;
const TOAST: f64 = 2.5;
pub const FLASH: f64 = 0.18;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Pad,
    Seq,
    Song,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Take {
    Idle,
    Armed,
    Recording,
}

pub struct Capture {
    pub take: Take,
    pub group: usize,
    pub pad: usize,
    pub source: usize,
    pub frames: Vec<[f32; 2]>,
    preroll: VecDeque<[f32; 2]>,
    pub rate: u32,
    pub level: f32,
}

impl Default for Capture {
    fn default() -> Self {
        Self {
            take: Take::Idle,
            group: 0,
            pad: 0,
            source: 0,
            frames: Vec::new(),
            preroll: VecDeque::new(),
            rate: 48000,
            level: 0.0,
        }
    }
}

pub struct App {
    pub project: Project,
    pub path: Option<PathBuf>,
    pub group: usize,
    pub pad: usize,
    pub page: usize,
    pub view: View,
    pub step: Option<usize>,
    pub step_mode: bool,
    pub step_page: usize,
    pub keys: bool,
    pub octave: i8,
    pub rec_seq: bool,
    pub capture: Capture,
    pub source: usize,
    pub sender: Sender,
    pub output: Option<Output>,
    pub input: Option<Input>,
    pub midi: Midi,
    pub route: Arc<Route>,
    pub settings: Settings,
    pub undo: Vec<Project>,
    pub report: Report,
    pub flashes: [[f64; PADS]; GROUPS],
    pub toast: Option<(String, f64)>,
    pub learn: Option<Param>,
    pub setup: bool,
    pub help: bool,
    pub held: Vec<(egui::Key, usize, usize, i8)>,
    pub mouse: Option<(usize, usize, i8)>,
    pub now: f64,
    pub meter: [f32; 2],
    pub drag: Option<(usize, f32)>,
    pub lists: Option<(Vec<String>, Vec<String>, Vec<String>)>,
    pub ctx: egui::Context,
    title: String,
}

impl App {
    pub fn new(ctx: egui::Context) -> Self {
        let settings = Settings::load();
        let mut app = Self {
            project: Project::starter(),
            path: None,
            group: 0,
            pad: 0,
            page: 0,
            view: View::Pad,
            step: None,
            step_mode: false,
            step_page: 0,
            keys: false,
            octave: 0,
            rec_seq: false,
            capture: Capture::default(),
            source: 0,
            sender: Sender::default(),
            output: None,
            input: None,
            midi: Midi::default(),
            route: Arc::new(Route::default()),
            settings,
            undo: Vec::new(),
            report: Report::default(),
            flashes: [[f64::MIN; PADS]; GROUPS],
            toast: None,
            learn: None,
            setup: false,
            help: false,
            held: Vec::new(),
            mouse: None,
            now: 0.0,
            meter: [0.0; 2],
            drag: None,
            lists: None,
            ctx,
            title: String::new(),
        };
        if let Some(last) = app.settings.last.clone()
            && store::is_project(&last)
        {
            match store::load(&last) {
                Ok(p) => {
                    app.project = p;
                    app.path = Some(last);
                }
                Err(e) => app.say(format!("could not open last project: {e}")),
            }
        }
        app.open_output(app.settings.output.clone());
        let wanted = app
            .settings
            .midi
            .clone()
            .or_else(|| midi::ports().into_iter().next());
        if let Some(port) = wanted {
            app.connect_midi(&port);
        }
        app
    }

    pub fn say(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), self.now + TOAST));
    }

    pub fn open_output(&mut self, wanted: Option<String>) {
        self.output = None;
        match audio::open_output(self.project.clone(), wanted.as_deref(), &self.sender) {
            Ok(o) => {
                self.settings.output = Some(o.device.clone());
                self.output = Some(o);
            }
            Err(e) => {
                if wanted.is_some() {
                    self.open_output(None);
                } else {
                    self.say(format!("no audio output: {e}"));
                }
            }
        }
    }

    pub fn open_input(&mut self, wanted: Option<String>) -> bool {
        self.input = None;
        match audio::open_input(wanted.as_deref()) {
            Ok(i) => {
                self.settings.input = Some(i.device.clone());
                self.input = Some(i);
                true
            }
            Err(e) => {
                self.say(format!("no audio input: {e}"));
                false
            }
        }
    }

    pub fn connect_midi(&mut self, port: &str) {
        let result = self.midi.connect(
            port,
            self.route.clone(),
            self.sender.clone(),
            self.ctx.clone(),
        );
        match result {
            Ok(()) => self.settings.midi = Some(port.to_string()),
            Err(e) => self.say(format!("midi: {e}")),
        }
    }

    pub fn rate(&self) -> u32 {
        self.output.as_ref().map(|o| o.rate).unwrap_or(48000)
    }

    pub fn send(&self, c: Command) {
        self.sender.send(c);
    }

    pub fn sync_pad(&self, group: usize, pad: usize) {
        self.send(Command::Pad {
            group,
            pad,
            data: Box::new(self.project.groups[group].pads[pad].clone()),
        });
    }

    pub fn sync_pattern(&self, group: usize, index: usize) {
        self.send(Command::Pattern {
            group,
            index,
            data: Box::new(self.project.groups[group].patterns[index].clone()),
        });
    }

    pub fn sync_all(&self) {
        self.send(Command::Load(Box::new(self.project.clone())));
    }

    pub fn remember(&mut self) {
        if self.undo.len() >= UNDO {
            self.undo.remove(0);
        }
        self.undo.push(self.project.clone());
    }

    pub fn undo(&mut self) {
        match self.undo.pop() {
            Some(p) => {
                self.project = p;
                self.sync_all();
                self.say("undo");
            }
            None => self.say("nothing to undo"),
        }
    }

    pub fn label(&mut self, code: egui::Key, key: egui::Key) {
        let (code, key) = (code.name().to_string(), key.symbol_or_name().to_string());
        match self.settings.labels.iter_mut().find(|(c, _)| *c == code) {
            Some((_, k)) if *k == key => {}
            Some((_, k)) => *k = key,
            None => self.settings.labels.push((code, key)),
        }
    }

    pub fn key_label(&self, code: egui::Key) -> String {
        self.settings
            .labels
            .iter()
            .find(|(c, _)| c == code.name())
            .map(|(_, k)| k.clone())
            .unwrap_or_else(|| code.symbol_or_name().to_string())
    }

    pub fn current_pad(&self) -> &Pad {
        &self.project.groups[self.group].pads[self.pad]
    }

    pub fn editing(&self) -> usize {
        self.project.groups[self.group].pattern
    }

    pub fn select_group(&mut self, g: usize) {
        self.group = g.min(GROUPS - 1);
        self.route.group.store(self.group, Ordering::Relaxed);
    }

    pub fn select_pad(&mut self, p: usize) {
        self.pad = p.min(PADS - 1);
        self.route.pad.store(self.pad, Ordering::Relaxed);
    }

    pub fn set_keys(&mut self, on: bool) {
        self.keys = on;
        self.route.keys.store(on, Ordering::Relaxed);
    }

    pub fn select_pattern(&mut self, i: usize) {
        self.project.groups[self.group].pattern = i;
        self.step = None;
        self.send(Command::Select {
            group: self.group,
            pattern: i,
        });
    }

    pub fn toggle_play(&mut self) {
        let on = !self.report.playing;
        self.send(Command::Play(on));
        self.report.playing = on;
    }

    pub fn toggle_metronome(&mut self) {
        self.project.metronome = !self.project.metronome;
        self.send(Command::Metronome(self.project.metronome));
    }

    pub fn toggle_mute(&mut self, g: usize) {
        let group = &mut self.project.groups[g];
        group.mute = !group.mute;
        let (volume, mute) = (group.volume, group.mute);
        self.send(Command::Mix {
            group: g,
            volume,
            mute,
        });
    }

    pub fn press(&mut self, group: usize, pad: usize, note: i8, velocity: f32, sent: bool) {
        if !sent {
            self.send(Command::Note {
                group,
                pad,
                note,
                velocity,
                on: true,
            });
        }
        self.flashes[group][pad] = self.now;
        if group == self.group && !self.keys {
            self.select_pad(pad);
        }
        if self.rec_seq && self.report.playing {
            self.record_hit(group, pad, note, velocity);
        }
    }

    pub fn lift(&mut self, group: usize, pad: usize, note: i8, sent: bool) {
        if !sent {
            self.send(Command::Note {
                group,
                pad,
                note,
                velocity: 0.0,
                on: false,
            });
        }
    }

    fn record_hit(&mut self, group: usize, pad: usize, note: i8, velocity: f32) {
        let Some(out) = &self.output else {
            return;
        };
        let timing = out.status.timing(group);
        let index = timing.pattern;
        let length = self.project.groups[group].patterns[index].length;
        let step = timing.nearest(length);
        let data = Step {
            on: true,
            velocity,
            pitch: note,
            chance: 1.0,
        };
        *self.project.groups[group].patterns[index].step_mut(pad, step) = data;
        self.send(Command::Write {
            group,
            index,
            pad,
            step,
            data,
        });
    }

    pub fn toggle_step(&mut self, pad: usize, step: usize) {
        let (g, i) = (self.group, self.editing());
        let s = self.project.groups[g].patterns[i].step_mut(pad, step);
        s.on = !s.on;
        let data = *s;
        self.step = data.on.then_some(step);
        self.send(Command::Write {
            group: g,
            index: i,
            pad,
            step,
            data,
        });
    }

    pub fn clear_steps(&mut self) {
        self.remember();
        let (g, i) = (self.group, self.editing());
        self.project.groups[g].patterns[i].clear_pad(self.pad);
        self.sync_pattern(g, i);
        self.say(format!("cleared {}", self.pad_name(g, self.pad)));
    }

    pub fn pad_name(&self, g: usize, p: usize) -> String {
        format!("{}-{:02}", (b'A' + g as u8) as char, p + 1)
    }

    pub fn param(&self, p: Param) -> Option<f32> {
        let pad = self.current_pad();
        match p.target() {
            Target::Pad => Some(p.pad_get(pad)),
            Target::Step => {
                let step = self.step?;
                let s = self.project.pattern(self.group).step(self.pad, step);
                s.on.then(|| p.step_get(s))
            }
            Target::Global => Some(match p {
                Param::Length => params::length_index(self.project.pattern(self.group).length),
                Param::Tempo => self.project.tempo,
                Param::Swing => self.project.swing,
                Param::GroupVolume => self.project.groups[self.group].volume,
                Param::Master => self.project.volume,
                Param::Threshold => self.settings.threshold,
                Param::Capture => self.source as f32,
                _ => 0.0,
            }),
        }
    }

    pub fn set_param(&mut self, p: Param, v: f32) {
        let (g, pad) = (self.group, self.pad);
        match p.target() {
            Target::Pad => {
                p.pad_set(&mut self.project.groups[g].pads[pad], v);
                self.sync_pad(g, pad);
            }
            Target::Step => {
                let Some(step) = self.step else {
                    return;
                };
                let i = self.editing();
                let s = self.project.groups[g].patterns[i].step_mut(pad, step);
                p.step_set(s, v);
                let data = *s;
                self.send(Command::Write {
                    group: g,
                    index: i,
                    pad,
                    step,
                    data,
                });
            }
            Target::Global => match p {
                Param::Length => {
                    let i = self.editing();
                    let length = LENGTHS[(v.round() as usize).min(3)];
                    self.project.groups[g].patterns[i].length = length;
                    self.step_page = self.step_page.min(length / 16 - 1);
                    self.sync_pattern(g, i);
                }
                Param::Tempo => {
                    self.project.tempo = v;
                    self.send(Command::Tempo(v));
                }
                Param::Swing => {
                    self.project.swing = v;
                    self.send(Command::Swing(v));
                }
                Param::GroupVolume => {
                    let group = &mut self.project.groups[g];
                    group.volume = v;
                    let (volume, mute) = (group.volume, group.mute);
                    self.send(Command::Mix {
                        group: g,
                        volume,
                        mute,
                    });
                }
                Param::Master => {
                    self.project.volume = v;
                    self.send(Command::Volume(v));
                }
                Param::Threshold => self.settings.threshold = v,
                Param::Capture => self.source = v.round() as usize,
                _ => {}
            },
        }
    }

    pub fn page_params(&self) -> [Param; 4] {
        PAGES[self.page].1
    }

    pub fn control(&mut self, cc: u8, value: u8) {
        if let Some(p) = self.learn.take() {
            self.settings.learn(cc, p);
            self.say(format!("cc {cc} -> {}", p.spec().name));
            return;
        }
        if let Some(p) = self.settings.learned(cc) {
            self.set_param(p, p.value_at(value as f32 / 127.0));
        }
    }

    pub fn arm(&mut self) {
        if self.capture.take != Take::Idle {
            self.stop_capture();
            return;
        }
        let source = self.source;
        if source == 0 {
            if self.input.is_none() && !self.open_input(self.settings.input.clone()) {
                return;
            }
            if let Some(i) = self.input.as_mut() {
                i.drain();
            }
        } else {
            let tap = if source == 1 {
                Tap::Master
            } else {
                Tap::Group(source - 2)
            };
            if let Some(o) = self.output.as_mut() {
                while o.tap.pop().is_ok() {}
            }
            self.send(Command::Tap(Some(tap)));
        }
        let rate = if source == 0 {
            self.input.as_ref().map(|i| i.rate)
        } else {
            None
        }
        .unwrap_or_else(|| self.rate());
        self.capture = Capture {
            take: Take::Armed,
            group: self.group,
            pad: self.pad,
            source,
            rate,
            ..Capture::default()
        };
        self.say(format!("armed {}", self.pad_name(self.group, self.pad)));
    }

    pub fn cancel_capture(&mut self) {
        if self.capture.source != 0 {
            self.send(Command::Tap(None));
        }
        self.capture = Capture::default();
    }

    pub fn stop_capture(&mut self) {
        let take = std::mem::take(&mut self.capture);
        if take.source != 0 {
            self.send(Command::Tap(None));
        }
        if take.take != Take::Recording || take.frames.is_empty() {
            self.say("nothing recorded");
            return;
        }
        let mut frames = take.frames;
        if take.source == 0 {
            sample::normalize(&mut frames);
        }
        let label = if take.source == 0 { "rec" } else { "resample" };
        let name = format!("{label}-{}", self.project.samples().len() + 1);
        let s = Sample::new(name, take.rate, frames);
        self.place(take.group, take.pad, s);
    }

    pub fn place(&mut self, group: usize, pad: usize, s: Sample) {
        self.remember();
        let seconds = s.seconds();
        let name = s.name.clone();
        let p = &mut self.project.groups[group].pads[pad];
        p.sample = Some(Arc::new(s));
        p.name.clear();
        p.source = Source::Sample;
        p.start = 0.0;
        p.end = 1.0;
        p.reverse = false;
        self.sync_pad(group, pad);
        self.say(format!(
            "{name} ({seconds:.1}s) -> {}",
            self.pad_name(group, pad)
        ));
    }

    fn pump_capture(&mut self) {
        let source = self.capture.source;
        let active = self.capture.take != Take::Idle;
        let mut incoming: Vec<[f32; 2]> = Vec::new();
        if let Some(i) = self.input.as_mut() {
            while let Ok(f) = i.frames.pop() {
                if active && source == 0 {
                    incoming.push(f);
                }
            }
        }
        if let Some(o) = self.output.as_mut() {
            while let Ok(f) = o.tap.pop() {
                if active && source != 0 {
                    incoming.push(f);
                }
            }
        }
        if !active {
            return;
        }
        let threshold = if source == 0 {
            self.settings.threshold
        } else {
            TAP_FLOOR
        };
        let c = &mut self.capture;
        let pre = (c.rate as f32 * PREROLL) as usize;
        let max = (c.rate as f32 * MAX_TAKE) as usize;
        let mut level = 0.0f32;
        for f in incoming {
            let v = f[0].abs().max(f[1].abs());
            level = level.max(v);
            match c.take {
                Take::Armed => {
                    c.preroll.push_back(f);
                    if c.preroll.len() > pre {
                        c.preroll.pop_front();
                    }
                    if v >= threshold {
                        c.take = Take::Recording;
                        c.frames.extend(c.preroll.drain(..));
                    }
                }
                Take::Recording => c.frames.push(f),
                Take::Idle => {}
            }
        }
        c.level = c.level * 0.85 + level * 0.15;
        c.level = c.level.max(level);
        if c.frames.len() >= max {
            self.stop_capture();
        }
    }

    pub fn chop(&mut self) {
        let Some(s) = self.current_pad().sample.clone() else {
            self.say("no sample to chop");
            return;
        };
        self.remember();
        let base = self.current_pad().clone();
        let (a, b) = (base.start.min(base.end), base.start.max(base.end));
        let region: Vec<[f32; 2]> = {
            let n = s.len();
            let (i, j) = ((a * n as f32) as usize, ((b * n as f32) as usize).min(n));
            s.frames[i..j.max(i)].to_vec()
        };
        let probe = Sample::new("probe", s.rate, region);
        let cuts = chop::slices(&probe);
        let g = self.group;
        for (i, (x, y)) in cuts.iter().enumerate().take(PADS) {
            let pad = &mut self.project.groups[g].pads[i];
            *pad = Pad {
                start: a + x * (b - a),
                end: a + y * (b - a),
                mode: flip_engine::Mode::OneShot,
                choke: 0,
                ..base.clone()
            };
            self.sync_pad(g, i);
        }
        self.select_pad(0);
        self.say(format!("chopped into {}", cuts.len().min(PADS)));
    }

    pub fn bounce(&mut self) {
        let rate = self.rate();
        let frames = flip_engine::bounce(&self.project, rate, Bounce::Pattern, 0.0);
        if sample::peak(&frames) <= 0.0 {
            self.say("pattern is silent");
            return;
        }
        let g = self.group;
        let target = (0..PADS)
            .find(|&p| self.project.groups[g].pads[p].silent())
            .unwrap_or(self.pad);
        let name = format!("bounce-{}", self.project.samples().len() + 1);
        self.place(g, target, Sample::new(name, rate, frames));
        self.select_pad(target);
    }

    pub fn export(&mut self) {
        let song = self.project.song_mode && !self.project.song.is_empty();
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export WAV")
            .add_filter("WAV", &["wav"])
            .set_file_name(if song { "song.wav" } else { "pattern.wav" })
            .save_file()
        else {
            return;
        };
        let rate = self.rate();
        let what = if song { Bounce::Song } else { Bounce::Pattern };
        let frames = flip_engine::bounce(&self.project, rate, what, 2.0);
        match sample::save(&path, rate, &frames) {
            Ok(()) => self.say(format!("exported {}", file_name(&path))),
            Err(e) => self.say(format!("export failed: {e}")),
        }
    }

    pub fn save(&mut self, choose: bool) {
        let path = match (&self.path, choose) {
            (Some(p), false) => p.clone(),
            _ => {
                let base = crate::settings::projects();
                let _ = std::fs::create_dir_all(&base);
                let Some(p) = rfd::FileDialog::new()
                    .set_title("Save project")
                    .set_directory(&base)
                    .set_file_name("untitled")
                    .save_file()
                else {
                    return;
                };
                p
            }
        };
        match store::save(&self.project, &path) {
            Ok(()) => {
                self.say(format!("saved {}", file_name(&path)));
                self.settings.last = Some(path.clone());
                self.settings.save();
                self.path = Some(path);
            }
            Err(e) => self.say(format!("save failed: {e}")),
        }
    }

    pub fn open(&mut self) {
        let Some(dir) = rfd::FileDialog::new()
            .set_title("Open project folder")
            .set_directory(crate::settings::projects())
            .pick_folder()
        else {
            return;
        };
        self.open_path(&dir);
    }

    pub fn open_path(&mut self, dir: &Path) {
        match store::load(dir) {
            Ok(p) => {
                self.project = p;
                self.path = Some(dir.to_path_buf());
                self.undo.clear();
                self.step = None;
                self.sync_all();
                self.settings.last = Some(dir.to_path_buf());
                self.settings.save();
                self.say(format!("opened {}", file_name(dir)));
            }
            Err(e) => self.say(format!("open failed: {e}")),
        }
    }

    pub fn new_project(&mut self) {
        self.remember();
        self.project = Project::starter();
        self.path = None;
        self.step = None;
        self.sync_all();
        self.say("new project");
    }

    pub fn load_files(&mut self, paths: Vec<PathBuf>) {
        if let [one] = paths.as_slice()
            && store::is_project(one)
        {
            self.open_path(one);
            return;
        }
        let mut slot = self.pad;
        for path in paths {
            match sample::load(&path) {
                Ok(s) => {
                    self.place(self.group, slot, s);
                    slot = (slot + 1) % PADS;
                }
                Err(e) => self.say(format!("{}: {e}", file_name(&path))),
            }
        }
    }

    pub fn load_dialog(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .set_title("Load samples")
            .add_filter(
                "Audio",
                &["wav", "aif", "aiff", "mp3", "flac", "ogg", "m4a"],
            )
            .pick_files()
        {
            self.load_files(paths);
        }
    }

    pub fn song_add(&mut self) {
        let entry = std::array::from_fn(|g| self.project.groups[g].pattern);
        self.project.song.push(entry);
        self.send(Command::Song(Box::new(self.project.song.clone())));
    }

    pub fn song_remove(&mut self) {
        self.project.song.pop();
        self.send(Command::Song(Box::new(self.project.song.clone())));
    }

    pub fn toggle_song_mode(&mut self) {
        self.project.song_mode = !self.project.song_mode;
        self.send(Command::SongMode(self.project.song_mode));
    }

    pub fn tick(&mut self, now: f64) {
        self.now = now;
        if let Some(o) = self.output.as_mut() {
            while o.trash.pop().is_ok() {}
            self.report = o.status.read();
        }
        for g in 0..GROUPS {
            for p in 0..PADS {
                if self.report.hits[g] & (1 << p) != 0 {
                    self.flashes[g][p] = now;
                }
            }
        }
        for ch in 0..2 {
            self.meter[ch] = (self.meter[ch] * 0.9).max(self.report.peak[ch]);
        }
        for event in self.midi.poll() {
            match event {
                midi::Event::Note {
                    group,
                    pad,
                    note,
                    velocity,
                    on,
                } => {
                    if on {
                        self.press(group, pad, note, velocity, true);
                    } else {
                        self.lift(group, pad, note, true);
                    }
                }
                midi::Event::Control { cc, value } => self.control(cc, value),
            }
        }
        self.pump_capture();
        if self.toast.as_ref().is_some_and(|(_, until)| now > *until) {
            self.toast = None;
        }
        let name = self
            .path
            .as_deref()
            .map(file_name)
            .unwrap_or_else(|| "untitled".into());
        let title = format!("flip - {name}");
        if title != self.title {
            self.ctx
                .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    pub fn busy(&self) -> bool {
        self.report.playing
            || self.capture.take != Take::Idle
            || self.report.voices > 0
            || self.toast.is_some()
            || self.meter[0] > 1e-3
    }

    pub fn steps_visible(&self) -> usize {
        self.project.pattern(self.group).length.min(STEPS)
    }
}

pub fn file_name(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_string()
}
