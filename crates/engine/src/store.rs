use std::path::Path;
use std::sync::Arc;

use crate::project::Project;
use crate::sample::{self, Sample};

const FILE: &str = "project.json";
const SAMPLES: &str = "samples";

pub fn save(project: &Project, dir: &Path) -> Result<(), String> {
    let samples = dir.join(SAMPLES);
    std::fs::create_dir_all(&samples).map_err(|e| e.to_string())?;
    let mut out = project.clone();
    for pad in out.groups.iter_mut().flat_map(|g| &mut g.pads) {
        pad.sample_id = pad.sample.as_ref().map(|s| s.id.clone());
    }
    for s in project.samples() {
        let path = samples.join(format!("{}.wav", s.id));
        if !path.exists() {
            sample::save(&path, s.rate, &s.frames)?;
        }
    }
    let names: Vec<(String, String)> = project
        .samples()
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();
    let json = serde_json::json!({ "project": out, "names": names });
    let text = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(FILE), text).map_err(|e| e.to_string())
}

pub fn load(dir: &Path) -> Result<Project, String> {
    let text = std::fs::read_to_string(dir.join(FILE)).map_err(|e| e.to_string())?;
    let mut json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let names: Vec<(String, String)> =
        serde_json::from_value(json["names"].take()).unwrap_or_default();
    let mut project: Project =
        serde_json::from_value(json["project"].take()).map_err(|e| e.to_string())?;
    project.fix();
    let mut cache: Vec<Arc<Sample>> = Vec::new();
    for pad in project.groups.iter_mut().flat_map(|g| &mut g.pads) {
        let Some(id) = pad.sample_id.clone() else {
            continue;
        };
        if let Some(s) = cache.iter().find(|s| s.id == id) {
            pad.sample = Some(s.clone());
            continue;
        }
        let path = dir.join(SAMPLES).join(format!("{id}.wav"));
        match sample::load(&path) {
            Ok(loaded) => {
                let name = names
                    .iter()
                    .find(|(i, _)| *i == id)
                    .map(|(_, n)| n.clone())
                    .unwrap_or(loaded.name.clone());
                let s = Arc::new(Sample::with_id(id, name, loaded.rate, loaded.frames));
                cache.push(s.clone());
                pad.sample = Some(s);
            }
            Err(_) => pad.sample_id = None,
        }
    }
    Ok(project)
}

pub fn is_project(dir: &Path) -> bool {
    dir.join(FILE).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod when_saving_a_project {
        use super::*;

        #[test]
        fn loads_back_with_shared_samples() {
            let dir = std::env::temp_dir().join(format!("flip-store-{}", sample::fresh_id()));
            let mut project = Project::starter();
            let s = Arc::new(Sample::new("vox", 44100, vec![[0.25, -0.25]; 500]));
            project.groups[2].pads[0].sample = Some(s.clone());
            project.groups[2].pads[1].sample = Some(s);
            project.groups[2].pads[1].start = 0.5;
            project.tempo = 120.0;
            save(&project, &dir).unwrap();
            let back = load(&dir).unwrap();
            std::fs::remove_dir_all(&dir).ok();
            let a = back.groups[2].pads[0].sample.clone().unwrap();
            let b = back.groups[2].pads[1].sample.clone().unwrap();
            assert!(Arc::ptr_eq(&a, &b));
            assert_eq!(a.name, "vox");
            assert_eq!(a.len(), 500);
            assert_eq!(back.groups[2].pads[1].start, 0.5);
            assert_eq!(back.tempo, 120.0);
            assert_eq!(back.groups[0].patterns[0], project.groups[0].patterns[0]);
        }
    }
}
