use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::project::{GROUPS, Mode, PADS, Pad, Source};
use crate::sample::{self, Sample};

pub const KIT_PADS: usize = 12;
pub const CATEGORIES: [&str; 6] = ["KICK", "SNARE", "CYMBAL", "PERC", "BASS", "MELODIC"];

const KITS: &str = "kits.json";
const SOUNDS: &str = "sounds";
const RECORD: usize = 26;
const BLOCK: usize = 512;
const UNSET: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct KitPad {
    pub sound: u16,
    pub start: u32,
    pub end: u32,
    pub volume: u8,
    pub pitch: i8,
    pub pan: i8,
    pub release: u8,
    pub choke: u8,
    pub mode: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Kit {
    pub name: String,
    pub tempo: Option<f32>,
    pub groups: Vec<Vec<Option<KitPad>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
    pub number: u16,
    pub name: String,
    pub path: PathBuf,
}

impl Sound {
    pub fn category(&self) -> usize {
        (self.number as usize / 100).min(CATEGORIES.len() - 1)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Factory {
    pub sounds: Vec<Sound>,
    pub kits: Vec<Kit>,
}

pub fn parse_pad(bytes: &[u8]) -> Option<KitPad> {
    if bytes.len() < RECORD {
        return None;
    }
    let sound = u16::from_le_bytes([bytes[1], bytes[2]]);
    if sound == 0 {
        return None;
    }
    let word =
        |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    Some(KitPad {
        sound,
        start: word(4),
        end: word(8),
        volume: bytes[16],
        pitch: bytes[17] as i8,
        pan: bytes[18] as i8,
        release: bytes[20],
        choke: bytes[22],
        mode: bytes[23],
    })
}

fn parse_tempo(bytes: &[u8]) -> Option<f32> {
    let tempo = f32::from_le_bytes(bytes.get(12..16)?.try_into().ok()?);
    (40.0..=300.0).contains(&tempo).then_some(tempo)
}

fn octal(field: &[u8]) -> usize {
    field
        .iter()
        .take_while(|&&b| b != 0 && b != b' ')
        .fold(0, |n, &b| n * 8 + (b.wrapping_sub(b'0') & 7) as usize)
}

pub fn parse_tar(data: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let mut at = 0;
    while at + BLOCK <= data.len() {
        let header = &data[at..at + BLOCK];
        if header.iter().all(|&b| b == 0) {
            break;
        }
        let name_end = header[..100].iter().position(|&b| b == 0).unwrap_or(100);
        let name = String::from_utf8_lossy(&header[..name_end]).to_string();
        let size = octal(&header[124..136]);
        let body = at + BLOCK;
        if matches!(header[156], b'0' | 0) && body + size <= data.len() {
            files.push((name, data[body..body + size].to_vec()));
        }
        at = body + size.div_ceil(BLOCK) * BLOCK;
    }
    files
}

pub fn parse_kit(name: &str, tar: &[u8]) -> Option<Kit> {
    let mut groups = vec![vec![None; KIT_PADS]; GROUPS];
    let mut tempo = None;
    for (path, bytes) in parse_tar(tar) {
        let parts: Vec<&str> = path.trim_start_matches("./").split('/').collect();
        let [_, group, pad] = parts.as_slice() else {
            continue;
        };
        if parts[0] != "pads" {
            continue;
        }
        let g = group.bytes().next().map(|b| b.wrapping_sub(b'a') as usize);
        let p = pad.strip_prefix('p').and_then(|n| n.parse::<usize>().ok());
        let (Some(g), Some(p)) = (g, p) else {
            continue;
        };
        if g >= GROUPS || p == 0 || p > KIT_PADS {
            continue;
        }
        tempo = tempo.or(parse_tempo(&bytes));
        groups[g][p - 1] = parse_pad(&bytes);
    }
    groups.iter().flatten().any(Option::is_some).then(|| Kit {
        name: name.into(),
        tempo,
        groups,
    })
}

pub fn flip_pad(index: usize) -> usize {
    (index / 3) * 4 + index % 3
}

fn sound_number(file: &str) -> Option<(u16, String)> {
    let stem = file.strip_suffix(".wav")?;
    let (number, name) = stem.split_once(' ')?;
    Some((number.parse().ok()?, name.to_string()))
}

pub fn install(pak: &Path, dir: &Path) -> Result<Factory, String> {
    let file = std::fs::File::open(pak).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let sounds = dir.join(SOUNDS);
    std::fs::create_dir_all(&sounds).map_err(|e| e.to_string())?;
    let mut kits = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if !entry.is_file() {
            continue;
        }
        let name = entry
            .name()
            .map_err(|e| e.to_string())?
            .trim_start_matches('/')
            .to_string();
        let Some((folder, file)) = name.split_once('/') else {
            continue;
        };
        if file.contains('/') || file.contains("..") || file.starts_with('.') {
            continue;
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
        match folder {
            "sounds" if sound_number(file).is_some() => {
                std::fs::write(sounds.join(file), data).map_err(|e| e.to_string())?;
            }
            "projects" => {
                if let Some(kit) = file.strip_suffix(".tar").and_then(|n| parse_kit(n, &data)) {
                    kits.push(kit);
                }
            }
            _ => {}
        }
    }
    if kits.is_empty() {
        return Err("no kits in this pak".into());
    }
    kits.sort_by(|a, b| a.name.cmp(&b.name));
    let text = serde_json::to_string(&kits).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(KITS), text).map_err(|e| e.to_string())?;
    open(dir).ok_or_else(|| "install failed".into())
}

pub fn open(dir: &Path) -> Option<Factory> {
    let text = std::fs::read_to_string(dir.join(KITS)).ok()?;
    let kits: Vec<Kit> = serde_json::from_str(&text).ok()?;
    let mut sounds: Vec<Sound> = std::fs::read_dir(dir.join(SOUNDS))
        .ok()?
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let (number, name) = sound_number(path.file_name()?.to_str()?)?;
            Some(Sound { number, name, path })
        })
        .collect();
    sounds.sort_by_key(|s| s.number);
    Some(Factory { sounds, kits })
}

