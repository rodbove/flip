use crate::dsp::Rng;
use crate::preset::{self, amp, analog, fm, morph, op, string};
use crate::project::{
    Filter, GROUPS, Mode, PADS, Pad, Pattern, Project, Source, Step, Synth, Wave,
};

const DRUMS: usize = 0;
const BASS: usize = 1;
const KEYS: usize = 2;
const LEAD: usize = 3;
const FX_PADS: usize = 4;
const BASS_BASE: f32 = -24.0;
const KEYS_BASE: f32 = -12.0;
const LEAD_BASE: f32 = 0.0;
const NOTE_NAMES: [&str; 12] = [
    "c", "c#", "d", "d#", "e", "f", "f#", "g", "g#", "a", "a#", "b",
];

#[derive(Clone, Copy)]
struct Hit {
    name: &'static str,
    a: Wave,
    b: Wave,
    mix: f32,
    pitch: f32,
    sweep: f32,
    decay: f32,
    filter: Filter,
    cutoff: f32,
}

#[allow(clippy::too_many_arguments)]
const fn h(
    name: &'static str,
    a: Wave,
    b: Wave,
    mix: f32,
    pitch: f32,
    sweep: f32,
    decay: f32,
    filter: Filter,
    cutoff: f32,
) -> Hit {
    Hit {
        name,
        a,
        b,
        mix,
        pitch,
        sweep,
        decay,
        filter,
        cutoff,
    }
}

use Filter::{Band, High, Low};
use Wave::{Noise, Saw, Sine, Square, Triangle};

const KIT: [Hit; PADS] = [
    h("kick", Sine, Sine, 0.0, -26.0, 30.0, 0.35, Low, 1.0),
    h("kick 2", Sine, Triangle, 0.2, -29.0, 24.0, 0.7, Low, 0.6),
    h("snare", Triangle, Noise, 0.65, -5.0, 8.0, 0.18, Band, 0.8),
    h("clap", Noise, Noise, 1.0, 0.0, 0.0, 0.14, Band, 0.68),
    h("hat", Noise, Noise, 1.0, 0.0, 0.0, 0.05, High, 0.86),
    h("open hat", Noise, Noise, 1.0, 0.0, 0.0, 0.35, High, 0.84),
    h("rim", Square, Square, 0.0, 14.0, 4.0, 0.03, High, 0.6),
    h("cowbell", Square, Square, 0.5, 8.0, 0.0, 0.2, Band, 0.72),
    h("conga", Sine, Triangle, 0.3, -2.0, 6.0, 0.22, Low, 0.9),
    h("tom lo", Sine, Sine, 0.0, -14.0, 12.0, 0.3, Low, 1.0),
    h("tom hi", Sine, Sine, 0.0, -9.0, 12.0, 0.25, Low, 1.0),
    h("shaker", Noise, Noise, 1.0, 0.0, 0.0, 0.08, High, 0.92),
    h("crash", Noise, Square, 0.8, 20.0, 0.0, 1.1, High, 0.8),
    h("zap", Saw, Saw, 0.0, 0.0, 36.0, 0.12, Low, 0.9),
    h("808", Sine, Sine, 0.0, -28.0, 5.0, 1.2, Low, 1.0),
    h("impact", Noise, Sine, 0.5, -20.0, 24.0, 0.8, Low, 0.5),
];

type Patch = fn() -> Pad;

pub struct Mood {
    pub name: &'static str,
    pub blurb: &'static str,
    pub tempo: f32,
    pub swing: f32,
    root: i8,
    scale: &'static [i8],
    kit: fn() -> [Hit; PADS],
    tone: f32,
    bass: &'static [(&'static str, Patch)],
    keys: &'static [(&'static str, Patch)],
    lead: &'static [(&'static str, Patch)],
    beat: &'static [(usize, &'static str)],
    line: &'static str,
}

const MAJOR_PENT: &[i8] = &[0, 2, 4, 7, 9];
const MINOR: &[i8] = &[0, 2, 3, 5, 7, 8, 10];
const DORIAN: &[i8] = &[0, 2, 3, 5, 7, 9, 10];
const MAJOR: &[i8] = &[0, 2, 4, 5, 7, 9, 11];
const LYDIAN: &[i8] = &[0, 2, 4, 6, 7, 9, 11];
const PHRYGIAN: &[i8] = &[0, 1, 3, 5, 7, 8, 10];
const HARMONIC: &[i8] = &[0, 2, 3, 5, 7, 8, 11];
const MIXOLYDIAN: &[i8] = &[0, 2, 4, 5, 7, 9, 10];

fn factory(name: &str) -> Pad {
    preset::factory()
        .into_iter()
        .find(|p| p.name == name)
        .map(|p| p.sound)
        .unwrap_or_default()
}

