use std::f32::consts::PI;

use eframe::egui::{
    self, Align2, Color32, FontId, Id, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui,
    pos2, vec2,
};
use flip_engine::{GROUPS, PADS, PATTERNS};

use crate::app::{App, FLASH, Take, View};
use crate::keys::{HELP, PAD_KEYS};
use crate::params::{PAGES, Param};
use crate::theme::*;

const W: f32 = 1100.0;
const H: f32 = 700.0;
const DRAG_RANGE: f32 = 160.0;
const FINE: f32 = 6.0;
const SCROLL_RANGE: f32 = 800.0;
const MARKER_START: usize = 10;
const MARKER_END: usize = 11;

struct L {
    o: Pos2,
    k: f32,
}

impl L {
    fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.p(x, y), vec2(w * self.k, h * self.k))
    }

    fn p(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.o.x + x * self.k, self.o.y + y * self.k)
    }

    fn f(&self, size: f32) -> FontId {
        FontId::monospace(size * self.k)
    }

    fn s(&self, v: f32) -> f32 {
        v * self.k
    }
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let c = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(c(a.r(), b.r()), c(a.g(), b.g()), c(a.b(), b.b()))
}

fn key(
    ui: &Ui,
    l: &L,
    id: impl std::hash::Hash + std::fmt::Debug,
    rect: Rect,
    label: &str,
    lit: bool,
    dark: bool,
) -> Response {
    let resp = ui.interact(rect, Id::unique(id), Sense::click());
    let down = resp.is_pointer_button_down_on();
    let painter = ui.painter();
    let radius = l.s(5.0);
    let (face, shadow, ink) = if dark {
        (KEY, KEY_SHADOW, KEY_INK)
    } else {
        (PAD, PAD_SHADOW, INK)
    };
    painter.rect_filled(rect.translate(vec2(0.0, l.s(3.0))), radius, shadow);
    let face_rect = rect.translate(vec2(0.0, if down { l.s(2.0) } else { 0.0 }));
    let face = if resp.hovered() {
        mix(face, ORANGE, 0.08)
    } else {
        face
    };
    painter.rect_filled(face_rect, radius, face);
    if lit {
        painter.circle_filled(
            face_rect.left_top() + vec2(l.s(8.0), l.s(8.0)),
            l.s(3.0),
            ORANGE,
        );
    }
    painter.text(
        face_rect.center(),
        Align2::CENTER_CENTER,
        label,
        l.f(11.0),
        ink,
    );
    resp
}

pub fn draw(app: &mut App, ui: &mut Ui) {
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, BACKDROP);
    let k = (full.width() / W).min(full.height() / H);
    let o = full.center() - vec2(W * k / 2.0, H * k / 2.0);
    let l = L { o, k };
    let body = l.r(10.0, 10.0, 1080.0, 680.0);
    ui.painter()
        .rect_filled(body.translate(vec2(0.0, l.s(4.0))), l.s(20.0), KEY_SHADOW);
    ui.painter().rect_filled(body, l.s(20.0), BODY);
    ui.painter().rect_stroke(
        body,
        l.s(20.0),
        Stroke::new(l.s(1.0), BODY_EDGE),
        StrokeKind::Inside,
    );
    header(app, ui, &l);
    screen(app, ui, &l);
    knobs(app, ui, &l);
    pages(app, ui, &l);
    functions(app, ui, &l);
    groups(app, ui, &l);
    pads(app, ui, &l);
    patterns(app, ui, &l);
    footer(app, ui, &l);
}

fn header(app: &mut App, ui: &Ui, l: &L) {
    let painter = ui.painter();
    painter.text(
        l.p(40.0, 56.0),
        Align2::LEFT_CENTER,
        "flip",
        FontId::proportional(l.s(36.0)),
        INK,
    );
    painter.text(
        l.p(118.0, 50.0),
        Align2::LEFT_CENTER,
        "PAD SAMPLER",
        l.f(10.0),
        INK,
    );
    painter.text(
        l.p(118.0, 64.0),
        Align2::LEFT_CENTER,
        "F-1  4 x 16  64 STEP",
        l.f(10.0),
        DIM_INK,
    );
    let buttons = ["NEW", "OPEN", "SAVE", "EXPORT", "UNDO", "SETUP"];
    let w = (382.0 - 5.0 * 6.0) / 6.0;
    for (i, name) in buttons.iter().enumerate() {
        let rect = l.r(674.0 + i as f32 * (w + 6.0), 40.0, w, 30.0);
        let lit = *name == "SETUP" && app.setup;
        if key(ui, l, ("head", i), rect, name, lit, false).clicked() {
            match i {
                0 => app.new_project(),
                1 => app.open(),
                2 => app.save(false),
                3 => app.export(),
                4 => app.undo(),
                _ => {
                    app.setup = !app.setup;
                    app.lists = None;
                }
            }
        }
    }
}

fn screen(app: &mut App, ui: &Ui, l: &L) {
    let outer = l.r(40.0, 96.0, 600.0, 280.0);
    let painter = ui.painter();
    painter.rect_filled(outer, l.s(12.0), KEY);
    let inner = outer.shrink(l.s(8.0));
    painter.rect_filled(inner, l.s(6.0), SCREEN);
    let bar = Rect::from_min_size(inner.min, vec2(inner.width(), l.s(24.0)));
    let foot = Rect::from_min_max(pos2(inner.min.x, inner.max.y - l.s(22.0)), inner.max);
    let body = Rect::from_min_max(
        pos2(inner.min.x + l.s(10.0), bar.max.y + l.s(4.0)),
        pos2(inner.max.x - l.s(10.0), foot.min.y - l.s(2.0)),
    );
    top_bar(app, ui, l, bar);
    if app.capture.take != Take::Idle {
        rec_view(app, ui, l, body);
    } else if app.help {
        help_view(ui, l, body);
    } else if app.browser {
        preset_view(app, ui, l, body);
    } else {
        match app.view {
            View::Pad => pad_view(app, ui, l, body),
            View::Seq => seq_view(app, ui, l, body),
            View::Song => song_view(app, ui, l, body),
        }
    }
    let painter = ui.painter();
    let (text, color) = match &app.toast {
        Some((t, _)) => (t.clone(), ORANGE),
        None if app.learn.is_some() => ("move a midi control to map it".to_string(), ORANGE),
        None => (hint(app), SCREEN_DIM),
    };
    painter.text(
        pos2(foot.min.x + l.s(10.0), foot.center().y),
        Align2::LEFT_CENTER,
        text,
        l.f(10.0),
        color,
    );
    let hovering = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    if hovering {
        painter.rect_filled(inner, l.s(6.0), Color32::from_black_alpha(220));
        painter.text(
            inner.center(),
            Align2::CENTER_CENTER,
            format!("DROP TO LOAD INTO {}", app.pad_name(app.group, app.pad)),
            l.f(16.0),
            ORANGE,
        );
    }
}