pub fn find_pak(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "pak")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.contains("factory"))
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

pub fn pad(kp: &KitPad, name: &str, sample: Arc<Sample>) -> Pad {
    let len = sample.len().max(1) as f32;
    let start = if kp.start == UNSET {
        0.0
    } else {
        (kp.start as f32 / len).clamp(0.0, 1.0)
    };
    let end = if kp.end == UNSET || kp.end == 0 {
        1.0
    } else {
        (kp.end as f32 / len).clamp(start, 1.0)
    };
    let mode = if kp.mode == 0 {
        Mode::OneShot
    } else {
        Mode::Gate
    };
    let release = 0.02 + (kp.release as f32 / 255.0).powi(2) * 3.0;
    Pad {
        sample: Some(sample),
        name: name.into(),
        source: Source::Sample,
        start,
        end,
        pitch: kp.pitch as f32,
        gain: (kp.volume as f32 / 100.0 * 0.8).clamp(0.0, 1.0),
        pan: (kp.pan as f32 / 16.0).clamp(-1.0, 1.0),
        release,
        mode,
        choke: kp.choke,
        ..Pad::default()
    }
}

impl Factory {
    pub fn sound(&self, number: u16) -> Option<&Sound> {
        self.sounds.iter().find(|s| s.number == number)
    }

    pub fn in_category(&self, category: usize) -> Vec<&Sound> {
        self.sounds
            .iter()
            .filter(|s| s.category() == category)
            .collect()
    }

    pub fn load(&self, number: u16) -> Result<Sample, String> {
        let sound = self.sound(number).ok_or("sound not installed")?;
        let mut s = sample::load(&sound.path)?;
        s.name = sound.name.clone();
        Ok(s)
    }