fn e_piano() -> Pad {
    factory("E-Piano")
}
fn clav() -> Pad {
    factory("Clav")
}
fn organ() -> Pad {
    factory("Organ")
}
fn fm_bass() -> Pad {
    factory("FM Bass")
}
fn analog_bass() -> Pad {
    factory("Analog Bass")
}
fn acid() -> Pad {
    factory("Acid")
}
fn reese() -> Pad {
    factory("Reese")
}
fn pluck_bass() -> Pad {
    factory("Pluck Bass")
}
fn square_lead() -> Pad {
    factory("Square Lead")
}
fn fm_lead() -> Pad {
    factory("FM Lead")
}
fn saw_lead() -> Pad {
    factory("Saw Lead")
}
fn warm_pad() -> Pad {
    factory("Warm Pad")
}
fn strings() -> Pad {
    factory("Strings")
}
fn analog_pad() -> Pad {
    factory("Analog Pad")
}
fn violin() -> Pad {
    factory("Violin")
}
fn cello() -> Pad {
    factory("Cello")
}
fn harp() -> Pad {
    factory("Harp")
}
fn nylon() -> Pad {
    factory("Nylon")
}
fn steel() -> Pad {
    factory("Steel")
}
fn analog_pluck() -> Pad {
    factory("Analog Pluck")
}
fn bells() -> Pad {
    factory("DX Bells")
}
fn glass() -> Pad {
    factory("Glass")
}
fn marimba() -> Pad {
    factory("Marimba")
}
fn brass() -> Pad {
    factory("Brass")
}
fn synth_brass() -> Pad {
    factory("Synth Brass")
}

fn sub() -> Pad {
    morph(0.0, 0.0, 0.6, 0.2, 0.5, amp(0.003, 0.8, 0.8, 0.15))
}

fn boom() -> Pad {
    let mut pad = analog(Sine, Sine, 0.0, 0.0, 1.0, amp(0.002, 1.4, 0.0, 0.6));
    pad.mode = Mode::OneShot;
    pad.synth.sweep = 7.0;
    pad
}

fn chip_square() -> Pad {
    analog(Square, Square, 12.0, 0.25, 1.0, amp(0.001, 0.15, 0.6, 0.05))
}

fn chip_tri() -> Pad {
    analog(
        Triangle,
        Triangle,
        0.0,
        0.0,
        1.0,
        amp(0.001, 0.2, 0.8, 0.04),
    )
}

fn chip_arp() -> Pad {
    analog(Square, Square, 0.0, 0.0, 1.0, amp(0.001, 0.12, 0.0, 0.05))
}

fn flute() -> Pad {
    Pad {
        resonance: 0.1,
        ..analog(Sine, Noise, 0.0, 0.06, 0.62, amp(0.06, 0.3, 0.85, 0.2))
    }
}

fn whistle() -> Pad {
    analog(Sine, Triangle, 0.02, 0.2, 1.0, amp(0.03, 0.2, 0.9, 0.15))
}

fn upright() -> Pad {
    string(0.75, 0.25, 0.35, 0.0, amp(0.001, 1.0, 1.0, 0.25))
}

fn vibes() -> Pad {
    fm(
        3,
        [
            op(1.0, 1.0, 2.5, 0.0),
            op(4.0, 0.3, 0.5, 0.0),
            op(1.0, 0.5, 2.0, 2.0),
            op(3.0, 0.2, 0.3, 0.0),
        ],
        0.0,
        1.0,
        0.5,
        amp(0.001, 2.5, 0.0, 1.2),
    )
}

fn cavaquinho() -> Pad {
    string(0.3, 0.85, 0.5, 0.0, amp(0.001, 0.6, 1.0, 0.2))
}

fn accordion() -> Pad {
    morph(1.7, 0.18, 0.0, 0.1, 0.6, amp(0.04, 0.2, 0.9, 0.15))
}

fn dark_pad() -> Pad {
    Pad {
        resonance: 0.4,
        env_amount: 0.15,
        ..morph(2.4, 0.35, 0.5, 0.3, 0.3, amp(1.0, 0.6, 0.9, 2.0))
    }
}

fn dusty_keys() -> Pad {
    Pad {
        cutoff: 0.62,
        ..e_piano()
    }
}

fn tape_pad() -> Pad {
    Pad {
        cutoff: 0.4,
        ..warm_pad()
    }
}

fn stab() -> Pad {
    Pad {
        env_amount: 0.4,
        mode: Mode::Gate,
        ..analog(Saw, Saw, 7.0, 0.5, 0.45, amp(0.001, 0.3, 0.0, 0.15))
    }
}

fn hoover() -> Pad {
    Pad {
        resonance: 0.3,
        ..morph(2.7, 0.45, 0.3, 0.5, 0.55, amp(0.01, 0.3, 0.8, 0.3))
    }
}

