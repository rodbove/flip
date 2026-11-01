use std::path::{Path, PathBuf};

use flip_engine::preset::{self, CATEGORIES, Preset, USER};

use crate::settings;

pub fn dir() -> PathBuf {
    settings::projects().join("presets")
}

pub struct Library {
    pub items: Vec<Preset>,
    pub category: usize,
    pub cursor: usize,
}

impl Library {
    pub fn load() -> Self {
        Self::with_user(read_dir(&dir()))
    }

    fn with_user(user: Vec<Preset>) -> Self {
        let mut items = preset::factory();
        items.extend(user);
        Self {
            items,
            category: 0,
            cursor: 0,
        }
    }

    pub fn shown(&self) -> Vec<usize> {
        let name = CATEGORIES[self.category.min(CATEGORIES.len() - 1)];
        self.items
            .iter()
            .enumerate()
            .filter(|(_, p)| p.category == name)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn selected(&self) -> Option<usize> {
        self.shown().get(self.cursor).copied()
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let count = self.shown().len();
        if count == 0 {
            self.cursor = 0;
            return;
        }
        self.cursor = (self.cursor as i32 + delta).rem_euclid(count as i32) as usize;
    }

    pub fn move_category(&mut self, delta: i32) {
        let count = CATEGORIES.len() as i32;
        self.category = (self.category as i32 + delta).rem_euclid(count) as usize;
        self.cursor = 0;
    }

    pub fn add(&mut self, preset: Preset) {
        match self
            .items
            .iter_mut()
            .find(|p| p.category == USER && p.name == preset.name)
        {
            Some(existing) => *existing = preset,
            None => self.items.push(preset),
        }
    }
}

pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "preset".into() } else { out }
}

pub fn save(preset: &Preset) -> Result<PathBuf, String> {
    save_in(&dir(), preset)
}

fn save_in(dir: &Path, preset: &Preset) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", slug(&preset.name)));
    let text = serde_json::to_string_pretty(preset).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

fn read_dir(dir: &Path) -> Vec<Preset> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<Preset> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .filter_map(|t| serde_json::from_str::<Preset>(&t).ok())
        .map(|mut p| {
            p.category = USER.into();
            p
        })
        .collect();
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use flip_engine::{Pad, Source};

    mod when_naming_a_file {
        use super::*;

        #[test]
        fn keeps_letters_and_digits() {
            assert_eq!(slug("My E-Piano 2!"), "my-e-piano-2");
            assert_eq!(slug("  ??  "), "preset");
        }
    }

    mod when_saving_a_user_preset {
        use super::*;

        #[test]
        fn loads_back_in_the_user_category() {
            let dir = std::env::temp_dir()
                .join(format!("flip-presets-{}", flip_engine::sample::fresh_id()));
            let pad = Pad {
                source: Source::String,
                ..Pad::default()
            };
            save_in(&dir, &Preset::capture("Twang", &pad)).unwrap();
            let user = read_dir(&dir);
            std::fs::remove_dir_all(&dir).ok();
            assert_eq!(user.len(), 1);
            let mut library = Library::with_user(user);
            library.category = CATEGORIES.iter().position(|c| *c == USER).unwrap();
            let i = library.selected().unwrap();
            assert_eq!(library.items[i].name, "Twang");
            assert_eq!(library.items[i].sound.source, Source::String);
        }
    }

    mod when_browsing {
        use super::*;

        #[test]
        fn wraps_around_the_category() {
            let mut library = Library::with_user(Vec::new());
            let count = library.shown().len();
            library.move_cursor(-1);
            assert_eq!(library.cursor, count - 1);
            library.move_cursor(1);
            assert_eq!(library.cursor, 0);
            library.move_category(-1);
            assert_eq!(CATEGORIES[library.category], USER);
            assert!(library.selected().is_none());
        }
    }
}