fn hint(app: &App) -> String {
    let mode = if app.keys {
        format!("KEYS  OCT {:+}", app.octave)
    } else {
        "PADS".into()
    };
    let rec = if app.rec_seq { "  REC" } else { "" };
    format!("{mode}{rec}  /  help")
}

fn top_bar(app: &mut App, ui: &Ui, l: &L, bar: Rect) {
    let painter = ui.painter();
    let y = bar.center().y;
    let playing = app.report.playing;
    painter.text(
        pos2(bar.min.x + l.s(10.0), y),
        Align2::LEFT_CENTER,
        if playing { "PLAY" } else { "STOP" },
        l.f(11.0),
        if playing { ORANGE } else { SCREEN_DIM },
    );
    let g = app.group;
    let info = format!(
        "{:5.1} BPM  SW {:.0}%  {}{:02}",
        app.project.tempo,
        app.project.swing * 100.0,
        (b'A' + g as u8) as char,
        app.editing() + 1,
    );
    painter.text(
        pos2(bar.min.x + l.s(56.0), y),
        Align2::LEFT_CENTER,
        info,
        l.f(11.0),
        SCREEN_INK,
    );
    if playing {
        let beat = (app.report.phase as usize / 4) % 4;
        for b in 0..4 {
            let c = pos2(bar.min.x + l.s(290.0 + b as f32 * 12.0), y);
            let color = if b == beat { ORANGE } else { SCREEN_DIM };
            painter.circle_filled(c, l.s(3.0), color);
        }
    }
    for ch in 0..2 {
        let x = bar.min.x + l.s(350.0);
        let rect = Rect::from_min_size(
            pos2(x, y - l.s(5.0) + ch as f32 * l.s(6.0)),
            vec2(l.s(80.0), l.s(4.0)),
        );
        painter.rect_filled(rect, 0.0, SCREEN_FAINT);
        let level = app.meter[ch].clamp(0.0, 1.0);
        let mut fill = rect;
        fill.set_width(rect.width() * level);
        let color = if level > 0.95 { ORANGE } else { SCREEN_INK };
        painter.rect_filled(fill, 0.0, color);
    }
    let tabs = [("PAD", View::Pad), ("SEQ", View::Seq), ("SONG", View::Song)];
    for (i, (name, view)) in tabs.iter().enumerate() {
        let rect = Rect::from_center_size(
            pos2(bar.max.x - l.s(130.0 - i as f32 * 44.0), y),
            vec2(l.s(40.0), l.s(18.0)),
        );
        let resp = ui.interact(rect, Id::unique(("tab", i)), Sense::click());
        let on = app.view == *view && !app.help;
        if on {
            ui.painter().rect_filled(rect, l.s(3.0), SCREEN_INK);
        }
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            *name,
            l.f(10.0),
            if on { SCREEN } else { SCREEN_DIM },
        );
        if resp.clicked() {
            app.view = *view;
            app.help = false;
        }
    }
}

fn pad_view(app: &mut App, ui: &Ui, l: &L, body: Rect) {
    let pad = app.current_pad().clone();
    let name = app.pad_name(app.group, app.pad);
    let painter = ui.painter();
    let title = if pad.silent() {
        "EMPTY".to_string()
    } else {
        pad.label().to_uppercase()
    };
    painter.text(
        body.left_top(),
        Align2::LEFT_TOP,
        format!("{name}  {title}"),
        l.f(13.0),
        SCREEN_INK,
    );
    let detail = match (&pad.sample, pad.source) {
        (_, src) if src.is_synth() => Param::Source.text(Param::Source.pad_get(&pad)),
        (Some(s), _) => format!(
            "{:.2}s  {}k{}",
            s.seconds() * (pad.end - pad.start).abs(),
            s.rate / 1000,
            if pad.reverse { "  REV" } else { "" }
        ),
        (None, _) => String::new(),
    };
    painter.text(
        body.right_top(),
        Align2::RIGHT_TOP,
        detail,
        l.f(11.0),
        SCREEN_DIM,
    );
    let area = Rect::from_min_max(
        body.min + vec2(0.0, l.s(24.0)),
        body.max - vec2(0.0, l.s(18.0)),
    );
    let stats = format!(
        "{}  {:+.0}st  {}  CUT {:.0}%",
        Param::Mode.text(Param::Mode.pad_get(&pad)),
        pad.pitch,
        Param::Filter.text(Param::Filter.pad_get(&pad)),
        pad.cutoff * 100.0
    );
    painter.text(
        body.left_bottom(),
        Align2::LEFT_BOTTOM,
        stats,
        l.f(10.0),
        SCREEN_DIM,
    );
    match (&pad.sample, pad.source) {
        (_, src) if src.is_synth() => synth_view(&pad, ui, l, area),
        (Some(s), _) => {
            wave_view(app, ui, l, area, &s.overview, pad.start, pad.end);
        }
        (None, _) => {
            painter.text(
                area.center() - vec2(0.0, l.s(10.0)),
                Align2::CENTER_CENTER,
                "EMPTY PAD",
                l.f(16.0),
                SCREEN_DIM,
            );
            painter.text(
                area.center() + vec2(0.0, l.s(14.0)),
                Align2::CENTER_CENTER,
                "drop a file, LOAD, SAMPLE to record, P for presets, or REC page ENGINE",
                l.f(10.0),
                SCREEN_DIM,
            );
        }
    }
}