fn kit_dusty() -> [Hit; PADS] {
    let mut k = KIT;
    k[0] = h("kick", Sine, Triangle, 0.15, -27.0, 18.0, 0.45, Low, 0.7);
    k[1] = h("kick soft", Sine, Sine, 0.0, -28.0, 12.0, 0.4, Low, 0.6);
    k[2] = h("snare", Triangle, Noise, 0.7, -6.0, 6.0, 0.22, Band, 0.7);
    k[3] = h("snare 2", Noise, Square, 0.8, -3.0, 4.0, 0.16, Band, 0.62);
    k[4] = h("hat", Noise, Noise, 1.0, 0.0, 0.0, 0.06, High, 0.78);
    k[5] = h("open hat", Noise, Noise, 1.0, 0.0, 0.0, 0.3, High, 0.76);
    k[7] = h("tamb", Noise, Square, 0.8, 18.0, 0.0, 0.12, High, 0.8);
    k[12] = h("vinyl", Noise, Noise, 1.0, 0.0, 0.0, 2.0, Band, 0.85);
    k[13] = h("snap", Noise, Noise, 1.0, 0.0, 0.0, 0.06, Band, 0.75);
    k
}

fn kit_club() -> [Hit; PADS] {
    let mut k = KIT;
    k[0] = h("kick", Sine, Sine, 0.0, -25.0, 28.0, 0.5, Low, 1.0);
    k[1] = h("kick long", Sine, Sine, 0.0, -26.0, 30.0, 0.9, Low, 0.8);
    k[2] = h("snare", Triangle, Noise, 0.75, -4.0, 10.0, 0.2, Band, 0.82);
    k[3] = h("clap", Noise, Noise, 1.0, 0.0, 0.0, 0.2, Band, 0.7);
    k[4] = h("hat", Noise, Noise, 1.0, 0.0, 0.0, 0.04, High, 0.9);
    k[5] = h("open hat", Noise, Noise, 1.0, 0.0, 0.0, 0.45, High, 0.88);
    k[7] = h("ride", Noise, Square, 0.7, 24.0, 0.0, 0.8, High, 0.82);
    k[11] = h("shaker", Noise, Noise, 1.0, 0.0, 0.0, 0.06, High, 0.95);
    k
}

fn kit_trap() -> [Hit; PADS] {
    let mut k = KIT;
    k[0] = h("808", Sine, Sine, 0.0, -28.0, 10.0, 1.6, Low, 1.0);
    k[1] = h("kick", Sine, Triangle, 0.3, -24.0, 34.0, 0.25, Low, 0.9);
    k[2] = h("snare", Noise, Triangle, 0.35, 0.0, 6.0, 0.22, High, 0.6);
    k[3] = h("clap", Noise, Noise, 1.0, 0.0, 0.0, 0.25, Band, 0.72);
    k[4] = h("hat", Noise, Noise, 1.0, 0.0, 0.0, 0.03, High, 0.95);
    k[5] = h("open hat", Noise, Noise, 1.0, 0.0, 0.0, 0.25, High, 0.9);
    k[7] = h("perc", Sine, Square, 0.3, 19.0, 12.0, 0.08, Band, 0.85);
    k[13] = h("laser", Saw, Square, 0.3, 12.0, 48.0, 0.2, Low, 0.9);
    k[14] = h("808 long", Sine, Sine, 0.0, -31.0, 8.0, 2.5, Low, 1.0);
    k
}

fn kit_dream() -> [Hit; PADS] {
    let mut k = KIT;
    k[0] = h("kick soft", Sine, Sine, 0.0, -27.0, 14.0, 0.4, Low, 0.55);
    k[2] = h("brush", Noise, Noise, 1.0, 0.0, 0.0, 0.3, Low, 0.72);
    k[3] = h("snap", Noise, Noise, 1.0, 0.0, 0.0, 0.08, Band, 0.8);
    k[7] = h("chime", Sine, Triangle, 0.3, 31.0, 0.0, 1.5, Low, 1.0);
    k[8] = h("chime 2", Sine, Sine, 0.0, 36.0, 0.0, 1.2, Low, 1.0);
    k[12] = h("air", Noise, Noise, 1.0, 0.0, 0.0, 2.5, Band, 0.9);
    k
}

fn kit_funk() -> [Hit; PADS] {
    let mut k = KIT;
    k[0] = h("kick", Sine, Triangle, 0.2, -25.0, 22.0, 0.25, Low, 0.85);
    k[2] = h("snare", Triangle, Noise, 0.6, -3.0, 10.0, 0.14, Band, 0.85);
    k[3] = h("ghost", Triangle, Noise, 0.6, -3.0, 10.0, 0.06, Band, 0.8);
    k[4] = h("hat", Noise, Noise, 1.0, 0.0, 0.0, 0.045, High, 0.88);
    k[12] = h("tamb", Noise, Square, 0.8, 18.0, 0.0, 0.12, High, 0.85);
    k
}

