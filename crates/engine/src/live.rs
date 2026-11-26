use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use rtrb::{Consumer, Producer, RingBuffer};

use crate::engine::{Command, Engine, Report, Trash};
use crate::project::{GROUPS, Project};

const COMMANDS: usize = 1024;
const TRASH: usize = 1024;
const BLOCK: usize = 4096;
const TAP_SECONDS: usize = 4;
const MONITOR_SLACK: f64 = 0.02;

#[derive(Clone, Copy, Debug)]
pub struct Timing {
    pub playing: bool,
    pub phase: f64,
    pub next_time: f64,
    pub pos: usize,
    pub pattern: usize,
}

impl Timing {
    pub fn position(&self, length: usize) -> f64 {
        let length = length.max(1);
        let current = (self.pos + length - 1) % length;
        let into = (1.0 - (self.next_time - self.phase)).clamp(0.0, 0.999);
        current as f64 + into
    }

    pub fn nearest(&self, length: usize) -> usize {
        let length = length.max(1);
        let pos = self.pos % length;
        if self.next_time - self.phase < 0.5 {
            pos
        } else {
            (pos + length - 1) % length
        }
    }
}

#[derive(Default)]
pub struct Status {
    playing: AtomicBool,
    phase: AtomicU64,
    next_time: AtomicU64,
    pos: [AtomicUsize; GROUPS],
    current: [AtomicUsize; GROUPS],
    entry: AtomicUsize,
    hits: [AtomicU32; GROUPS],
    peak: [AtomicU32; 2],
    voices: AtomicUsize,
}

impl Status {
    fn publish(&self, r: &Report) {
        let o = Ordering::Relaxed;
        self.playing.store(r.playing, o);
        self.phase.store(r.phase.to_bits(), o);
        self.next_time.store(r.next_time.to_bits(), o);
        for g in 0..GROUPS {
            self.pos[g].store(r.pos[g], o);
            self.current[g].store(r.current[g], o);
            self.hits[g].fetch_or(r.hits[g], o);
        }
        self.entry.store(r.entry, o);
        for ch in 0..2 {
            self.peak[ch].fetch_max(r.peak[ch].to_bits(), o);
        }
        self.voices.store(r.voices, o);
    }

    pub fn timing(&self, group: usize) -> Timing {
        let o = Ordering::Relaxed;
        Timing {
            playing: self.playing.load(o),
            phase: f64::from_bits(self.phase.load(o)),
            next_time: f64::from_bits(self.next_time.load(o)),
            pos: self.pos[group].load(o),
            pattern: self.current[group].load(o),
        }
    }

    pub fn read(&self) -> Report {
        let o = Ordering::Relaxed;
        Report {
            playing: self.playing.load(o),
            phase: f64::from_bits(self.phase.load(o)),
            next_time: f64::from_bits(self.next_time.load(o)),
            pos: std::array::from_fn(|g| self.pos[g].load(o)),
            current: std::array::from_fn(|g| self.current[g].load(o)),
            entry: self.entry.load(o),
            hits: std::array::from_fn(|g| self.hits[g].swap(0, o)),
            peak: std::array::from_fn(|ch| f32::from_bits(self.peak[ch].swap(0, o))),
            voices: self.voices.load(o),
        }
    }
}

struct Monitor {
    input: Option<Consumer<[f32; 2]>>,
    gain: f32,
    step: f64,
    pos: f64,
    prev: [f32; 2],
    next: [f32; 2],
    slack: usize,
}

impl Monitor {
    fn new() -> Self {
        Self {
            input: None,
            gain: 0.0,
            step: 1.0,
            pos: 0.0,
            prev: [0.0; 2],
            next: [0.0; 2],
            slack: 0,
        }
    }

    fn listen(
        &mut self,
        input: Option<Consumer<[f32; 2]>>,
        rate: u32,
        output: u32,
    ) -> Option<Consumer<[f32; 2]>> {
        self.step = rate.max(1) as f64 / output.max(1) as f64;
        self.slack = (rate as f64 * MONITOR_SLACK) as usize;
        self.pos = 0.0;
        self.prev = [0.0; 2];
        self.next = [0.0; 2];
        std::mem::replace(&mut self.input, input)
    }

