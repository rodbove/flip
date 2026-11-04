use eframe::egui::{Align2, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use flip_engine::keys::{CATEGORIES, PATCHES, in_category};
use flip_engine::{GROUPS, TICKS};

use crate::app::App;
use crate::theme::*;
use crate::view::L;

const KEYS: usize = 128;
const MIDDLE_C: i32 = 60;
const LOW: u8 = 36;
const HIGH: u8 = 96;
const TRAIL: f64 = 4.0;
const LANES: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lane {
    pub key: u8,
    pub start: f64,
    pub end: Option<f64>,
    pub live: bool,
}

pub struct Keyboard {
    pub lanes: Vec<Lane>,
    live: [u8; KEYS],
    playing: [bool; KEYS],
    pub heard: bool,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            lanes: Vec::new(),
            live: [0; KEYS],
            playing: [false; KEYS],
            heard: false,
        }
    }
}

pub fn key_of(note: i8) -> u8 {
    (MIDDLE_C + note as i32).clamp(0, KEYS as i32 - 1) as u8
}

pub fn black(key: u8) -> bool {
    matches!(key % 12, 1 | 3 | 6 | 8 | 10)
}

impl Keyboard {
    pub fn press(&mut self, key: u8, now: f64) {
        let k = key as usize % KEYS;
        self.live[k] = self.live[k].saturating_add(1);
        self.open(key, now, true);
    }

    pub fn release(&mut self, key: u8, now: f64) {
        let k = key as usize % KEYS;
        if self.live[k] == 0 {
            return;
        }
        self.live[k] -= 1;
        if self.live[k] == 0 {
            self.close(key, now, true);
        }
    }

    pub fn playback(&mut self, active: &[bool; KEYS], now: f64) {
        for (k, (&was, &is)) in self.playing.clone().iter().zip(active.iter()).enumerate() {
            match (was, is) {
                (false, true) => self.open(k as u8, now, false),
                (true, false) => self.close(k as u8, now, false),
                _ => {}
            }
        }
        self.playing = *active;
    }

    pub fn live_down(&self, key: u8) -> bool {
        self.live[key as usize % KEYS] > 0
    }

    pub fn play_down(&self, key: u8) -> bool {
        self.playing[key as usize % KEYS]
    }

    pub fn range(&self) -> (u8, u8) {
        let (mut low, mut high) = (LOW, HIGH);
        for lane in &self.lanes {
            low = low.min(lane.key - lane.key % 12);
            high = high.max((lane.key - lane.key % 12 + 12).min(KEYS as u8 - 1));
        }
        (low, high)
    }

    pub fn prune(&mut self, now: f64) {
        self.lanes.retain(|l| l.end.is_none_or(|e| now - e < TRAIL));
        if self.lanes.len() > LANES {
            let extra = self.lanes.len() - LANES;
            self.lanes.drain(..extra);
        }
    }

    fn open(&mut self, key: u8, now: f64, live: bool) {
        self.lanes.push(Lane {
            key,
            start: now,
            end: None,
            live,
        });
    }

    fn close(&mut self, key: u8, now: f64, live: bool) {
        if let Some(lane) = self
            .lanes
            .iter_mut()
            .rev()
            .find(|l| l.key == key && l.live == live && l.end.is_none())
        {
            lane.end = Some(now);
        }
    }
}

pub fn sounding(app: &App) -> [bool; KEYS] {
    let mut active = [false; KEYS];
    let Some(out) = &app.output else {
        return active;
    };
    for g in 0..GROUPS {
        let timing = out.status.timing(g);
        if !timing.playing {
            continue;
        }
        let pattern = &app.project.groups[g].patterns[timing.pattern];
        let tick = timing.position(pattern.length) * TICKS as f64;
        for n in &pattern.notes {
            let (a, b) = (n.start as f64, (n.start + n.length.max(1)) as f64);
            if tick >= a && tick < b {
                active[key_of(n.pitch) as usize] = true;
            }
        }
    }
    active
}

fn white_count(low: u8, high: u8) -> usize {
    (low..=high).filter(|k| !black(*k)).count()
}

pub fn name(key: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", NAMES[key as usize % 12], key as i32 / 12 - 1)
}