fn kit_chip() -> [Hit; PADS] {
    let mut k = KIT;
    k[0] = h("kick", Square, Square, 0.0, -24.0, 36.0, 0.15, Low, 0.5);
    k[1] = h(
        "kick 2", Triangle, Triangle, 0.0, -24.0, 24.0, 0.2, Low, 1.0,
    );
    k[2] = h("snare", Noise, Square, 0.8, 0.0, 12.0, 0.12, Low, 0.75);
    k[3] = h("noise", Noise, Noise, 1.0, 0.0, 0.0, 0.2, Low, 0.8);
    k[4] = h("hat", Noise, Noise, 1.0, 0.0, 0.0, 0.03, High, 0.8);
    k[5] = h("open hat", Noise, Noise, 1.0, 0.0, 0.0, 0.2, High, 0.78);
    k[6] = h("blip", Square, Square, 0.0, 24.0, 0.0, 0.05, Low, 1.0);
    k[7] = h("coin", Square, Square, 0.0, 28.0, -5.0, 0.25, Low, 1.0);
    k[8] = h("jump", Square, Square, 0.0, 12.0, -24.0, 0.2, Low, 1.0);
    k[13] = h("laser", Square, Square, 0.0, 24.0, 36.0, 0.15, Low, 1.0);
    k[15] = h("boom", Noise, Square, 0.5, -24.0, 12.0, 0.5, Low, 0.55);
    k
}

fn kit_organic() -> [Hit; PADS] {
    let mut k = KIT;
    k[0] = h("kick", Triangle, Sine, 0.4, -27.0, 14.0, 0.35, Low, 0.5);
    k[2] = h("snare", Noise, Triangle, 0.4, -4.0, 4.0, 0.25, Band, 0.62);
    k[6] = h("wood", Square, Sine, 0.4, 20.0, 2.0, 0.04, Band, 0.78);
    k[7] = h("block", Square, Sine, 0.4, 15.0, 2.0, 0.05, Band, 0.72);
    k[8] = h("djembe", Sine, Noise, 0.15, -6.0, 8.0, 0.25, Low, 0.75);
    k[12] = h("rain", Noise, Noise, 1.0, 0.0, 0.0, 1.5, Band, 0.92);
    k
}

fn kit_dark() -> [Hit; PADS] {
    let mut k = KIT;
    k[0] = h("kick", Sine, Square, 0.15, -30.0, 40.0, 0.6, Low, 0.7);
    k[1] = h("rumble", Sine, Noise, 0.2, -32.0, 6.0, 1.5, Low, 0.35);
    k[2] = h("snare", Noise, Square, 0.6, -2.0, 14.0, 0.25, Band, 0.75);
    k[3] = h("clang", Square, Square, 0.5, 6.0, 0.0, 0.4, Band, 0.6);
    k[7] = h("metal", Square, Square, 0.5, 13.0, 0.0, 0.5, High, 0.7);
    k[13] = h("drone hit", Saw, Saw, 0.5, -12.0, -12.0, 1.5, Low, 0.35);
    k
}

fn kit_carioca() -> [Hit; PADS] {
    let mut k = KIT;
    k[0] = h("tamborzao", Sine, Sine, 0.0, -27.0, 14.0, 1.0, Low, 1.0);
    k[1] = h("kick", Sine, Triangle, 0.3, -24.0, 30.0, 0.3, Low, 0.9);
    k[2] = h("snare", Noise, Triangle, 0.4, 0.0, 8.0, 0.2, Band, 0.75);
    k[3] = h("clap", Noise, Noise, 1.0, 0.0, 0.0, 0.25, Band, 0.7);
    k[7] = h("atabaque", Sine, Triangle, 0.25, -8.0, 10.0, 0.3, Low, 0.85);
    k[8] = h("conga", Sine, Triangle, 0.3, -1.0, 8.0, 0.18, Low, 0.9);
    k[12] = h("beatbox", Noise, Sine, 0.6, -12.0, 18.0, 0.12, Low, 0.6);
    k[13] = h("tuim", Sine, Sine, 0.0, 31.0, -12.0, 0.25, Low, 1.0);
    k
}

fn kit_bossa() -> [Hit; PADS] {
    let mut k = KIT;
    k[0] = h("kick soft", Sine, Sine, 0.0, -27.0, 10.0, 0.35, Low, 0.5);
    k[2] = h("brush", Noise, Noise, 1.0, 0.0, 0.0, 0.32, Low, 0.75);
    k[3] = h("brush tap", Noise, Noise, 1.0, 0.0, 0.0, 0.1, Band, 0.72);
    k[6] = h("cross stick", Square, Sine, 0.5, 12.0, 4.0, 0.05, Band, 0.7);
    k[7] = h("ride", Noise, Square, 0.6, 24.0, 0.0, 1.0, High, 0.8);
    k[11] = h("ganza", Noise, Noise, 1.0, 0.0, 0.0, 0.07, High, 0.88);
    k
}