fn wave_view(app: &mut App, ui: &Ui, l: &L, area: Rect, overview: &[f32], start: f32, end: f32) {
    let painter = ui.painter();
    let mid = area.center().y;
    let half = area.height() / 2.0;
    let (a, b) = (start.min(end), start.max(end));
    let columns = (area.width() / l.s(2.0)) as usize;
    for c in 0..columns {
        let t = c as f32 / columns as f32;
        let i = ((t * overview.len() as f32) as usize).min(overview.len().saturating_sub(1));
        let v = overview.get(i).copied().unwrap_or(0.0).min(1.0);
        let x = area.min.x + t * area.width();
        let color = if t >= a && t <= b {
            SCREEN_INK
        } else {
            SCREEN_DIM
        };
        let h = (v * half).max(l.s(0.5));
        painter.line_segment(
            [pos2(x, mid - h), pos2(x, mid + h)],
            Stroke::new(l.s(1.2), color),
        );
    }
    for (t, label) in [(start, "S"), (end, "E")] {
        let x = area.min.x + t * area.width();
        painter.line_segment(
            [pos2(x, area.min.y), pos2(x, area.max.y)],
            Stroke::new(l.s(1.5), ORANGE),
        );
        let flag = Rect::from_min_size(pos2(x, area.min.y), vec2(l.s(12.0), l.s(12.0)));
        painter.rect_filled(flag, 0.0, ORANGE);
        painter.text(
            flag.center(),
            Align2::CENTER_CENTER,
            label,
            l.f(9.0),
            SCREEN,
        );
    }
    let resp = ui.interact(area, Id::unique("wave"), Sense::click_and_drag());
    let Some(pos) = resp.interact_pointer_pos() else {
        return;
    };
    let t = ((pos.x - area.min.x) / area.width()).clamp(0.0, 1.0);
    if resp.drag_started() || resp.clicked() {
        let marker = if (t - start).abs() <= (t - end).abs() {
            MARKER_START
        } else {
            MARKER_END
        };
        app.drag = Some((marker, t));
    }
    if (resp.dragged() || resp.clicked())
        && let Some((marker, _)) = app.drag
    {
        let param = if marker == MARKER_START {
            Param::Start
        } else {
            Param::End
        };
        app.set_param(param, t);
    }
    if resp.drag_stopped() || resp.clicked() {
        app.drag = None;
    }
}

fn synth_view(pad: &flip_engine::Pad, ui: &Ui, l: &L, area: Rect) {
    let painter = ui.painter();
    let left = Rect::from_min_max(area.min, pos2(area.center().x - l.s(10.0), area.max.y));
    let right = Rect::from_min_max(pos2(area.center().x + l.s(10.0), area.min.y), area.max);
    let wave = flip_engine::synth::preview(pad, 201);
    let n = wave.len() - 1;
    let points: Vec<Pos2> = wave
        .iter()
        .enumerate()
        .map(|(i, v)| {
            pos2(
                left.min.x + i as f32 / n as f32 * left.width(),
                left.center().y - v * left.height() * 0.4,
            )
        })
        .collect();
    painter.line_segment(
        [
            pos2(left.min.x, left.center().y),
            pos2(left.max.x, left.center().y),
        ],
        Stroke::new(l.s(1.0), SCREEN_FAINT),
    );
    painter.add(Shape::line(points, Stroke::new(l.s(1.5), SCREEN_INK)));
    let total = pad.attack + pad.decay + pad.release + 0.5;
    let x = |t: f32| right.min.x + t / total * right.width();
    let y = |v: f32| right.max.y - v * right.height() * 0.9;
    let env = vec![
        pos2(x(0.0), y(0.0)),
        pos2(x(pad.attack), y(1.0)),
        pos2(x(pad.attack + pad.decay), y(pad.sustain)),
        pos2(x(pad.attack + pad.decay + 0.5), y(pad.sustain)),
        pos2(x(total), y(0.0)),
    ];
    painter.add(Shape::line(env, Stroke::new(l.s(1.5), ORANGE)));
    painter.text(
        right.left_top(),
        Align2::LEFT_TOP,
        "ENV",
        l.f(9.0),
        SCREEN_DIM,
    );
    painter.text(
        left.left_top(),
        Align2::LEFT_TOP,
        synth_caption(pad),
        l.f(9.0),
        SCREEN_DIM,
    );
}

fn synth_caption(pad: &flip_engine::Pad) -> String {
    let show = |p: Param| p.text(p.pad_get(pad));
    match pad.source {
        flip_engine::Source::Fm => format!(
            "ALGO {}  DEPTH {}",
            show(Param::FmAlgo),
            show(Param::FmDepth)
        ),
        flip_engine::Source::String => {
            format!("DAMP {}  BOW {}", show(Param::Damping), show(Param::Bow))
        }
        flip_engine::Source::Morph => show(Param::MorphPos),
        _ => format!("{} + {}", show(Param::WaveA), show(Param::WaveB)),
    }
}

