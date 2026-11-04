use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::params::Param;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub output: Option<String>,
    pub input: Option<String>,
    pub midi: Option<String>,
    pub learned: Vec<(u8, Param)>,
    pub labels: Vec<(String, String)>,
    pub last: Option<PathBuf>,
    pub threshold: f32,
    pub quantize: usize,
    pub pad_keys: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output: None,
            input: None,
            midi: None,
            learned: Vec::new(),
            labels: Vec::new(),
            last: None,
            threshold: 0.05,
            quantize: 2,
            pad_keys: false,
        }
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}

fn dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        home().join("Library/Application Support/Flip")
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".config"))
            .join("flip")
    }
}

fn music() -> PathBuf {
    if !cfg!(target_os = "macos") {
        let file = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".config"))
            .join("user-dirs.dirs");
        if let Some(dir) = std::fs::read_to_string(file)
            .ok()
            .and_then(|t| xdg_music(&t, &home()))
        {
            return dir;
        }
    }
    home().join("Music")
}

fn xdg_music(text: &str, home: &std::path::Path) -> Option<PathBuf> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("XDG_MUSIC_DIR="))?;
    let value = line["XDG_MUSIC_DIR=".len()..].trim_matches('"');
    let path = match value.strip_prefix("$HOME") {
        Some(rest) => home.join(rest.trim_start_matches('/')),
        None => PathBuf::from(value),
    };
    (path.is_absolute() && path != home).then_some(path)
}

pub fn projects() -> PathBuf {
    music().join("Flip")
}

pub fn factory() -> PathBuf {
    projects().join("Factory")
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(dir().join("settings.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let d = dir();
        if std::fs::create_dir_all(&d).is_ok()
            && let Ok(text) = serde_json::to_string_pretty(self)
        {
            let _ = std::fs::write(d.join("settings.json"), text);
        }
    }

    pub fn learned(&self, cc: u8) -> Option<Param> {
        self.learned.iter().find(|(c, _)| *c == cc).map(|(_, p)| *p)
    }

    pub fn learn(&mut self, cc: u8, param: Param) {
        self.learned.retain(|(c, p)| *c != cc && *p != param);
        self.learned.push((cc, param));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod when_reading_the_music_folder {
        use super::*;

        fn home() -> PathBuf {
            PathBuf::from("/home/me")
        }

        mod with_a_home_relative_path {
            use super::*;

            #[test]
            fn joins_it_to_home() {
                let text = "# comment\nXDG_MUSIC_DIR=\"$HOME/Musik\"\n";
                assert_eq!(xdg_music(text, &home()), Some(home().join("Musik")));
            }
        }

        mod with_an_absolute_path {
            use super::*;

            #[test]
            fn uses_it() {
                let text = "XDG_MUSIC_DIR=\"/data/music\"";
                assert_eq!(xdg_music(text, &home()), Some(PathBuf::from("/data/music")));
            }
        }

        mod with_the_folder_turned_off {
            use super::*;

            #[test]
            fn falls_back() {
                assert_eq!(xdg_music("XDG_MUSIC_DIR=\"$HOME/\"", &home()), None);
                assert_eq!(
                    xdg_music("XDG_DESKTOP_DIR=\"$HOME/Desktop\"", &home()),
                    None
                );
            }
        }
    }
}