    fn mix(&mut self, block: &mut [[f32; 2]]) {
        let Some(input) = self.input.as_mut() else {
            return;
        };
        let needed = (block.len() as f64 * self.step).ceil() as usize;
        let excess = input.slots().saturating_sub(needed + self.slack);
        if excess > 0
            && let Ok(chunk) = input.read_chunk(excess)
        {
            chunk.commit_all();
        }
        if self.gain <= 0.0 {
            return;
        }
        for frame in block.iter_mut() {
            self.pos += self.step;
            while self.pos >= 1.0 {
                self.pos -= 1.0;
                self.prev = self.next;
                self.next = input.pop().unwrap_or([0.0; 2]);
            }
            let t = self.pos as f32;
            for (out, (a, b)) in frame.iter_mut().zip(self.prev.iter().zip(self.next)) {
                *out += (a + (b - a) * t) * self.gain;
            }
        }
    }
}

pub struct Runner {
    engine: Engine,
    monitor: Monitor,
    rate: u32,
    commands: Consumer<Command>,
    trash: Producer<Trash>,
    tap: Producer<[f32; 2]>,
    status: Arc<Status>,
    block: Vec<[f32; 2]>,
}

pub struct Remote {
    pub commands: Producer<Command>,
    pub trash: Consumer<Trash>,
    pub tap: Consumer<[f32; 2]>,
    pub status: Arc<Status>,
}

pub fn live(project: Project, rate: u32) -> (Runner, Remote) {
    let (cp, cc) = RingBuffer::new(COMMANDS);
    let (tp, tc) = RingBuffer::new(TRASH);
    let (ap, ac) = RingBuffer::new(rate as usize * TAP_SECONDS);
    let status = Arc::new(Status::default());
    let runner = Runner {
        engine: Engine::new(project, rate),
        monitor: Monitor::new(),
        rate,
        commands: cc,
        trash: tp,
        tap: ap,
        status: status.clone(),
        block: vec![[0.0; 2]; BLOCK],
    };
    let remote = Remote {
        commands: cp,
        trash: tc,
        tap: ac,
        status,
    };
    (runner, remote)
}