fn seq_view(app: &mut App, ui: &Ui, l: &L, body: Rect) {
    let g = app.group;
    let index = app.editing();
    let length = app.steps_visible();
    let painter = ui.painter();
    painter.text(
        body.left_top(),
        Align2::LEFT_TOP,
        format!(
            "PATTERN {}{:02}  {} STEPS  PAD {}",
            (b'A' + g as u8) as char,
            index + 1,
            length,
            app.pad_name(g, app.pad)
        ),
        l.f(11.0),
        SCREEN_INK,
    );
    let grid = Rect::from_min_max(body.min + vec2(l.s(22.0), l.s(20.0)), body.max);
    let cw = grid.width() / length as f32;
    let ch = grid.height() / PADS as f32;
    let playing = app.report.playing && app.report.current[g] == index;
    let now = (app.report.pos[g] + length - 1) % length;
    let pattern = app.project.groups[g].patterns[index].clone();
    for row in 0..PADS {
        let y = grid.min.y + row as f32 * ch;
        if row == app.pad {
            painter.rect_filled(
                Rect::from_min_size(pos2(body.min.x, y), vec2(body.width(), ch)),
                0.0,
                SCREEN_FAINT,
            );
        }
        painter.text(
            pos2(body.min.x, y + ch / 2.0),
            Align2::LEFT_CENTER,
            format!("{:02}", row + 1),
            l.f(8.0),
            if row == app.pad { ORANGE } else { SCREEN_DIM },
        );
        for col in 0..length {
            let cell = Rect::from_min_size(pos2(grid.min.x + col as f32 * cw, y), vec2(cw, ch))
                .shrink(l.s(1.0));
            let s = pattern.step(row, col);
            let beat = col % 4 == 0;
            if s.on {
                let color = mix(SCREEN_DIM, SCREEN_INK, s.velocity);
                painter.rect_filled(cell, 1.0, color);
                if s.chance < 1.0 {
                    painter.rect_filled(
                        Rect::from_min_size(cell.min, vec2(cell.width(), l.s(2.0))),
                        0.0,
                        ORANGE,
                    );
                }
            } else {
                let r = if beat { l.s(1.4) } else { l.s(0.8) };
                painter.circle_filled(cell.center(), r, SCREEN_DIM);
            }
            if row == app.pad && app.step == Some(col) {
                painter.rect_stroke(
                    cell,
                    1.0,
                    Stroke::new(l.s(1.5), ORANGE),
                    StrokeKind::Outside,
                );
            }
        }
    }
    if playing {
        let x = grid.min.x + now as f32 * cw;
        painter.rect_stroke(
            Rect::from_min_size(pos2(x, grid.min.y), vec2(cw, grid.height())),
            0.0,
            Stroke::new(l.s(1.0), ORANGE),
            StrokeKind::Inside,
        );
    }
    let resp = ui.interact(grid, Id::unique("grid"), Sense::click());
    if let Some(pos) = resp.interact_pointer_pos()
        && (resp.clicked() || resp.secondary_clicked())
    {
        let col = (((pos.x - grid.min.x) / cw) as usize).min(length - 1);
        let row = (((pos.y - grid.min.y) / ch) as usize).min(PADS - 1);
        app.select_pad(row);
        if resp.clicked() {
            app.toggle_step(row, col);
        } else if pattern.step(row, col).on {
            app.step = Some(col);
            app.page = 5;
        }
    }
}

fn song_view(app: &mut App, ui: &Ui, l: &L, body: Rect) {
    let painter = ui.painter();
    let song = app.project.song.clone();
    painter.text(
        body.left_top(),
        Align2::LEFT_TOP,
        format!(
            "SONG  {} PARTS  {}",
            song.len(),
            if app.project.song_mode {
                "SONG MODE"
            } else {
                "PATTERN MODE"
            }
        ),
        l.f(11.0),
        SCREEN_INK,
    );
    let area = Rect::from_min_max(
        body.min + vec2(0.0, l.s(22.0)),
        body.max - vec2(0.0, l.s(30.0)),
    );
    let (bw, bh) = (l.s(62.0), l.s(70.0));
    let per_row = ((area.width() + l.s(6.0)) / (bw + l.s(6.0)))
        .floor()
        .max(1.0) as usize;
    let active = app.report.playing && app.project.song_mode && !song.is_empty();
    for (i, entry) in song.iter().enumerate() {
        let (r, c) = (i / per_row, i % per_row);
        let rect = Rect::from_min_size(
            area.min + vec2(c as f32 * (bw + l.s(6.0)), r as f32 * (bh + l.s(6.0))),
            vec2(bw, bh),
        );
        if rect.max.y > area.max.y {
            break;
        }
        let on = active && app.report.entry == i;
        painter.rect_stroke(
            rect,
            l.s(3.0),
            Stroke::new(l.s(1.0), if on { ORANGE } else { SCREEN_DIM }),
            StrokeKind::Inside,
        );
        painter.text(
            rect.left_top() + vec2(l.s(5.0), l.s(4.0)),
            Align2::LEFT_TOP,
            format!("{:02}", i + 1),
            l.f(10.0),
            if on { ORANGE } else { SCREEN_INK },
        );
        for (g, p) in entry.iter().enumerate() {
            painter.text(
                rect.left_top() + vec2(l.s(5.0), l.s(20.0 + g as f32 * 12.0)),
                Align2::LEFT_TOP,
                format!("{}{:02}", (b'A' + g as u8) as char, p + 1),
                l.f(9.0),
                SCREEN_DIM,
            );
        }
    }
    if song.is_empty() {
        painter.text(
            area.center(),
            Align2::CENTER_CENTER,
            "pick a pattern per group, then ADD",
            l.f(11.0),
            SCREEN_DIM,
        );
    }
    let labels = [
        "+ ADD",
        "- DEL",
        if app.project.song_mode {
            "SONG ON"
        } else {
            "SONG OFF"
        },
    ];
    for (i, label) in labels.iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(body.min.x + i as f32 * l.s(96.0), body.max.y - l.s(22.0)),
            vec2(l.s(88.0), l.s(20.0)),
        );
        let resp = ui.interact(rect, Id::unique(("song", i)), Sense::click());
        let lit = i == 2 && app.project.song_mode;
        ui.painter().rect_stroke(
            rect,
            l.s(3.0),
            Stroke::new(l.s(1.0), if lit { ORANGE } else { SCREEN_INK }),
            StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            *label,
            l.f(10.0),
            if lit { ORANGE } else { SCREEN_INK },
        );
        if resp.clicked() {
            match i {
                0 => app.song_add(),
                1 => app.song_remove(),
                _ => app.toggle_song_mode(),
            }
        }
    }
}