const PATCH_COLUMNS: usize = 4;
const PATCH_ROWS: usize = 3;

fn chip(ui: &Ui, l: &L, rect: Rect, id: &str, label: &str, on: bool) -> bool {
    let resp = ui.interact(rect, Id::unique(id), Sense::click());
    let painter = ui.painter();
    if on {
        painter.rect_filled(rect, l.s(3.0), ORANGE);
    } else {
        painter.rect_stroke(
            rect,
            l.s(3.0),
            Stroke::new(
                l.s(1.0),
                if resp.hovered() {
                    SCREEN_INK
                } else {
                    SCREEN_DIM
                },
            ),
            StrokeKind::Inside,
        );
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        l.f(9.0),
        if on { SCREEN } else { SCREEN_DIM },
    );
    resp.clicked()
}

fn patches(app: &mut App, ui: &Ui, l: &L, area: Rect) {
    let tab_h = l.s(15.0);
    let tab_w = area.width() / CATEGORIES.len() as f32;
    let mut pick_category = None;
    for (i, name) in CATEGORIES.iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(area.min.x + i as f32 * tab_w, area.min.y),
            vec2(tab_w, tab_h),
        );
        let resp = ui.interact(rect, Id::unique(("keys-tab", i)), Sense::click());
        let on = app.keys_category == i;
        if on {
            ui.painter()
                .rect_filled(rect.shrink(l.s(1.0)), l.s(3.0), SCREEN_FAINT);
        }
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            *name,
            l.f(9.0),
            if on || resp.hovered() {
                ORANGE
            } else {
                SCREEN_DIM
            },
        );
        if resp.clicked() {
            pick_category = Some(i);
        }
    }
    let current = &app.project.groups[app.group].keys.patch;
    let follow_pad = app.settings.pad_keys;
    let list = Rect::from_min_max(pos2(area.min.x, area.min.y + tab_h + l.s(3.0)), area.max);
    let row_h = list.height() / PATCH_ROWS as f32;
    let col_w = list.width() / PATCH_COLUMNS as f32;
    let mut load = None;
    for (n, index) in in_category(app.keys_category)
        .into_iter()
        .take(PATCH_ROWS * PATCH_COLUMNS)
        .enumerate()
    {
        let rect = Rect::from_min_size(
            list.min
                + vec2(
                    (n % PATCH_COLUMNS) as f32 * col_w,
                    (n / PATCH_COLUMNS) as f32 * row_h,
                ),
            vec2(col_w - l.s(4.0), row_h - l.s(2.0)),
        );
        let resp = ui.interact(rect, Id::unique(("keys-patch", index)), Sense::click());
        let name = PATCHES[index].name;
        let on = name == current && !follow_pad;
        let painter = ui.painter();
        if on {
            painter.rect_filled(rect, l.s(3.0), SCREEN_FAINT);
        } else if resp.hovered() {
            painter.rect_stroke(
                rect,
                l.s(3.0),
                Stroke::new(l.s(1.0), SCREEN_FAINT),
                StrokeKind::Inside,
            );
        }
        painter.text(
            rect.left_center() + vec2(l.s(6.0), 0.0),
            Align2::LEFT_CENTER,
            name,
            l.f(10.0),
            if on || resp.hovered() {
                ORANGE
            } else {
                SCREEN_INK
            },
        );
        if resp.clicked() {
            load = Some(index);
        }
    }
    if let Some(i) = pick_category {
        app.keys_category = i;
    }
    if let Some(i) = load {
        app.load_patch(i);
    }
}

