use eframe::egui::{Align2, CursorIcon, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use flip_engine::{Note, Pattern, TICKS};

use crate::app::{App, RollDrag};
use crate::record;
use crate::theme::*;
use crate::view::{L, mix};

pub const ROWS: i8 = 25;
const LOWEST: i8 = -48;
const HIGHEST: i8 = 48;
const EDGE: f32 = 6.0;
const SCROLL_STEP: f32 = 24.0;
const NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

pub fn clamp_low(low: i8) -> i8 {
    low.clamp(LOWEST, HIGHEST - ROWS + 1)
}

pub fn pitch_name(pitch: i8) -> String {
    let midi = 60 + pitch as i32;
    format!(
        "{}{}",
        NAMES[midi.rem_euclid(12) as usize],
        midi.div_euclid(12) - 1
    )
}

fn is_black(pitch: i8) -> bool {
    matches!((60 + pitch as i32).rem_euclid(12), 1 | 3 | 6 | 8 | 10)
}

pub fn row_pitch(low: i8, row: usize) -> i8 {
    low + (ROWS - 1) - row as i8
}

pub fn snap_free(tick: f64, grid: u32, free: bool) -> f64 {
    if free || grid <= 1 {
        tick.floor()
    } else {
        (tick / grid as f64).round() * grid as f64
    }
}

pub fn hit(notes: &[Note], pad: usize, tick: f64, pitch: i8) -> Option<usize> {
    notes.iter().rposition(|n| {
        n.pad as usize == pad
            && n.pitch == pitch
            && tick >= n.start as f64
            && tick < (n.start + n.length) as f64
    })
}

struct Frame {
    grid: Rect,
    total: u32,
    low: i8,
}

impl Frame {
    fn row_h(&self) -> f32 {
        self.grid.height() / ROWS as f32
    }

    fn x(&self, tick: f64) -> f32 {
        self.grid.min.x + (tick / self.total as f64) as f32 * self.grid.width()
    }

    fn tick(&self, x: f32) -> f64 {
        ((x - self.grid.min.x) / self.grid.width()).clamp(0.0, 0.9999) as f64 * self.total as f64
    }

    fn pitch(&self, y: f32) -> i8 {
        let row = ((y - self.grid.min.y) / self.row_h()).clamp(0.0, ROWS as f32 - 1.0) as usize;
        row_pitch(self.low, row)
    }

    fn rect(&self, n: &Note) -> Option<Rect> {
        let top = self.low + ROWS - 1;
        if n.pitch < self.low || n.pitch > top {
            return None;
        }
        let row = (top - n.pitch) as f32;
        let x0 = self.x(n.start as f64);
        let x1 = self
            .x((n.start + n.length).min(self.total) as f64)
            .max(x0 + 2.0);
        let y = self.grid.min.y + row * self.row_h();
        Some(Rect::from_min_max(
            pos2(x0, y + 1.0),
            pos2(x1, y + self.row_h() - 1.0),
        ))
    }
}

pub fn view(app: &mut App, ui: &Ui, l: &L, body: Rect) {
    let g = app.group;
    let index = app.editing();
    let pad = app.roll_pad();
    let pattern: Pattern = app.project.groups[g].patterns[index].clone();
    let grid_index = app.settings.quantize.min(record::GRIDS.len() - 1);
    let grid_ticks = record::grid(grid_index);
    let painter = ui.painter();
    painter.text(
        body.left_top(),
        Align2::LEFT_TOP,
        format!(
            "ROLL {}{:02}  PAD {}  GRID {}",
            (b'A' + g as u8) as char,
            index + 1,
            app.pad_name(g, pad),
            record::GRIDS[grid_index].0
        ),
        l.f(11.0),
        SCREEN_INK,
    );
    painter.text(
        body.right_top(),
        Align2::RIGHT_TOP,
        "click add  drag move  edge size  right-click delete",
        l.f(8.0),
        SCREEN_DIM,
    );
    let chip = Rect::from_min_size(
        body.left_top() + vec2(l.s(236.0), -l.s(1.0)),
        vec2(l.s(44.0), l.s(14.0)),
    );
    let chip_resp = ui.interact(chip, Id::unique("roll-keys"), Sense::click());
    if app.roll_keys {
        painter.rect_filled(chip, l.s(3.0), ORANGE);
    } else {
        painter.rect_stroke(
            chip,
            l.s(3.0),
            Stroke::new(
                l.s(1.0),
                if chip_resp.hovered() {
                    SCREEN_INK
                } else {
                    SCREEN_DIM
                },
            ),
            StrokeKind::Inside,
        );
    }
    painter.text(
        chip.center(),
        Align2::CENTER_CENTER,
        "KEYS",
        l.f(9.0),
        if app.roll_keys { SCREEN } else { SCREEN_DIM },
    );
    if chip_resp.clicked() {
        app.roll_keys = !app.roll_keys;
    }
    let area = Rect::from_min_max(body.min + vec2(0.0, l.s(18.0)), body.max);
    let keys_w = l.s(30.0);
    let f = Frame {
        grid: Rect::from_min_max(area.min + vec2(keys_w, 0.0), area.max),
        total: pattern.ticks(),
        low: app.roll_low,
    };
    let row_h = f.row_h();
    for row in 0..ROWS as usize {
        let pitch = row_pitch(f.low, row);
        let y = f.grid.min.y + row as f32 * row_h;
        let lane = Rect::from_min_size(pos2(f.grid.min.x, y), vec2(f.grid.width(), row_h));
        if is_black(pitch) {
            painter.rect_filled(lane, 0.0, mix(SCREEN, SCREEN_FAINT, 0.6));
        }
        let key = Rect::from_min_size(pos2(area.min.x, y), vec2(keys_w - l.s(3.0), row_h));
        painter.rect_filled(
            key.shrink(0.5),
            1.0,
            if is_black(pitch) {
                SCREEN_FAINT
            } else {
                SCREEN_DIM
            },
        );
        if (60 + pitch as i32).rem_euclid(12) == 0 {
            painter.text(
                key.left_center() + vec2(l.s(2.0), 0.0),
                Align2::LEFT_CENTER,
                pitch_name(pitch),
                l.f(7.0),
                SCREEN_INK,
            );
        }
    }
    for step in 0..=pattern.length {
        let x = f.x((step as u32 * TICKS) as f64);
        let color = if step % 4 == 0 {
            SCREEN_DIM
        } else {
            SCREEN_FAINT
        };
        painter.line_segment(
            [pos2(x, f.grid.min.y), pos2(x, f.grid.max.y)],
            Stroke::new(1.0, color),
        );
    }
    for n in pattern.notes.iter().filter(|n| n.pad as usize != pad) {
        if let Some(r) = f.rect(n) {
            painter.rect_filled(r, 1.0, SCREEN_FAINT);
        }
    }
    for n in pattern.notes.iter().filter(|n| n.pad as usize == pad) {
        if let Some(r) = f.rect(n) {
            painter.rect_filled(r, 1.0, mix(SCREEN_DIM, SCREEN_INK, n.velocity));
        }
    }
    if let Some(drag) = &app.roll_drag
        && let Some(r) = f.rect(&drag.note)
    {
        painter.rect_filled(r, 1.0, ORANGE);
    }
    if app.report.playing
        && app.report.current[g] == index
        && let Some((_, tick, _)) = app.clock(g)
    {
        let x = f.x(tick);
        painter.line_segment(
            [pos2(x, f.grid.min.y), pos2(x, f.grid.max.y)],
            Stroke::new(l.s(1.0), ORANGE),
        );
    }
    painter.rect_stroke(
        f.grid,
        0.0,
        Stroke::new(1.0, SCREEN_DIM),
        StrokeKind::Outside,
    );

    let resp = ui.interact(f.grid, Id::unique("roll"), Sense::click_and_drag());
    let free = ui.input(|i| i.modifiers.shift);
    if resp.hovered() {
        let delta = ui.input(|i| i.smooth_scroll_delta.y);
        app.roll_scroll += delta;
        while app.roll_scroll.abs() >= SCROLL_STEP {
            let up = app.roll_scroll > 0.0;
            app.roll_scroll -= SCROLL_STEP * app.roll_scroll.signum();
            app.roll_low = clamp_low(app.roll_low + if up { 1 } else { -1 });
        }
        if let Some(p) = resp.hover_pos() {
            let tick = f.tick(p.x);
            let pitch = f.pitch(p.y);
            if let Some(i) = hit(&pattern.notes, pad, tick, pitch)
                && let Some(r) = f.rect(&pattern.notes[i])
                && r.max.x - p.x < l.s(EDGE)
            {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
        }
    }
    let Some(p) = resp.interact_pointer_pos() else {
        return;
    };
    let tick = f.tick(p.x);
    let pitch = f.pitch(p.y);
    let found = hit(&pattern.notes, pad, tick, pitch);
    if resp.drag_started() {
        app.remember();
        let drag = match found {
            Some(i) => {
                let note = app.project.groups[g].patterns[index].notes.remove(i);
                let edge = f.rect(&note).is_some_and(|r| r.max.x - p.x < l.s(EDGE));
                RollDrag {
                    note,
                    resize: edge,
                    grab: tick - note.start as f64,
                }
            }
            None => {
                let start = snap_free(tick, grid_ticks, free).floor().max(0.0) as u32;
                RollDrag {
                    note: Note {
                        pad: pad as u8,
                        start: start.min(f.total - 1),
                        length: grid_ticks.max(TICKS / 4),
                        pitch,
                        velocity: 0.8,
                    },
                    resize: true,
                    grab: 0.0,
                }
            }
        };
        app.roll_drag = Some(drag);
    } else if resp.dragged()
        && let Some(drag) = app.roll_drag.as_mut()
    {
        if drag.resize {
            let end = snap_free(tick, grid_ticks, free).max(drag.note.start as f64 + 1.0);
            drag.note.length = ((end - drag.note.start as f64) as u32).max(1);
        } else {
            let start =
                snap_free(tick - drag.grab, grid_ticks, free).clamp(0.0, (f.total - 1) as f64);
            drag.note.start = start as u32;
            drag.note.pitch = pitch;
        }
    }
    if resp.drag_stopped()
        && let Some(drag) = app.roll_drag.take()
    {
        app.project.groups[g].patterns[index].add_note(drag.note);
        app.sync_pattern(g, index);
    } else if resp.clicked() && found.is_none() {
        app.remember();
        let start = snap_free(tick, grid_ticks, free).max(0.0) as u32;
        app.project.groups[g].patterns[index].add_note(Note {
            pad: pad as u8,
            start: start.min(f.total - 1),
            length: grid_ticks.max(TICKS / 4),
            pitch,
            velocity: 0.8,
        });
        app.sync_pattern(g, index);
    } else if resp.secondary_clicked()
        && let Some(i) = found
    {
        app.remember();
        app.project.groups[g].patterns[index].notes.remove(i);
        app.sync_pattern(g, index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(start: u32, length: u32, pitch: i8, pad: u8) -> Note {
        Note {
            pad,
            start,
            length,
            pitch,
            velocity: 1.0,
        }
    }

    mod when_naming_pitches {
        use super::*;

        #[test]
        fn calls_the_offset_zero_middle_c() {
            assert_eq!(pitch_name(0), "C4");
            assert_eq!(pitch_name(-13), "B2");
            assert_eq!(pitch_name(13), "C#5");
        }
    }

    mod when_scrolling {
        use super::*;

        #[test]
        fn stays_inside_the_playable_range() {
            assert_eq!(clamp_low(-100), LOWEST);
            assert_eq!(clamp_low(100), HIGHEST - ROWS + 1);
            assert_eq!(row_pitch(-12, 0), 12);
            assert_eq!(row_pitch(-12, ROWS as usize - 1), -12);
        }
    }

    mod when_pointing_at_a_note {
        use super::*;

        #[test]
        fn finds_only_notes_of_the_selected_pad() {
            let notes = [note(0, 24, 0, 1), note(0, 24, 0, 2)];
            assert_eq!(hit(&notes, 2, 10.0, 0), Some(1));
            assert_eq!(hit(&notes, 3, 10.0, 0), None);
            assert_eq!(hit(&notes, 1, 24.0, 0), None);
            assert_eq!(hit(&notes, 1, 10.0, 1), None);
        }

        #[test]
        fn snaps_unless_free() {
            assert_eq!(snap_free(30.0, TICKS, false), 24.0);
            assert_eq!(snap_free(30.0, TICKS, true), 30.0);
            assert_eq!(snap_free(30.7, 1, false), 30.0);
        }
    }
}
