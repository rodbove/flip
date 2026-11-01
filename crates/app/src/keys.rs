use eframe::egui::{self, Key};

use crate::app::{App, Take, View};

const VELOCITY: f32 = 0.85;

pub const PAD_KEYS: [Key; 16] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Q,
    Key::W,
    Key::E,
    Key::R,
    Key::A,
    Key::S,
    Key::D,
    Key::F,
    Key::Z,
    Key::X,
    Key::C,
    Key::V,
];

const PIANO: [(Key, i8); 13] = [
    (Key::A, 0),
    (Key::W, 1),
    (Key::S, 2),
    (Key::E, 3),
    (Key::D, 4),
    (Key::F, 5),
    (Key::T, 6),
    (Key::G, 7),
    (Key::Y, 8),
    (Key::H, 9),
    (Key::U, 10),
    (Key::J, 11),
    (Key::K, 12),
];

pub const HELP: &[(&str, &str)] = &[
    ("4x4 left block", "play pads"),
    ("SHIFT + pad", "select pad, no sound"),
    ("TAB", "keys mode (piano)"),
    ("A W S E ... K", "piano keys"),
    ("Z / X", "octave (keys mode)"),
    ("5 / 6 / 7", "screen pad/seq/song"),
    ("SPACE", "play / stop"),
    ("ENTER", "record into pattern"),
    ("B", "sample (again: stop)"),
    ("M", "metronome"),
    ("[ ]", "pattern -/+"),
    ("LEFT / RIGHT", "group -/+"),
    ("UP / DOWN", "tempo -/+"),
    ("BACKSPACE", "clear pad steps"),
    ("CMD Z S O E", "undo save open export"),
    ("ESC", "cancel / close"),
    ("/", "this help"),
];

fn note_for(app: &App, key: Key) -> Option<(usize, usize, i8)> {
    if app.keys {
        let (_, semis) = PIANO.iter().find(|(k, _)| *k == key)?;
        Some((app.group, app.pad, semis + app.octave * 12))
    } else {
        let pad = PAD_KEYS.iter().position(|k| *k == key)?;
        Some((app.group, pad, 0))
    }
}

pub fn handle(app: &mut App, ctx: &egui::Context) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let events = ctx.input(|i| i.events.clone());
    for event in events {
        let egui::Event::Key {
            key,
            physical_key,
            pressed,
            repeat,
            modifiers,
            ..
        } = event
        else {
            continue;
        };
        let code = physical_key.unwrap_or(key);
        if pressed && physical_key.is_some() {
            app.label(code, key);
        }
        if !pressed {
            let mut i = 0;
            while i < app.held.len() {
                if app.held[i].0 == code {
                    let (_, g, p, n) = app.held.remove(i);
                    app.lift(g, p, n, false);
                } else {
                    i += 1;
                }
            }
            continue;
        }
        if repeat {
            continue;
        }
        if modifiers.command {
            match key {
                Key::Z => app.undo(),
                Key::S => app.save(modifiers.shift),
                Key::O => app.open(),
                Key::E => app.export(),
                Key::N => app.new_project(),
                _ => {}
            }
            continue;
        }
        if app.keys && matches!(code, Key::Z | Key::X) {
            app.octave = (app.octave + if code == Key::Z { -1 } else { 1 }).clamp(-4, 4);
            app.say(format!("octave {:+}", app.octave));
            continue;
        }
        if let Some((g, p, n)) = note_for(app, code) {
            if modifiers.shift && !app.keys {
                app.select_pad(p);
            } else {
                app.press(g, p, n, VELOCITY, false);
                app.held.push((code, g, p, n));
            }
            continue;
        }
        match code {
            Key::Space => app.toggle_play(),
            Key::Enter => {
                app.rec_seq = !app.rec_seq;
                app.say(if app.rec_seq {
                    "pattern record on"
                } else {
                    "pattern record off"
                });
            }
            Key::B => app.arm(),
            Key::M => app.toggle_metronome(),
            Key::Tab => {
                let on = !app.keys;
                app.set_keys(on);
                app.say(if on { "keys mode" } else { "pads mode" });
            }
            Key::OpenBracket => {
                let i = app.editing();
                app.select_pattern((i + 15) % 16);
            }
            Key::CloseBracket => {
                let i = app.editing();
                app.select_pattern((i + 1) % 16);
            }
            Key::ArrowLeft => app.select_group((app.group + 3) % 4),
            Key::ArrowRight => app.select_group((app.group + 1) % 4),
            Key::ArrowUp | Key::ArrowDown => {
                let d = if key == Key::ArrowUp { 1.0 } else { -1.0 };
                let t = (app.project.tempo.round() + d).clamp(40.0, 240.0);
                app.set_param(crate::params::Param::Tempo, t);
            }
            Key::Backspace | Key::Delete => app.clear_steps(),
            Key::Escape => {
                if app.capture.take != Take::Idle {
                    app.cancel_capture();
                    app.say("recording cancelled");
                }
                app.help = false;
                app.setup = false;
                app.learn = None;
            }
            Key::Slash | Key::Questionmark => app.help = !app.help,
            Key::Num5 => app.view = View::Pad,
            Key::Num6 => app.view = View::Seq,
            Key::Num7 => app.view = View::Song,
            _ => {}
        }
    }
}