fn rec_view(app: &mut App, ui: &Ui, l: &L, body: Rect) {
    let c = &app.capture;
    let painter = ui.painter();
    let blink = (app.now * 3.0).fract() < 0.5;
    let (title, color) = match c.take {
        Take::Armed => ("WAITING FOR SOUND", if blink { ORANGE } else { SCREEN_DIM }),
        _ => ("RECORDING", ORANGE),
    };
    painter.text(body.left_top(), Align2::LEFT_TOP, title, l.f(16.0), color);
    let source = crate::params::CAPTURES[c.source.min(5)];
    painter.text(
        body.right_top(),
        Align2::RIGHT_TOP,
        format!("{source} -> {}", app.pad_name(c.group, c.pad)),
        l.f(11.0),
        SCREEN_INK,
    );
    let seconds = c.frames.len() as f32 / c.rate as f32;
    painter.text(
        body.center() - vec2(0.0, l.s(20.0)),
        Align2::CENTER_CENTER,
        format!("{seconds:5.1}s"),
        l.f(34.0),
        SCREEN_INK,
    );
    let bar = Rect::from_min_size(
        pos2(body.min.x, body.max.y - l.s(50.0)),
        vec2(body.width(), l.s(14.0)),
    );
    painter.rect_filled(bar, 0.0, SCREEN_FAINT);
    let mut fill = bar;
    fill.set_width(bar.width() * c.level.clamp(0.0, 1.0));
    painter.rect_filled(fill, 0.0, SCREEN_INK);
    if c.source == 0 {
        let x = bar.min.x + bar.width() * app.settings.threshold;
        painter.line_segment(
            [pos2(x, bar.min.y - l.s(4.0)), pos2(x, bar.max.y + l.s(4.0))],
            Stroke::new(l.s(2.0), ORANGE),
        );
    }
    painter.text(
        body.left_bottom(),
        Align2::LEFT_BOTTOM,
        "SAMPLE or B to stop  ESC to cancel  REC page sets THRESH and SOURCE",
        l.f(10.0),
        SCREEN_DIM,
    );
}

fn help_view(ui: &Ui, l: &L, body: Rect) {
    let painter = ui.painter();
    let rows = HELP.len().div_ceil(2);
    let col_w = body.width() / 2.0;
    let row_h = (body.height() / rows as f32).min(l.s(22.0));
    for (i, (keys, what)) in HELP.iter().enumerate() {
        let (c, r) = (i / rows, i % rows);
        let p = body.min + vec2(c as f32 * col_w, r as f32 * row_h);
        painter.text(p, Align2::LEFT_TOP, *keys, l.f(10.0), ORANGE);
        painter.text(
            p + vec2(l.s(112.0), 0.0),
            Align2::LEFT_TOP,
            *what,
            l.f(10.0),
            SCREEN_INK,
        );
    }
}

fn preset_view(app: &mut App, ui: &Ui, l: &L, body: Rect) {
    use flip_engine::preset::CATEGORIES;
    let tab_w = body.width() / CATEGORIES.len() as f32;
    let tab_h = l.s(18.0);
    let mut pick_category = None;
    for (i, name) in CATEGORIES.iter().enumerate() {
        let rect = Rect::from_min_size(body.min + vec2(i as f32 * tab_w, 0.0), vec2(tab_w, tab_h));
        let resp = ui.interact(rect, Id::unique(("preset-tab", i)), Sense::click());
        let on = app.library.category == i;
        let painter = ui.painter();
        if on {
            painter.rect_filled(rect.shrink(l.s(1.0)), l.s(3.0), SCREEN_FAINT);
        }
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            *name,
            l.f(9.0),
            if on { ORANGE } else { SCREEN_DIM },
        );
        if resp.clicked() {
            pick_category = Some(i);
        }
    }
    if let Some(i) = pick_category {
        app.library.category = i;
        app.library.cursor = 0;
    }
    let list = Rect::from_min_max(body.min + vec2(0.0, tab_h + l.s(6.0)), body.max);
    let shown = app.library.shown();
    if shown.is_empty() {
        ui.painter().text(
            list.center(),
            Align2::CENTER_CENTER,
            "no presets here yet  SHIFT P saves the selected pad",
            l.f(10.0),
            SCREEN_DIM,
        );
        return;
    }
    let row_h = l.s(18.0);
    let rows = ((list.height() / row_h).floor() as usize).max(1);
    let columns = 3;
    let col_w = list.width() / columns as f32;
    let page = app.library.cursor / (rows * columns);
    let first = page * rows * columns;
    let mut load = None;
    for (slot, &index) in shown.iter().enumerate().skip(first).take(rows * columns) {
        let n = slot - first;
        let rect = Rect::from_min_size(
            list.min + vec2((n / rows) as f32 * col_w, (n % rows) as f32 * row_h),
            vec2(col_w - l.s(6.0), row_h - l.s(2.0)),
        );
        let resp = ui.interact(rect, Id::unique(("preset", index)), Sense::click());
        let on = slot == app.library.cursor;
        let painter = ui.painter();
        if on {
            painter.rect_filled(rect, l.s(3.0), SCREEN_FAINT);
        }
        let preset = &app.library.items[index];
        painter.text(
            rect.left_center() + vec2(l.s(6.0), 0.0),
            Align2::LEFT_CENTER,
            preset.name.to_uppercase(),
            l.f(10.0),
            if on { ORANGE } else { SCREEN_INK },
        );
        painter.text(
            rect.right_center() - vec2(l.s(6.0), 0.0),
            Align2::RIGHT_CENTER,
            Param::Source.text(Param::Source.pad_get(&preset.sound)),
            l.f(8.0),
            SCREEN_DIM,
        );
        if resp.clicked() {
            app.library.cursor = slot;
            load = Some(index);
        }
    }
    if let Some(i) = load {
        app.load_preset(i);
    }
}