pub fn view(app: &mut App, ui: &Ui, l: &L, body: Rect) {
    let toggle = Rect::from_min_size(
        pos2(body.max.x - l.s(118.0), body.min.y - l.s(1.0)),
        vec2(l.s(118.0), l.s(14.0)),
    );
    let label = if app.settings.pad_keys {
        format!("PLAYS PAD {}", app.pad_name(app.group, app.pad))
    } else {
        "PLAYS PATCH".to_string()
    };
    if chip(ui, l, toggle, "keys-sound", &label, app.settings.pad_keys) {
        let on = !app.settings.pad_keys;
        app.set_pad_keys(on);
        app.say(if on {
            "keyboard plays the selected pad"
        } else {
            "keyboard plays the keys patch"
        });
    }
    let browser = Rect::from_min_max(
        body.min + vec2(0.0, l.s(16.0)),
        pos2(body.max.x, body.min.y + l.s(80.0)),
    );
    patches(app, ui, l, browser);
    let painter = ui.painter();
    let (low, high) = app.keyboard.range();
    let whites = white_count(low, high).max(1);
    let keys_h = l.s(46.0);
    let keys = Rect::from_min_max(pos2(body.min.x, body.max.y - keys_h), body.max);
    let trail = Rect::from_min_max(
        pos2(body.min.x, browser.max.y + l.s(14.0)),
        pos2(body.max.x, keys.min.y - l.s(2.0)),
    );
    let ww = keys.width() / whites as f32;
    let mut x_of = [0.0f32; KEYS];
    let mut w_of = [0.0f32; KEYS];
    let mut white = 0usize;
    for k in low..=high {
        if black(k) {
            x_of[k as usize] = keys.min.x + white as f32 * ww - ww * 0.3;
            w_of[k as usize] = ww * 0.6;
        } else {
            x_of[k as usize] = keys.min.x + white as f32 * ww;
            w_of[k as usize] = ww;
            white += 1;
        }
    }
    let live: Vec<u8> = (low..=high)
        .filter(|k| app.keyboard.live_down(*k))
        .collect();
    let held = if live.is_empty() {
        "play a MIDI keyboard, click the keys, or TAB for keys mode".to_string()
    } else {
        live.iter().map(|k| name(*k)).collect::<Vec<_>>().join(" ")
    };
    let sound = if app.settings.pad_keys {
        app.pad_name(app.group, app.pad)
    } else {
        app.project.groups[app.group].keys.patch.clone()
    };
    painter.text(
        body.left_top(),
        Align2::LEFT_TOP,
        format!("KEYS {}  {sound}", (b'A' + app.group as u8) as char),
        l.f(11.0),
        SCREEN_INK,
    );
    let legend_y = browser.max.y + l.s(2.0);
    painter.text(
        pos2(body.min.x, legend_y),
        Align2::LEFT_TOP,
        held,
        l.f(10.0),
        if live.is_empty() { SCREEN_DIM } else { ORANGE },
    );
    painter.text(
        pos2(body.max.x, legend_y),
        Align2::RIGHT_TOP,
        "LIVE",
        l.f(9.0),
        ORANGE,
    );
    painter.text(
        pos2(body.max.x - l.s(36.0), legend_y),
        Align2::RIGHT_TOP,
        "PLAYBACK",
        l.f(9.0),
        BLUE,
    );
    for c in (low..=high).filter(|k| k % 12 == 0) {
        let x = x_of[c as usize];
        painter.line_segment(
            [pos2(x, trail.min.y), pos2(x, trail.max.y)],
            Stroke::new(l.s(1.0), SCREEN_FAINT),
        );
    }
    let speed = trail.height() as f64 / TRAIL;
    let clip = painter.with_clip_rect(trail);
    for lane in &app.keyboard.lanes {
        if lane.key < low || lane.key > high {
            continue;
        }
        let k = lane.key as usize;
        let end = lane.end.unwrap_or(app.now);
        let bottom = trail.max.y - ((app.now - end) * speed) as f32;
        let top = trail.max.y - ((app.now - lane.start) * speed) as f32;
        let w = if black(lane.key) {
            w_of[k]
        } else {
            w_of[k] * 0.7
        };
        let x = x_of[k] + (w_of[k] - w) / 2.0;
        let bar = Rect::from_min_max(
            pos2(x, top.max(trail.min.y - 1.0)),
            pos2(x + w, bottom.max(top + 2.0)),
        );
        let color = if lane.live { ORANGE } else { BLUE };
        clip.rect_filled(bar, l.s(2.0), color);
    }
    let press = |k: u8| -> Option<eframe::egui::Color32> {
        if app.keyboard.live_down(k) {
            Some(ORANGE)
        } else if app.keyboard.play_down(k) {
            Some(BLUE)
        } else {
            None
        }
    };
    for k in (low..=high).filter(|k| !black(*k)) {
        let r = Rect::from_min_size(pos2(x_of[k as usize], keys.min.y), vec2(ww, keys.height()));
        let face = press(k).unwrap_or(SCREEN_INK);
        painter.rect_filled(r.shrink(l.s(0.5)), l.s(1.5), face);
        if k % 12 == 0 {
            painter.text(
                pos2(r.center().x, r.max.y - l.s(3.0)),
                Align2::CENTER_BOTTOM,
                name(k),
                l.f(6.5),
                SCREEN,
            );
        }
    }
    for k in (low..=high).filter(|k| black(*k)) {
        let r = Rect::from_min_size(
            pos2(x_of[k as usize], keys.min.y),
            vec2(w_of[k as usize], keys.height() * 0.6),
        );
        let face = press(k).unwrap_or(SCREEN);
        painter.rect_filled(r, l.s(1.5), face);
        painter.rect_stroke(
            r,
            l.s(1.5),
            Stroke::new(l.s(0.8), SCREEN_DIM),
            StrokeKind::Inside,
        );
    }
    let resp = ui.interact(keys, Id::unique("keys-board"), Sense::click_and_drag());
    if resp.is_pointer_button_down_on()
        && app.mouse.is_none()
        && let Some(p) = resp.interact_pointer_pos()
    {
        let hit = |k: u8, tall: f32| {
            Rect::from_min_size(
                pos2(x_of[k as usize], keys.min.y),
                vec2(w_of[k as usize], keys.height() * tall),
            )
            .contains(p)
        };
        let key = (low..=high)
            .filter(|k| black(*k))
            .find(|k| hit(*k, 0.6))
            .or_else(|| (low..=high).filter(|k| !black(*k)).find(|k| hit(*k, 1.0)));
        if let Some(k) = key {
            let note = (k as i32 - MIDDLE_C).clamp(-48, 48) as i8;
            let velocity = 0.5 + 0.4 * ((p.y - keys.min.y) / keys.height()).clamp(0.0, 1.0);
            let (g, pad) = (app.group, app.piano_pad());
            app.press(g, pad, note, velocity, false, true);
            app.mouse = Some((g, pad, note));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod when_a_key_is_played_live {
        use super::*;

        #[test]
        fn opens_and_closes_a_lane() {
            let mut k = Keyboard::default();
            k.press(60, 1.0);
            assert!(k.live_down(60));
            k.release(60, 1.5);
            assert!(!k.live_down(60));
            assert_eq!(k.lanes[0].end, Some(1.5));
            assert!(k.lanes[0].live);
        }

        #[test]
        fn ignores_a_release_without_a_press() {
            let mut k = Keyboard::default();
            k.release(60, 1.0);
            assert!(k.lanes.is_empty());
        }
    }

    mod when_the_song_plays_notes {
        use super::*;

        #[test]
        fn follows_the_sounding_keys() {
            let mut k = Keyboard::default();
            let mut active = [false; KEYS];
            active[64] = true;
            k.playback(&active, 2.0);
            assert!(k.play_down(64));
            active[64] = false;
            k.playback(&active, 2.5);
            assert!(!k.play_down(64));
            assert_eq!(k.lanes.len(), 1);
            assert!(!k.lanes[0].live);
            assert_eq!(k.lanes[0].end, Some(2.5));
        }
    }

    mod when_lanes_get_old {
        use super::*;

        #[test]
        fn drops_them_from_the_trail() {
            let mut k = Keyboard::default();
            k.press(60, 0.0);
            k.release(60, 0.5);
            k.press(62, 9.0);
            k.prune(9.0);
            assert_eq!(k.lanes.len(), 1);
            assert_eq!(k.lanes[0].key, 62);
        }
    }

    mod when_a_key_is_out_of_range {
        use super::*;

        #[test]
        fn widens_the_keyboard_by_octaves() {
            let mut k = Keyboard::default();
            k.press(30, 0.0);
            assert_eq!(k.range(), (24, HIGH));
        }
    }

    mod when_naming_keys {
        use super::*;

        #[test]
        fn uses_middle_c_as_c4() {
            assert_eq!(name(60), "C4");
            assert_eq!(name(61), "C#4");
            assert_eq!(key_of(0), 60);
            assert_eq!(key_of(-24), 36);
        }
    }
}