fn kit_samba() -> [Hit; PADS] {
    let mut k = KIT;
    k[0] = h("surdo", Sine, Sine, 0.0, -24.0, 6.0, 0.9, Low, 0.8);
    k[1] = h("surdo lo", Sine, Sine, 0.0, -27.0, 6.0, 1.1, Low, 0.7);
    k[2] = h("caixa", Noise, Triangle, 0.25, 2.0, 6.0, 0.12, Band, 0.85);
    k[3] = h(
        "caixa ghost",
        Noise,
        Triangle,
        0.25,
        2.0,
        6.0,
        0.05,
        Band,
        0.85,
    );
    k[6] = h("tamborim", Square, Sine, 0.5, 22.0, 6.0, 0.06, Band, 0.8);
    k[7] = h(
        "agogo hi", Square, Triangle, 0.5, 26.0, 0.0, 0.3, Band, 0.82,
    );
    k[8] = h("agogo lo", Square, Triangle, 0.5, 21.0, 0.0, 0.3, Band, 0.8);
    k[9] = h("pandeiro", Noise, Sine, 0.5, 6.0, 4.0, 0.15, Band, 0.85);
    k[10] = h("repique", Sine, Noise, 0.2, -2.0, 10.0, 0.15, Low, 0.95);
    k[11] = h("ganza", Noise, Noise, 1.0, 0.0, 0.0, 0.07, High, 0.9);
    k[13] = h("cuica", Sine, Sine, 0.0, 14.0, -10.0, 0.35, Low, 1.0);
    k[14] = h("cuica hi", Sine, Sine, 0.0, 19.0, -12.0, 0.25, Low, 1.0);
    k[15] = h("apito", Square, Sine, 0.6, 31.0, 0.0, 0.3, Band, 0.9);
    k
}

fn kit_mpb() -> [Hit; PADS] {
    let mut k = kit_bossa();
    k[0] = h("kick", Sine, Triangle, 0.15, -26.0, 16.0, 0.35, Low, 0.6);
    k[2] = h("snare", Triangle, Noise, 0.55, -5.0, 6.0, 0.18, Band, 0.72);
    k[8] = h("conga", Sine, Triangle, 0.3, -2.0, 6.0, 0.22, Low, 0.9);
    k[9] = h("pandeiro", Noise, Sine, 0.5, 6.0, 4.0, 0.15, Band, 0.85);
    k[12] = h("triangulo", Sine, Sine, 0.0, 41.0, 0.0, 0.8, High, 0.6);
    k
}