pub fn naming(app: &mut App, ctx: &egui::Context) {
    let Some(mut name) = app.naming.take() else {
        return;
    };
    let mut open = true;
    let mut done = false;
    egui::Window::new("SAVE PRESET")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let edit = ui.text_edit_singleline(&mut name);
            if !edit.has_focus() && !edit.lost_focus() {
                edit.request_focus();
            }
            let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("SAVE").clicked() || enter {
                done = true;
            }
        });
    if done {
        app.save_preset(&name);
    } else if open && !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.naming = Some(name);
    }
}

fn knobs(app: &mut App, ui: &Ui, l: &L) {
    let params = app.page_params();
    for (i, param) in params.iter().enumerate() {
        let center = l.p(115.0 + i as f32 * 150.0, 438.0);
        knob(app, ui, l, i, *param, center);
    }
}

fn knob(app: &mut App, ui: &Ui, l: &L, i: usize, param: Param, center: Pos2) {
    let radius = l.s(34.0);
    let rect = Rect::from_center_size(center, vec2(radius * 2.6, radius * 2.6));
    let resp = ui.interact(rect, Id::unique(("knob", i)), Sense::click_and_drag());
    let value = app.param(param);
    let unit = value.map(|v| param.unit_of(v));
    let painter = ui.painter();
    let start = 0.75 * PI;
    let sweep = 1.5 * PI;
    let ring = radius + l.s(9.0);
    for t in 0..=20 {
        let a = start + sweep * t as f32 / 20.0;
        let p = center + vec2(a.cos(), a.sin()) * ring;
        let on = unit.is_some_and(|u| t as f32 / 20.0 <= u + 1e-4);
        painter.circle_filled(p, l.s(1.6), if on { INK } else { BODY_EDGE });
    }
    let color = KNOBS[i];
    let face = if value.is_none() {
        mix(color, BODY, 0.6)
    } else {
        color
    };
    painter.circle_filled(center + vec2(0.0, l.s(3.0)), radius, mix(color, INK, 0.5));
    painter.circle_filled(center, radius, face);
    painter.circle_stroke(center, radius, Stroke::new(l.s(1.0), mix(color, INK, 0.3)));
    if let Some(u) = unit {
        let a = start + sweep * u;
        let dir = vec2(a.cos(), a.sin());
        let ink = if i == 2 { INK } else { WHITE };
        painter.line_segment(
            [center + dir * radius * 0.35, center + dir * radius * 0.85],
            Stroke::new(l.s(3.0), ink),
        );
    }
    if app.learn == Some(param) && (app.now * 4.0).fract() < 0.5 {
        painter.circle_stroke(center, radius + l.s(4.0), Stroke::new(l.s(2.0), ORANGE));
    }
    let spec = param.spec();
    painter.text(
        center + vec2(0.0, radius + l.s(22.0)),
        Align2::CENTER_CENTER,
        spec.name,
        l.f(10.0),
        DIM_INK,
    );
    let text = value.map(|v| param.text(v)).unwrap_or_else(|| "--".into());
    painter.text(
        center + vec2(0.0, radius + l.s(36.0)),
        Align2::CENTER_CENTER,
        text,
        l.f(12.0),
        INK,
    );
    let Some(u) = unit else {
        return;
    };
    if resp.drag_started() {
        app.drag = Some((i, u));
    }
    if resp.dragged()
        && let Some((j, mut held)) = app.drag
        && j == i
    {
        let fine = if ui.input(|inp| inp.modifiers.shift) {
            FINE
        } else {
            1.0
        };
        held = (held - resp.drag_delta().y / (l.s(DRAG_RANGE) * fine)).clamp(0.0, 1.0);
        app.drag = Some((i, held));
        app.set_param(param, param.value_at(held));
    }
    if resp.drag_stopped() {
        app.drag = None;
    }
    if resp.double_clicked() {
        app.set_param(param, spec.default);
    }
    if resp.secondary_clicked() {
        app.learn = Some(param);
    }
    if resp.hovered() && spec.steps.is_none() {
        let scroll = ui.input(|inp| inp.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let next = (u + scroll / l.s(SCROLL_RANGE)).clamp(0.0, 1.0);
            app.set_param(param, param.value_at(next));
        }
    }
}

fn pages(app: &mut App, ui: &Ui, l: &L) {
    let w = (600.0 - 7.0 * 6.0) / 8.0;
    for i in 0..PAGES.len() {
        let rect = l.r(40.0 + i as f32 * (w + 6.0), 526.0, w, 28.0);
        let name = app.page_name(i);
        if key(ui, l, ("page", i), rect, &name, app.page == i, false).clicked() {
            app.open_page(i);
        }
    }
}

fn functions(app: &mut App, ui: &Ui, l: &L) {
    let w = (600.0 - 7.0 * 6.0) / 8.0;
    let names = [
        "PLAY", "REC", "METRO", "SAMPLE", "CHOP", "BOUNCE", "LOAD", "CLEAR",
    ];
    let blink = (app.now * 3.0).fract() < 0.5;
    for (i, name) in names.iter().enumerate() {
        let rect = l.r(40.0 + i as f32 * (w + 6.0), 572.0, w, 52.0);
        let lit = match i {
            0 => app.report.playing,
            1 => app.rec_seq,
            2 => app.project.metronome,
            3 => match app.capture.take {
                Take::Idle => false,
                Take::Armed => blink,
                Take::Recording => true,
            },
            _ => false,
        };
        if key(ui, l, ("fn", i), rect, name, lit, true).clicked() {
            match i {
                0 => app.toggle_play(),
                1 => app.rec_seq = !app.rec_seq,
                2 => app.toggle_metronome(),
                3 => app.arm(),
                4 => app.chop(),
                5 => app.bounce(),
                6 => app.load_dialog(),
                _ => app.clear_steps(),
            }
        }
    }
}

