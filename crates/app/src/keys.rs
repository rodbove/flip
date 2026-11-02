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
    ("5 6 7 8", "pad seq song roll"),
    ("- / =", "keys / mood screen"),
    ("L", "dice: new sound, same mood"),
    ("SPACE", "play / stop"),
    ("ENTER", "record into pattern"),
    ("9 / 0", "record grid / quantize notes"),
    ("B", "sample (again: stop)"),
    ("M", "metronome"),
    ("T / TEMPO", "type bpm, T again to set"),
    ("N / STEPS", "type loop steps, N again"),
    ("[ ]", "pattern -/+"),
    ("LEFT / RIGHT", "group -/+"),
    ("UP / DOWN", "tempo -/+"),
    ("BACKSPACE", "clear pad steps"),
    ("CMD Z S O E", "undo save open export"),
    ("P / SHIFT P", "presets / save preset"),
    (", .", "previous / next preset"),
    ("ESC", "cancel / close"),
    ("/", "this help"),
];

fn digit(key: Key) -> Option<char> {
    let all = [
        Key::Num0,
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
        Key::Num9,
    ];
    let n = all.iter().position(|k| *k == key)?;
    char::from_digit(n as u32, 10)
}

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
        if app.steps_entry.is_some() {
            match code {
                Key::Enter | Key::N => app.steps_key(),
                Key::Escape => app.steps_cancel(),
                Key::Backspace | Key::Delete => app.steps_back(),
                _ => {
                    if let Some(d) = digit(code) {
                        app.steps_type(d);
                    }
                }
            }
            continue;
        }
        if app.tempo_entry.is_some() {
            match code {
                Key::Enter | Key::T => app.tempo_key(),
                Key::Escape => app.tempo_cancel(),
                Key::Backspace | Key::Delete => app.tempo_back(),
                Key::Period | Key::Comma => app.tempo_type('.'),
                _ => {
                    if let Some(d) = digit(code) {
                        app.tempo_type(d);
                    }
                }
            }
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
        if app.browser {
            let handled = match code {
                Key::ArrowUp => {
                    app.library.move_cursor(-1);
                    true
                }
                Key::ArrowDown => {
                    app.library.move_cursor(1);
                    true
                }
                Key::ArrowLeft => {
                    app.library.move_category(-1);
                    true
                }
                Key::ArrowRight => {
                    app.library.move_category(1);
                    true
                }
                Key::Enter => {
                    if let Some(i) = app.library.selected() {
                        app.load_preset(i);
                    }
                    true
                }
                _ => false,
            };
            if handled {
                continue;
            }
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
                app.press(g, p, n, VELOCITY, false, app.keys);
                app.held.push((code, g, p, n));
            }
            continue;
        }
        match code {
            Key::Space => app.toggle_play(),
            Key::Enter => app.toggle_rec(),
            Key::B => app.arm(),
            Key::T => app.tempo_key(),
            Key::N => app.steps_key(),
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
                app.browser = false;
                app.setup = false;
                app.learn = None;
            }
            Key::Slash | Key::Questionmark => app.help = !app.help,
            Key::P if modifiers.shift => app.naming = Some(app.current_pad().label()),
            Key::P => {
                app.browser = !app.browser;
                app.help = false;
            }
            Key::Comma => app.step_preset(-1),
            Key::Period => app.step_preset(1),
            Key::Num5 => app.view = View::Pad,
            Key::Num6 => app.view = View::Seq,
            Key::Num7 => app.view = View::Song,
            Key::Num8 => app.view = View::Roll,
            Key::Minus => app.view = View::Keys,
            Key::Equals | Key::Plus => app.view = View::Mood,
            Key::L => app.dice(),
            Key::Num9 => app.cycle_grid(),
            Key::Num0 => app.quantize_notes(),
            _ => {}
        }
    }
}