pub const MOODS: [Mood; 12] = [
    Mood {
        name: "DUSTY",
        blurb: "lo-fi boom bap",
        tempo: 88.0,
        swing: 0.35,
        root: 9,
        scale: DORIAN,
        kit: kit_dusty,
        tone: 0.8,
        bass: &[("upright", upright), ("sub", sub), ("fm bass", fm_bass)],
        keys: &[
            ("keys", dusty_keys),
            ("tape pad", tape_pad),
            ("vibes", vibes),
        ],
        lead: &[("flute", flute), ("bells", bells), ("nylon", nylon)],
        beat: &[
            (0, "x......x..x....."),
            (2, "....x.......x..o"),
            (4, "x.x.x.x.x.x.x.x."),
        ],
        line: "0.........0.4...",
    },
    Mood {
        name: "CLUB",
        blurb: "house and techno",
        tempo: 124.0,
        swing: 0.1,
        root: 5,
        scale: MINOR,
        kit: kit_club,
        tone: 1.0,
        bass: &[("bass", analog_bass), ("acid", acid), ("sub", sub)],
        keys: &[("stab", stab), ("organ", organ), ("pad", analog_pad)],
        lead: &[("saw", saw_lead), ("hoover", hoover), ("bells", glass)],
        beat: &[
            (0, "x...x...x...x..."),
            (3, "....x.......x..."),
            (5, "..x...x...x...x."),
            (11, "oxoxoxoxoxoxoxox"),
        ],
        line: "..0...0...0...7.",
    },
    Mood {
        name: "TRAP",
        blurb: "808s and rolls",
        tempo: 140.0,
        swing: 0.0,
        root: 1,
        scale: HARMONIC,
        kit: kit_trap,
        tone: 1.0,
        bass: &[("808", boom), ("reese", reese), ("sub", sub)],
        keys: &[("bells", bells), ("pad", dark_pad), ("pluck", analog_pluck)],
        lead: &[
            ("lead", square_lead),
            ("flute", flute),
            ("brass", synth_brass),
        ],
        beat: &[
            (0, "x......x..x....."),
            (3, "........x......."),
            (4, "x.x.x.x.xxx.x.xx"),
        ],
        line: "0......0..0.....",
    },
    Mood {
        name: "DREAM",
        blurb: "ambient and soft",
        tempo: 80.0,
        swing: 0.0,
        root: 2,
        scale: LYDIAN,
        kit: kit_dream,
        tone: 0.9,
        bass: &[("sub", sub), ("pluck bass", pluck_bass), ("cello", cello)],
        keys: &[("pad", warm_pad), ("glass", glass), ("harp", harp)],
        lead: &[("whistle", whistle), ("strings", strings), ("bells", bells)],
        beat: &[
            (0, "x.......x......."),
            (2, "....x.......x..."),
            (11, "x.x.x.x.x.x.x.x."),
        ],
        line: "0.......4.......",
    },
    Mood {
        name: "FUNK",
        blurb: "tight and greasy",
        tempo: 104.0,
        swing: 0.2,
        root: 4,
        scale: DORIAN,
        kit: kit_funk,
        tone: 1.0,
        bass: &[
            ("slap", fm_bass),
            ("bass", analog_bass),
            ("pluck", pluck_bass),
        ],
        keys: &[("clav", clav), ("organ", organ), ("e-piano", e_piano)],
        lead: &[
            ("brass", brass),
            ("lead", fm_lead),
            ("synth brass", synth_brass),
        ],
        beat: &[
            (0, "x..x..x...x..x.."),
            (2, "....x..o.o..x..o"),
            (4, "xoxoxoxoxoxoxoxo"),
        ],
        line: "0..0..0...0.3.4.",
    },
    Mood {
        name: "CHIP",
        blurb: "8-bit game music",
        tempo: 150.0,
        swing: 0.0,
        root: 0,
        scale: MAJOR,
        kit: kit_chip,
        tone: 1.0,
        bass: &[
            ("tri bass", chip_tri),
            ("square bass", chip_square),
            ("sub", sub),
        ],
        keys: &[
            ("arp", chip_arp),
            ("square", chip_square),
            ("tri", chip_tri),
        ],
        lead: &[("lead", chip_square), ("arp", chip_arp), ("tri", chip_tri)],
        beat: &[
            (0, "x...x...x...x..."),
            (2, "....x.......x..."),
            (4, "x.x.x.x.x.x.x.x."),
        ],
        line: "0.0.4.0.0.0.4.7.",
    },
    Mood {
        name: "ORGANIC",
        blurb: "plucks and strings",
        tempo: 96.0,
        swing: 0.15,
        root: 7,
        scale: MAJOR_PENT,
        kit: kit_organic,
        tone: 0.9,
        bass: &[
            ("upright", upright),
            ("pluck bass", pluck_bass),
            ("cello", cello),
        ],
        keys: &[("nylon", nylon), ("harp", harp), ("marimba", marimba)],
        lead: &[("violin", violin), ("flute", flute), ("steel", steel)],
        beat: &[
            (0, "x.....x...x....."),
            (6, "....x.......x..."),
            (11, "oxoxoxoxoxoxoxox"),
        ],
        line: "0.....2...3.....",
    },
    Mood {
        name: "DARK",
        blurb: "heavy and cold",
        tempo: 130.0,
        swing: 0.0,
        root: 2,
        scale: PHRYGIAN,
        kit: kit_dark,
        tone: 0.85,
        bass: &[("reese", reese), ("acid", acid), ("sub", sub)],
        keys: &[
            ("dark pad", dark_pad),
            ("strings", strings),
            ("bells", glass),
        ],
        lead: &[("hoover", hoover), ("saw", saw_lead), ("cello", cello)],
        beat: &[
            (0, "x...x...x...x..."),
            (2, "....x.......x..."),
            (7, "..x..x....x..x.."),
        ],
        line: "0.0.1...0.0.....",
    },
    Mood {
        name: "CARIOCA",
        blurb: "funk, tamborzao",
        tempo: 130.0,
        swing: 0.0,
        root: 7,
        scale: MINOR,
        kit: kit_carioca,
        tone: 1.0,
        bass: &[("808", boom), ("sub", sub), ("bass", analog_bass)],
        keys: &[("stab", stab), ("bells", bells), ("pluck", analog_pluck)],
        lead: &[
            ("lead", square_lead),
            ("flute", flute),
            ("brass", synth_brass),
        ],
        beat: &[
            (0, "x..x..x.x..x..x."),
            (3, "....x.......x..."),
            (7, "..x...x..x....x."),
            (12, "x.......x......."),
        ],
        line: "0..0..0.0..0..0.",
    },
    Mood {
        name: "BOSSA",
        blurb: "soft and swung",
        tempo: 120.0,
        swing: 0.15,
        root: 2,
        scale: MAJOR,
        kit: kit_bossa,
        tone: 0.9,
        bass: &[
            ("upright", upright),
            ("pluck bass", pluck_bass),
            ("sub", sub),
        ],
        keys: &[("nylon", nylon), ("e-piano", e_piano), ("vibes", vibes)],
        lead: &[("flute", flute), ("whistle", whistle), ("strings", strings)],
        beat: &[
            (0, "x..x....x..x...."),
            (6, "x..x..x...x..x.."),
            (11, "xoxoxoxoxoxoxoxo"),
        ],
        line: "0.....4.0.....4.",
    },
    Mood {
        name: "SAMBA",
        blurb: "surdo, tamborim",
        tempo: 100.0,
        swing: 0.1,
        root: 0,
        scale: MAJOR,
        kit: kit_samba,
        tone: 1.0,
        bass: &[("bass", pluck_bass), ("upright", upright), ("tuba", brass)],
        keys: &[
            ("cavaquinho", cavaquinho),
            ("nylon", nylon),
            ("accordion", accordion),
        ],
        lead: &[("flute", flute), ("brass", brass), ("whistle", whistle)],
        beat: &[
            (1, "x.......x......."),
            (0, "....x.......x..."),
            (2, "xoxxoxoxxoxxoxox"),
            (6, "x.x.xx.x.x.xx.x."),
            (11, "xoooxoooxoooxooo"),
        ],
        line: "0...4...0...4...",
    },
    Mood {
        name: "MPB",
        blurb: "warm and acoustic",
        tempo: 96.0,
        swing: 0.1,
        root: 9,
        scale: MIXOLYDIAN,
        kit: kit_mpb,
        tone: 0.95,
        bass: &[
            ("bass", pluck_bass),
            ("upright", upright),
            ("fm bass", fm_bass),
        ],
        keys: &[
            ("e-piano", e_piano),
            ("nylon", nylon),
            ("accordion", accordion),
        ],
        lead: &[("flute", flute), ("strings", strings), ("vibes", vibes)],
        beat: &[
            (0, "x.....x.x......."),
            (6, "....x.......x..."),
            (11, "x.x.x.x.x.x.x.x."),
            (9, "..o...o...o...o."),
        ],
        line: "0...0.4.0...4.5.",
    },
];