fn groups(app: &mut App, ui: &Ui, l: &L) {
    for g in 0..GROUPS {
        let rect = l.r(674.0 + g as f32 * 58.0, 96.0, 52.0, 40.0);
        let group = &app.project.groups[g];
        let label = format!(
            "{}{}",
            (b'A' + g as u8) as char,
            if group.mute { "-" } else { "" }
        );
        let active = app.now - app.flashes[g].iter().copied().fold(f64::MIN, f64::max) < FLASH;
        let resp = key(ui, l, ("group", g), rect, &label, app.group == g, false);
        if active && !group.mute {
            ui.painter().circle_filled(
                rect.right_top() + vec2(-l.s(8.0), l.s(8.0)),
                l.s(2.5),
                GREEN,
            );
        }
        if resp.clicked() {
            app.select_group(g);
        }
        if resp.secondary_clicked() {
            app.toggle_mute(g);
        }
    }
    let rect = l.r(906.0, 96.0, 72.0, 40.0);
    if key(ui, l, "step", rect, "STEP", app.step_mode, true).clicked() {
        app.step_mode = !app.step_mode;
    }
    let rect = l.r(984.0, 96.0, 72.0, 40.0);
    if key(ui, l, "keys", rect, "KEYS", app.keys, true).clicked() {
        let on = !app.keys;
        app.set_keys(on);
    }
}

fn pads(app: &mut App, ui: &Ui, l: &L) {
    let g = app.group;
    let index = app.editing();
    let pattern = app.project.groups[g].patterns[index].clone();
    let length = pattern.length;
    let shift = ui.input(|i| i.modifiers.shift);
    let playing_here = app.report.playing && app.report.current[g] == index;
    let now_step = (app.report.pos[g] + length - 1) % length;
    for (i, code) in PAD_KEYS.iter().enumerate() {
        let key_label = app.key_label(*code);
        let (row, col) = (i / 4, i % 4);
        let rect = l.r(
            674.0 + col as f32 * 98.0,
            150.0 + row as f32 * 96.0,
            88.0,
            86.0,
        );
        let resp = ui.interact(rect, Id::unique(("pad", i)), Sense::click_and_drag());
        let down = resp.is_pointer_button_down_on();
        let step = app.step_page * 16 + i;
        if app.step_mode {
            if resp.clicked() && step < length {
                app.toggle_step(app.pad, step);
            }
            if resp.secondary_clicked() && step < length && pattern.step(app.pad, step).on {
                app.step = Some(step);
                app.page = 5;
            }
        } else if resp.secondary_clicked() || (shift && resp.clicked()) {
            app.select_pad(i);
        } else if down && !shift && app.mouse.is_none() {
            let velocity = resp
                .interact_pointer_pos()
                .map(|p| 1.0 - 0.6 * ((p.y - rect.min.y) / rect.height()).clamp(0.0, 1.0))
                .unwrap_or(0.85);
            app.press(g, i, 0, velocity, false);
            app.mouse = Some((g, i, 0));
        }
        let painter = ui.painter();
        let radius = l.s(8.0);
        painter.rect_filled(rect.translate(vec2(0.0, l.s(4.0))), radius, PAD_SHADOW);
        let face_rect = rect.translate(vec2(0.0, if down { l.s(3.0) } else { 0.0 }));
        let pad = &app.project.groups[g].pads[i];
        let mut face = PAD;
        if app.step_mode {
            if step >= length {
                face = mix(PAD, BODY, 0.7);
            } else {
                let s = pattern.step(app.pad, step);
                if s.on {
                    face = mix(PAD, ORANGE, 0.35 + 0.65 * s.velocity);
                }
                if playing_here && now_step == step {
                    face = mix(face, INK, 0.25);
                }
            }
        } else {
            let t = ((app.now - app.flashes[g][i]) / FLASH) as f32;
            if t < 1.0 {
                face = mix(PAD, ORANGE, 1.0 - t);
            }
        }
        painter.rect_filled(face_rect, radius, face);
        if i == app.pad && !app.step_mode {
            painter.rect_stroke(
                face_rect,
                radius,
                Stroke::new(l.s(2.0), ORANGE),
                StrokeKind::Inside,
            );
        }
        let corner = face_rect.left_top() + vec2(l.s(8.0), l.s(7.0));
        if app.step_mode {
            painter.text(
                corner,
                Align2::LEFT_TOP,
                format!("{:02}", step + 1),
                l.f(11.0),
                if step < length { INK } else { DIM_INK },
            );
            continue;
        }
        painter.text(
            corner,
            Align2::LEFT_TOP,
            format!("{:02}", i + 1),
            l.f(11.0),
            INK,
        );
        if !app.keys {
            painter.text(
                face_rect.right_top() + vec2(-l.s(8.0), l.s(7.0)),
                Align2::RIGHT_TOP,
                key_label,
                l.f(10.0),
                DIM_INK,
            );
        }
        let mut label = pad.label();
        if label.chars().count() > 10 {
            label = label.chars().take(9).collect::<String>() + ".";
        }
        painter.text(
            face_rect.left_bottom() + vec2(l.s(8.0), -l.s(8.0)),
            Align2::LEFT_BOTTOM,
            label,
            l.f(9.0),
            DIM_INK,
        );
        let used = (0..length).any(|s| pattern.step(i, s).on);
        if used {
            painter.circle_filled(
                face_rect.right_bottom() + vec2(-l.s(10.0), -l.s(10.0)),
                l.s(3.0),
                INK,
            );
        }
    }
    if let Some((g, p, n)) = app.mouse
        && !ui.input(|i| i.pointer.primary_down())
    {
        app.lift(g, p, n, false);
        app.mouse = None;
    }
}

