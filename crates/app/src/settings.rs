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

pub fn projects() -> PathBuf {
    home().join("Music/Flip")
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