pub fn find(name: &str) -> Option<usize> {
    MOODS.iter().position(|m| m.name == name)
}

fn hit_pad(hit: &Hit, tone: f32) -> Pad {
    Pad {
        name: hit.name.into(),
        source: Source::Synth,
        pitch: hit.pitch,
        attack: 0.001,
        decay: hit.decay,
        sustain: 0.0,
        release: hit.decay,
        filter: hit.filter,
        cutoff: (hit.cutoff * tone).clamp(0.0, 1.0),
        synth: Synth {
            wave_a: hit.a,
            wave_b: hit.b,
            detune: if hit.a == Square && hit.b == Square {
                7.0
            } else {
                0.0
            },
            mix: hit.mix,
            sweep: hit.sweep,
        },
        ..Pad::default()
    }
}

fn degree_pitch(mood: &Mood, degree: usize) -> i32 {
    let n = mood.scale.len();
    mood.root as i32 + 12 * (degree / n) as i32 + mood.scale[degree % n] as i32
}

pub fn pad_of(degree: usize) -> usize {
    let row = 3 - (degree / 4) % 4;
    row * 4 + degree % 4
}

fn note_name(semis: i32) -> &'static str {
    NOTE_NAMES[semis.rem_euclid(12) as usize]
}

fn melodic(mood: &Mood, patch: (&str, Patch), base: f32, degree: usize) -> Pad {
    let semis = degree_pitch(mood, degree);
    let mut pad = patch.1();
    pad.pitch = base + semis as f32;
    pad.name = format!("{} {}", patch.0, note_name(semis));
    pad.choke = 0;
    pad.cutoff = (pad.cutoff * (0.5 + mood.tone * 0.5)).clamp(0.0, 1.0);
    pad
}

fn fx(index: usize, mood: &Mood) -> Pad {
    let mut pad = match index {
        0 => Pad {
            name: "riser".into(),
            env_amount: 0.9,
            mode: Mode::OneShot,
            ..analog(Noise, Saw, 0.0, 0.3, 0.1, amp(2.0, 0.2, 0.0, 0.3))
        },
        1 => hit_pad(
            &h("drop", Sine, Noise, 0.2, -12.0, 36.0, 1.5, Low, 0.8),
            1.0,
        ),
        2 => hit_pad(
            &h("laser", Saw, Square, 0.4, 12.0, 48.0, 0.25, Low, 0.9),
            1.0,
        ),
        _ => Pad {
            name: "texture".into(),
            mode: Mode::OneShot,
            ..morph(2.2, 0.4, 0.0, 0.2, 0.35, amp(0.8, 2.0, 0.0, 1.5))
        },
    };
    pad.cutoff = (pad.cutoff * mood.tone).clamp(0.0, 1.0);
    pad
}

pub fn sounds(index: usize) -> [Vec<Pad>; GROUPS] {
    let mood = &MOODS[index % MOODS.len()];
    let mut drums: Vec<Pad> = (mood.kit)().iter().map(|h| hit_pad(h, mood.tone)).collect();
    for (i, pad) in drums.iter_mut().enumerate() {
        if i == 4 || i == 5 {
            pad.choke = 1;
        }
        if i == 11 || i == 12 {
            pad.gain = 0.55;
        }
    }
    let mut bass = vec![Pad::default(); PADS];
    let mut keys = vec![Pad::default(); PADS];
    for d in 0..PADS {
        bass[pad_of(d)] = melodic(mood, mood.bass[0], BASS_BASE, d);
        keys[pad_of(d)] = melodic(mood, mood.keys[0], KEYS_BASE, d);
    }
    let mut lead = vec![Pad::default(); PADS];
    for d in 0..PADS - FX_PADS {
        lead[pad_of(d)] = melodic(mood, mood.lead[0], LEAD_BASE, d);
    }
    for (i, pad) in lead.iter_mut().enumerate().take(FX_PADS) {
        *pad = fx(i, mood);
    }
    [drums, bass, keys, lead]
}

pub fn apply(project: &mut Project, index: usize) {
    let [a, b, c, d] = sounds(index);
    for (g, pads) in [a, b, c, d].into_iter().enumerate() {
        project.groups[g].pads = pads;
    }
    project.mood = MOODS[index % MOODS.len()].name.into();
}