    pub fn kit_pads(&self, kit: usize) -> Result<[Vec<Pad>; GROUPS], String> {
        let kit = self.kits.get(kit).ok_or("no such kit")?;
        let mut cache: Vec<(u16, Arc<Sample>)> = Vec::new();
        let mut out: [Vec<Pad>; GROUPS] = std::array::from_fn(|_| vec![Pad::default(); PADS]);
        for (g, pads) in kit.groups.iter().enumerate().take(GROUPS) {
            for (i, kp) in pads.iter().enumerate().take(KIT_PADS) {
                let Some(kp) = kp else {
                    continue;
                };
                let sample = match cache.iter().find(|(n, _)| *n == kp.sound) {
                    Some((_, s)) => s.clone(),
                    None => {
                        let s = Arc::new(self.load(kp.sound)?);
                        cache.push((kp.sound, s.clone()));
                        s
                    }
                };
                let name = sample.name.clone();
                out[g][flip_pad(i)] = pad(kp, &name, sample);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(sound: u16, end: u32, choke: u8, mode: u8) -> Vec<u8> {
        let mut b = vec![0u8; RECORD];
        b[1..3].copy_from_slice(&sound.to_le_bytes());
        b[8..12].copy_from_slice(&end.to_le_bytes());
        b[16] = 100;
        b[20] = 0xff;
        b[22] = choke;
        b[23] = mode;
        b[24] = 60;
        b
    }

    fn tar(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, body) in files {
            let mut header = vec![0u8; BLOCK];
            header[..name.len()].copy_from_slice(name.as_bytes());
            let size = format!("{:011o}\0", body.len());
            header[124..136].copy_from_slice(size.as_bytes());
            header[156] = b'0';
            out.extend(header);
            out.extend(*body);
            out.resize(out.len().div_ceil(BLOCK) * BLOCK, 0);
        }
        out.extend(vec![0u8; BLOCK * 2]);
        out
    }

    mod when_parsing_a_pad_record {
        use super::*;

        #[test]
        fn reads_sound_trim_choke_and_mode() {
            let pad = parse_pad(&record(343, 26897, 1, 2)).unwrap();
            assert_eq!(pad.sound, 343);
            assert_eq!(pad.end, 26897);
            assert_eq!(pad.volume, 100);
            assert_eq!(pad.choke, 1);
            assert_eq!(pad.mode, 2);
        }

        mod when_the_pad_is_empty {
            use super::*;

            #[test]
            fn returns_none() {
                assert!(parse_pad(&record(0, 0, 0, 0)).is_none());
                assert!(parse_pad(&[0u8; 4]).is_none());
            }
        }
    }

    mod when_parsing_a_project_tar {
        use super::*;

        #[test]
        fn maps_pads_into_groups() {
            let a = record(1, UNSET, 0, 0);
            let c = record(545, UNSET, 0, 1);
            let data = tar(&[("pads/a/p10", &a), ("pads/c/p01", &c), ("patterns/", &[])]);
            let kit = parse_kit("P01", &data).unwrap();
            assert_eq!(kit.groups[0][9].unwrap().sound, 1);
            assert_eq!(kit.groups[2][0].unwrap().sound, 545);
            assert!(kit.groups[1].iter().all(Option::is_none));
        }

        mod when_no_pad_has_a_sound {
            use super::*;

            #[test]
            fn returns_no_kit() {
                let empty = record(0, 0, 0, 0);
                assert!(parse_kit("P06", &tar(&[("pads/a/p01", &empty)])).is_none());
            }
        }
    }

    mod when_placing_kit_pads {
        use super::*;

        #[test]
        fn keeps_the_three_column_layout() {
            assert_eq!(flip_pad(0), 0);
            assert_eq!(flip_pad(2), 2);
            assert_eq!(flip_pad(3), 4);
            assert_eq!(flip_pad(11), 14);
        }

        #[test]
        fn converts_trim_volume_and_mode() {
            let kp = parse_pad(&record(200, 50, 1, 1)).unwrap();
            let s = Arc::new(Sample::new("hat", 46875, vec![[0.1, 0.1]; 100]));
            let p = pad(&kp, "hat", s);
            assert_eq!(p.end, 0.5);
            assert_eq!(p.start, 0.0);
            assert_eq!(p.gain, 0.8);
            assert_eq!(p.choke, 1);
            assert_eq!(p.mode, Mode::Gate);
        }
    }

    mod when_reading_sound_file_names {
        use super::*;

        #[test]
        fn splits_number_and_name() {
            assert_eq!(
                sound_number("001 micro kick.wav"),
                Some((1, "micro kick".into()))
            );
            assert_eq!(sound_number("readme.txt"), None);
        }
    }
}
