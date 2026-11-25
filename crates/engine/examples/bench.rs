use std::time::{Duration, Instant};

use flip_engine::keys::PATCHES;
use flip_engine::{Command, Engine, GROUPS, KEYS, Project, mood};

const RATE: u32 = 48000;
const BLOCK: usize = 256;
const SECONDS: usize = 10;
const CHORD: [i8; 8] = [-12, -5, 0, 4, 7, 11, 14, 19];

struct Result {
    name: String,
    total: Duration,
    worst: Duration,
    voices: usize,
}

fn project(index: usize) -> Project {
    let mut p = Project::default();
    mood::apply(&mut p, index);
    mood::beat(&mut p, index, 0);
    p
}

fn run(name: String, project: Project, chord: Option<usize>) -> Result {
    let mut engine = Engine::new(project, RATE);
    engine.handle(Command::Play(true));
    if let Some(patch) = chord {
        for g in 0..GROUPS {
            engine.handle(Command::Keys {
                group: g,
                data: Box::new(PATCHES[patch].keys.clone()),
            });
        }
        for (g, note) in (0..GROUPS).flat_map(|g| CHORD.map(|n| (g, n))) {
            engine.handle(Command::Note {
                group: g,
                pad: KEYS,
                note,
                velocity: 0.8,
                on: true,
            });
        }
    }
    let mut out = vec![[0.0f32; 2]; BLOCK];
    let blocks = RATE as usize * SECONDS / BLOCK;
    let mut total = Duration::ZERO;
    let mut worst = Duration::ZERO;
    let mut voices = 0;
    for _ in 0..blocks {
        let t = Instant::now();
        engine.render(&mut out, &mut |_| {});
        let dt = t.elapsed();
        total += dt;
        worst = worst.max(dt);
        engine.take_trash(usize::MAX).for_each(drop);
        voices = voices.max(engine.report().voices);
    }
    Result {
        name,
        total,
        worst,
        voices,
    }
}

fn main() {
    let _ftz = flip_engine::denormal::NoDenormals::new();
    let deadline = Duration::from_secs_f64(BLOCK as f64 / RATE as f64);
    let heavy = PATCHES
        .iter()
        .position(|p| p.name.eq_ignore_ascii_case("Supersaw Pad"))
        .unwrap_or(0);
    let mut results: Vec<Result> = mood::MOODS
        .iter()
        .enumerate()
        .map(|(i, m)| run(m.name.to_string(), project(i), None))
        .collect();
    let worst_mood = (0..mood::MOODS.len())
        .max_by_key(|&i| results[i].total)
        .unwrap_or(0);
    results.push(run(
        format!(
            "{} + 32 NOTE {}",
            mood::MOODS[worst_mood].name,
            PATCHES[heavy].name
        ),
        project(worst_mood),
        Some(heavy),
    ));
    println!(
        "{SECONDS} s of audio, {RATE} Hz, {BLOCK} frame blocks, deadline {:.2} ms",
        deadline.as_secs_f64() * 1000.0
    );
    println!(
        "{:<34} {:>8} {:>10} {:>10} {:>7}",
        "RUN", "SPEED", "AVG LOAD", "WORST", "VOICES"
    );
    for r in &results {
        let audio = SECONDS as f64;
        let speed = audio / r.total.as_secs_f64();
        let load = 100.0 / speed;
        let worst = 100.0 * r.worst.as_secs_f64() / deadline.as_secs_f64();
        println!(
            "{:<34} {:>7.1}x {:>9.1}% {:>9.1}% {:>7}",
            r.name, speed, load, worst, r.voices
        );
    }
}