impl Runner {
    pub fn process(&mut self, out: &mut [f32], channels: usize) {
        let _ftz = crate::denormal::NoDenormals::new();
        while let Ok(command) = self.commands.pop() {
            match command {
                Command::Listen { input, rate } => {
                    let old = self.monitor.listen(input, rate, self.rate);
                    if old.is_some() {
                        self.engine.handle(Command::Listen { input: old, rate });
                    }
                }
                Command::Monitor(gain) => self.monitor.gain = gain.max(0.0),
                other => self.engine.handle(other),
            }
        }
        let channels = channels.max(1);
        for chunk in out.chunks_mut(BLOCK * channels) {
            let frames = chunk.len() / channels;
            let block = &mut self.block[..frames];
            let tap = &mut self.tap;
            self.engine.render(block, &mut |f| {
                let _ = tap.push(f);
            });
            self.monitor.mix(block);
            for (dst, src) in chunk.chunks_mut(channels).zip(block.iter()) {
                match dst.len() {
                    1 => dst[0] = (src[0] + src[1]) * 0.5,
                    _ => {
                        dst[0] = src[0];
                        dst[1] = src[1];
                        for d in &mut dst[2..] {
                            *d = 0.0;
                        }
                    }
                }
            }
        }
        let room = self.trash.slots();
        for t in self.engine.take_trash(room) {
            let _ = self.trash.push(t);
        }
        self.status.publish(&self.engine.report());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod when_running_live {
        use super::*;

        #[test]
        fn plays_notes_sent_from_another_thread() {
            let (mut runner, mut remote) = live(Project::starter(), 48000);
            remote
                .commands
                .push(Command::Note {
                    group: 0,
                    pad: 0,
                    note: 0,
                    velocity: 1.0,
                    on: true,
                })
                .ok();
            let mut out = vec![0.0f32; 2048];
            runner.process(&mut out, 2);
            assert!(out.iter().any(|s| s.abs() > 0.05));
            let r = remote.status.read();
            assert_eq!(r.hits[0], 1);
            assert!(r.peak[0] > 0.05);
            assert_eq!(remote.status.read().hits[0], 0);
        }

        #[test]
        fn snaps_a_live_hit_to_the_closest_step() {
            let t = |phase, next_time, pos| Timing {
                playing: true,
                phase,
                next_time,
                pos,
                pattern: 0,
            };
            assert_eq!(t(4.8, 5.0, 5).nearest(16), 5);
            assert_eq!(t(4.2, 5.0, 5).nearest(16), 4);
            assert_eq!(t(0.1, 1.0, 1).nearest(16), 0);
            assert_eq!(t(15.9, 16.0, 0).nearest(16), 0);
            assert_eq!(t(16.2, 17.0, 1).nearest(16), 0);
        }

        #[test]
        fn reads_the_exact_place_in_the_pattern() {
            let t = |phase, next_time, pos| Timing {
                playing: true,
                phase,
                next_time,
                pos,
                pattern: 0,
            };
            assert!((t(4.25, 5.0, 5).position(16) - 4.25).abs() < 1e-9);
            assert!((t(15.5, 16.0, 0).position(16) - 15.5).abs() < 1e-9);
        }

        mod when_monitoring_an_input {
            use super::*;

            fn listen(gain: f32, rate: u32) -> (Runner, Remote, Producer<[f32; 2]>) {
                let (mut runner, mut remote) = live(Project::default(), 48000);
                let (producer, consumer) = RingBuffer::new(4096);
                remote
                    .commands
                    .push(Command::Listen {
                        input: Some(consumer),
                        rate,
                    })
                    .ok();
                remote.commands.push(Command::Monitor(gain)).ok();
                runner.process(&mut [0.0; 2], 2);
                (runner, remote, producer)
            }

            #[test]
            fn plays_the_input_at_the_monitor_gain() {
                let (mut runner, _remote, mut input) = listen(0.5, 48000);
                for _ in 0..256 {
                    input.push([0.4, -0.4]).ok();
                }
                let mut out = vec![0.0f32; 128];
                runner.process(&mut out, 2);
                assert!(out[20..].chunks(2).all(|f| (f[0] - 0.2).abs() < 1e-4));
                assert!(out[20..].chunks(2).all(|f| (f[1] + 0.2).abs() < 1e-4));
            }

            #[test]
            fn stays_silent_with_the_monitor_off() {
                let (mut runner, _remote, mut input) = listen(0.0, 48000);
                for _ in 0..256 {
                    input.push([0.4, 0.4]).ok();
                }
                let mut out = vec![0.0f32; 128];
                runner.process(&mut out, 2);
                assert!(out.iter().all(|s| *s == 0.0));
            }

            #[test]
            fn drops_input_that_lags_behind() {
                let (mut runner, _remote, mut input) = listen(1.0, 48000);
                for _ in 0..4000 {
                    input.push([0.1, 0.1]).ok();
                }
                runner.process(&mut vec![0.0f32; 128], 2);
                assert!(input.slots() > 4096 - 1100);
            }

            #[test]
            fn follows_a_different_input_rate() {
                let (mut runner, _remote, mut input) = listen(1.0, 24000);
                for _ in 0..200 {
                    input.push([0.3, 0.3]).ok();
                }
                runner.process(&mut vec![0.0f32; 128], 2);
                assert_eq!(input.slots(), 4096 - 200 + 32);
            }
        }

        #[test]
        fn fills_extra_channels_with_silence() {
            let (mut runner, _remote) = live(Project::starter(), 48000);
            let mut out = vec![1.0f32; 400];
            runner.process(&mut out, 4);
            assert!(out.chunks(4).all(|f| f[2] == 0.0 && f[3] == 0.0));
        }
    }
}
