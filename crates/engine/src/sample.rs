use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const OVERVIEW: usize = 1024;
const HEADROOM: f32 = 0.89;
const QUIET: f32 = 1e-3;

#[derive(Debug)]
pub struct Sample {
    pub id: String,
    pub name: String,
    pub rate: u32,
    pub frames: Vec<[f32; 2]>,
    pub overview: Vec<f32>,
}

pub fn fresh_id() -> String {
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!(
        "{nanos:x}{:02x}",
        COUNT.fetch_add(1, Ordering::Relaxed) % 256
    )
}

impl Sample {
    pub fn new(name: impl Into<String>, rate: u32, frames: Vec<[f32; 2]>) -> Self {
        Self::with_id(fresh_id(), name, rate, frames)
    }

    pub fn with_id(id: String, name: impl Into<String>, rate: u32, frames: Vec<[f32; 2]>) -> Self {
        let overview = overview(&frames);
        Self {
            id,
            name: name.into(),
            rate: rate.max(1),
            frames,
            overview,
        }
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn seconds(&self) -> f32 {
        self.frames.len() as f32 / self.rate as f32
    }

    pub fn read(&self, pos: f64) -> [f32; 2] {
        let n = self.frames.len();
        if n == 0 || pos < 0.0 {
            return [0.0; 2];
        }
        let i = pos.floor() as usize;
        if i >= n {
            return [0.0; 2];
        }
        let t = (pos - i as f64) as f32;
        let at = |k: isize| -> [f32; 2] {
            let k = i as isize + k;
            if k < 0 || k as usize >= n {
                [0.0; 2]
            } else {
                self.frames[k as usize]
            }
        };
        let (a, b, c, d) = (at(-1), at(0), at(1), at(2));
        let mut out = [0.0; 2];
        for ch in 0..2 {
            out[ch] = hermite(a[ch], b[ch], c[ch], d[ch], t);
        }
        out
    }
}

fn hermite(a: f32, b: f32, c: f32, d: f32, t: f32) -> f32 {
    let c0 = b;
    let c1 = 0.5 * (c - a);
    let c2 = a - 2.5 * b + 2.0 * c - 0.5 * d;
    let c3 = 0.5 * (d - a) + 1.5 * (b - c);
    ((c3 * t + c2) * t + c1) * t + c0
}

fn overview(frames: &[[f32; 2]]) -> Vec<f32> {
    if frames.is_empty() {
        return Vec::new();
    }
    let bucket = frames.len().div_ceil(OVERVIEW).max(1);
    frames
        .chunks(bucket)
        .map(|c| {
            c.iter()
                .map(|f| f[0].abs().max(f[1].abs()))
                .fold(0.0, f32::max)
        })
        .collect()
}

pub fn peak(frames: &[[f32; 2]]) -> f32 {
    frames
        .iter()
        .map(|f| f[0].abs().max(f[1].abs()))
        .fold(0.0, f32::max)
}

pub fn normalize(frames: &mut [[f32; 2]]) {
    let p = peak(frames);
    if p > QUIET {
        let g = HEADROOM / p;
        for f in frames {
            f[0] *= g;
            f[1] *= g;
        }
    }
}

pub fn load(path: &Path) -> Result<Sample, String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::errors::Error;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| e.to_string())?;
    let mut format = probed.format;
    let track = format.default_track().ok_or("no audio track")?.clone();
    let rate = track
        .codec_params
        .sample_rate
        .ok_or("unknown sample rate")?;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| e.to_string())?;
    let mut frames = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(Error::ResetRequired) => break,
            Err(e) => return Err(e.to_string()),
        };
        if packet.track_id() != track.id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.to_string()),
        };
        let spec = *decoded.spec();
        let channels = spec.channels.count().max(1);
        let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buf.copy_interleaved_ref(decoded);
        for f in buf.samples().chunks(channels) {
            let l = f[0];
            let r = if channels > 1 { f[1] } else { l };
            frames.push([l, r]);
        }
    }
    if frames.is_empty() {
        return Err("file has no audio".into());
    }
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("sample")
        .to_string();
    Ok(Sample::new(name, rate, frames))
}

pub fn save(path: &Path, rate: u32, frames: &[[f32; 2]]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for f in frames {
        writer.write_sample(f[0]).map_err(|e| e.to_string())?;
        writer.write_sample(f[1]).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod when_reading_between_frames {
        use super::*;

        #[test]
        fn follows_a_straight_line() {
            let frames = (0..8).map(|i| [i as f32, -(i as f32)]).collect();
            let s = Sample::new("ramp", 48000, frames);
            let v = s.read(3.25);
            assert!((v[0] - 3.25).abs() < 1e-4);
            assert!((v[1] + 3.25).abs() < 1e-4);
        }

        #[test]
        fn is_silent_past_the_end() {
            let s = Sample::new("one", 48000, vec![[1.0, 1.0]]);
            assert_eq!(s.read(5.0), [0.0, 0.0]);
            assert_eq!(s.read(-1.0), [0.0, 0.0]);
        }
    }

    mod when_saving_and_loading {
        use super::*;

        #[test]
        fn keeps_the_audio() {
            let dir = std::env::temp_dir().join(format!("flip-test-{}", fresh_id()));
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("tone.wav");
            let frames: Vec<[f32; 2]> = (0..1000)
                .map(|i| [(i as f32 * 0.1).sin(), (i as f32 * 0.2).cos() * 0.5])
                .collect();
            save(&path, 44100, &frames).unwrap();
            let back = load(&path).unwrap();
            std::fs::remove_dir_all(&dir).ok();
            assert_eq!(back.rate, 44100);
            assert_eq!(back.len(), frames.len());
            assert!((back.frames[10][1] - frames[10][1]).abs() < 1e-6);
            assert_eq!(back.name, "tone");
        }
    }

    mod when_normalizing {
        use super::*;

        #[test]
        fn lifts_quiet_audio() {
            let mut frames = vec![[0.1, -0.05]; 10];
            normalize(&mut frames);
            assert!((peak(&frames) - HEADROOM).abs() < 1e-5);
        }

        #[test]
        fn leaves_silence_alone() {
            let mut frames = vec![[0.0, 0.0]; 10];
            normalize(&mut frames);
            assert_eq!(peak(&frames), 0.0);
        }
    }
}