fn velocity(c: char) -> Option<f32> {
    match c {
        'X' => Some(1.0),
        'x' => Some(0.8),
        'o' => Some(0.45),
        _ => None,
    }
}

pub fn beat(project: &mut Project, index: usize, pattern: usize) {
    let mood = &MOODS[index % MOODS.len()];
    project.tempo = mood.tempo;
    project.swing = mood.swing;
    for g in [DRUMS, BASS] {
        project.groups[g].patterns[pattern] = Pattern::default();
    }
    let drums = &mut project.groups[DRUMS].patterns[pattern];
    for (pad, row) in mood.beat {
        for (s, c) in row.chars().enumerate().take(16) {
            if let Some(v) = velocity(c) {
                *drums.step_mut(*pad, s) = Step {
                    on: true,
                    velocity: v,
                    ..Step::default()
                };
            }
        }
    }
    let bass = &mut project.groups[BASS].patterns[pattern];
    for (s, c) in mood.line.chars().enumerate().take(16) {
        if let Some(d) = c.to_digit(16) {
            *bass.step_mut(pad_of(d as usize), s) = Step {
                on: true,
                velocity: 0.8,
                ..Step::default()
            };
        }
    }
}

fn wobble(rng: &mut Rng, amount: f32) -> f32 {
    1.0 + (rng.unit() * 2.0 - 1.0) * amount
}

pub fn dice(project: &Project, group: usize, pad: usize, seed: u32) -> Option<Pad> {
    let index = find(&project.mood)?;
    let mood = &MOODS[index];
    let mut rng = Rng::new(seed);
    let current = &project.groups[group].pads[pad];
    let mut p = match group {
        DRUMS => {
            let hit = (mood.kit)()[pad];
            let mut p = hit_pad(&hit, mood.tone);
            p.choke = current.choke;
            p.gain = current.gain;
            p
        }
        LEAD if pad < FX_PADS => fx(pad, mood),
        _ => {
            let list = match group {
                BASS => mood.bass,
                KEYS => mood.keys,
                _ => mood.lead,
            };
            let base = [BASS_BASE, BASS_BASE, KEYS_BASE, LEAD_BASE][group];
            let degree = (0..PADS).find(|d| pad_of(*d) == pad).unwrap_or(0);
            let pick = (rng.bits() as usize) % list.len();
            melodic(mood, list[pick], base, degree)
        }
    };
    p.decay = (p.decay * wobble(&mut rng, 0.35)).clamp(0.01, 6.0);
    p.cutoff = (p.cutoff * wobble(&mut rng, 0.12)).clamp(0.05, 1.0);
    if group == DRUMS {
        p.pitch += (wobble(&mut rng, 1.0) - 1.0) * 3.0;
        p.synth.sweep *= wobble(&mut rng, 0.4);
        p.synth.mix = (p.synth.mix * wobble(&mut rng, 0.3)).clamp(0.0, 1.0);
        p.release = p.decay;
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Bounce, bounce};

    mod when_loading_a_mood {
        use super::*;

        #[test]
        fn fills_every_pad_of_every_group() {
            for (i, m) in MOODS.iter().enumerate() {
                let mut project = Project::default();
                apply(&mut project, i);
                for g in &project.groups {
                    assert!(g.pads.iter().all(|p| !p.silent()), "{}", m.name);
                    assert!(g.pads.iter().all(|p| !p.name.is_empty()));
                }
                assert_eq!(project.mood, m.name);
            }
        }

        #[test]
        fn plays_its_notes_in_its_scale() {
            let [_, bass, _, _] = sounds(find("CHIP").unwrap());
            let low = &bass[pad_of(0)];
            let next = &bass[pad_of(1)];
            assert_eq!(next.pitch - low.pitch, 2.0);
            assert!(low.name.ends_with(" c"));
        }
    }

    mod when_writing_a_beat {
        use super::*;

        #[test]
        fn makes_sound_for_every_mood() {
            for (i, m) in MOODS.iter().enumerate() {
                let mut project = Project::default();
                apply(&mut project, i);
                beat(&mut project, i, 0);
                assert!(!project.groups[DRUMS].patterns[0].is_empty());
                assert!(!project.groups[BASS].patterns[0].is_empty());
                assert_eq!(project.tempo, m.tempo);
                let out = bounce(&project, 48000, Bounce::Pattern, 0.0);
                let peak = crate::sample::peak(&out);
                assert!(peak > 0.05 && peak <= 1.0, "{} peak {peak}", m.name);
            }
        }
    }

    mod when_rolling_the_dice {
        use super::*;

        #[test]
        fn changes_the_sound_but_keeps_the_note() {
            let mut project = Project::default();
            let i = find("FUNK").unwrap();
            apply(&mut project, i);
            let before = project.groups[BASS].pads[5].clone();
            let after = (1..20)
                .filter_map(|s| dice(&project, BASS, 5, s))
                .find(|p| p.decay != before.decay)
                .unwrap();
            assert_eq!(after.pitch, before.pitch);
        }

        #[test]
        fn needs_a_mood() {
            assert!(dice(&Project::default(), 0, 0, 1).is_none());
        }
    }
}