fn patterns(app: &mut App, ui: &Ui, l: &L) {
    let g = app.group;
    let w = (382.0 - 7.0 * 6.0) / 8.0;
    for i in 0..PATTERNS {
        let (row, col) = (i / 8, i % 8);
        let rect = l.r(
            674.0 + col as f32 * (w + 6.0),
            548.0 + row as f32 * 44.0,
            w,
            36.0,
        );
        let used = !app.project.groups[g].patterns[i].is_empty();
        let label = format!("{:02}", i + 1);
        let resp = key(
            ui,
            l,
            ("pattern", i),
            rect,
            &label,
            used,
            app.editing() != i,
        );
        let playing = app.report.playing && app.report.current[g] == i;
        if playing && (app.report.phase.fract() < 0.5 || app.editing() != i) {
            ui.painter().rect_stroke(
                rect.expand(l.s(2.0)),
                l.s(6.0),
                Stroke::new(l.s(1.5), ORANGE),
                StrokeKind::Outside,
            );
        }
        if resp.clicked() {
            app.select_pattern(i);
        }
        if resp.secondary_clicked() && i != app.editing() {
            app.remember();
            let from = app.editing();
            app.project.groups[g].patterns[i] = app.project.groups[g].patterns[from].clone();
            app.sync_pattern(g, i);
            app.say(format!("copied {:02} to {:02}", from + 1, i + 1));
        }
    }
    let length = app.project.pattern(g).length;
    let w = (382.0 - 3.0 * 6.0) / 4.0;
    for p in 0..4 {
        let rect = l.r(674.0 + p as f32 * (w + 6.0), 640.0, w, 26.0);
        let label = format!("{}-{}", p * 16 + 1, p * 16 + 16);
        let enabled = p * 16 < length;
        let resp = key(
            ui,
            l,
            ("steppage", p),
            rect,
            &label,
            app.step_page == p,
            true,
        );
        if !enabled {
            ui.painter()
                .rect_filled(rect, l.s(5.0), Color32::from_black_alpha(120));
        }
        if resp.clicked() && enabled {
            app.step_page = p;
        }
    }
}

fn footer(app: &App, ui: &Ui, l: &L) {
    let out = app
        .output
        .as_ref()
        .map(|o| format!("{} {}k", o.device, o.rate / 1000))
        .unwrap_or_else(|| "none".into());
    let midi = app.midi.port.clone().unwrap_or_else(|| "none".into());
    let text = format!("OUT {out}   MIDI {midi}");
    ui.painter().text(
        l.p(40.0, 652.0),
        Align2::LEFT_CENTER,
        text,
        l.f(9.0),
        DIM_INK,
    );
}

pub fn setup(app: &mut App, ctx: &egui::Context) {
    if !app.setup {
        return;
    }
    let lists = app.lists.get_or_insert_with(|| {
        (
            crate::audio::names(false),
            crate::audio::names(true),
            crate::midi::ports(),
        )
    });
    let (outs, ins, ports) = lists.clone();
    let mut open = true;
    let mut pick_out = None;
    let mut pick_in = None;
    let mut pick_midi = None;
    let mut refresh = false;
    let mut forget = false;
    egui::Window::new("SETUP")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(420.0)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            for font in ui.style_mut().text_styles.values_mut() {
                font.size *= 1.4;
            }
            ui.spacing_mut().item_spacing = vec2(10.0, 8.0);
            let current_out = app
                .output
                .as_ref()
                .map(|o| o.device.clone())
                .unwrap_or_default();
            let current_in = app
                .settings
                .input
                .clone()
                .unwrap_or_else(|| "default".into());
            let current_midi = app.midi.port.clone().unwrap_or_else(|| "none".into());
            egui::Grid::new("setup-grid").num_columns(2).show(ui, |ui| {
                ui.label("OUTPUT");
                egui::ComboBox::from_id_salt("out")
                    .selected_text(current_out.clone())
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for n in &outs {
                            if ui.selectable_label(*n == current_out, n.as_str()).clicked() {
                                pick_out = Some(n.clone());
                            }
                        }
                    });
                ui.end_row();
                ui.label("INPUT");
                egui::ComboBox::from_id_salt("in")
                    .selected_text(current_in.clone())
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for n in &ins {
                            if ui.selectable_label(*n == current_in, n.as_str()).clicked() {
                                pick_in = Some(n.clone());
                            }
                        }
                    });
                ui.end_row();
                ui.label("MIDI");
                egui::ComboBox::from_id_salt("midi")
                    .selected_text(current_midi.clone())
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for n in &ports {
                            if ui
                                .selectable_label(*n == current_midi, n.as_str())
                                .clicked()
                            {
                                pick_midi = Some(n.clone());
                            }
                        }
                    });
                ui.end_row();
            });
            ui.add_space(8.0);
            ui.label(format!("MIDI MAP: {} controls", app.settings.learned.len()));
            for (cc, p) in &app.settings.learned {
                ui.label(format!("  CC {cc:>3} -> {}", p.spec().name));
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("REFRESH").clicked() {
                    refresh = true;
                }
                if ui.button("CLEAR MIDI MAP").clicked() {
                    forget = true;
                }
            });
            ui.add_space(4.0);
            ui.label("Right-click a knob, then move a MIDI control to map it.");
            ui.label("Pad notes 36-51 play pads. Other notes play the selected pad.");
        });
    if let Some(n) = pick_out {
        app.open_output(Some(n));
        app.settings.save();
    }
    if let Some(n) = pick_in {
        app.cancel_capture();
        app.open_input(Some(n));
        app.settings.save();
    }
    if let Some(n) = pick_midi {
        app.connect_midi(&n);
        app.settings.save();
    }
    if refresh {
        app.lists = None;
    }
    if forget {
        app.settings.learned.clear();
        app.settings.save();
    }
    if !open {
        app.setup = false;
        app.settings.save();
    }
}
