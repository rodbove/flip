use crate::project::PADS;
use crate::sample::Sample;

const HOP: f32 = 0.01;
const GAP: f32 = 0.08;
const RISE: f32 = 2.0;
const FLOOR: f32 = 0.02;

pub fn even(count: usize) -> Vec<(f32, f32)> {
    let n = count.max(1) as f32;
    (0..count.max(1))
        .map(|i| (i as f32 / n, (i + 1) as f32 / n))
        .collect()
}

pub fn onsets(sample: &Sample) -> Vec<usize> {
    let hop = ((sample.rate as f32 * HOP) as usize).max(1);
    let energy: Vec<f32> = sample
        .frames
        .chunks(hop)
        .map(|c| {
            c.iter()
                .map(|f| (f[0] * f[0] + f[1] * f[1]) * 0.5)
                .sum::<f32>()
                / c.len() as f32
        })
        .map(f32::sqrt)
        .collect();
    let loudest = energy.iter().copied().fold(0.0, f32::max);
    if loudest <= 0.0 {
        return Vec::new();
    }
    let gap = ((GAP / HOP) as usize).max(1);
    let mut picks: Vec<(usize, f32)> = Vec::new();
    for i in 1..energy.len() {
        let before = energy[i.saturating_sub(3)..i]
            .iter()
            .copied()
            .fold(0.0, f32::max);
        let now = energy[i];
        if now < loudest * FLOOR || now < before * RISE {
            continue;
        }
        let strength = now - before;
        match picks.last_mut() {
            Some(last) if i - last.0 < gap => {
                if strength > last.1 {
                    *last = (i, strength);
                }
            }
            _ => picks.push((i, strength)),
        }
    }
    if energy[0] >= loudest * FLOOR && picks.first().is_none_or(|p| p.0 >= gap) {
        picks.insert(0, (0, f32::MAX));
    }
    picks.sort_by(|a, b| b.1.total_cmp(&a.1));
    picks.truncate(PADS);
    let mut out: Vec<usize> = picks.into_iter().map(|p| p.0 * hop).collect();
    out.sort_unstable();
    out
}

pub fn slices(sample: &Sample) -> Vec<(f32, f32)> {
    let starts = onsets(sample);
    if starts.len() < 2 {
        return even(PADS);
    }
    let n = sample.len() as f32;
    starts
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let end = starts.get(i + 1).copied().unwrap_or(sample.len());
            (s as f32 / n, end as f32 / n)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clicks(at: &[f32], seconds: f32) -> Sample {
        let rate = 48000;
        let mut frames = vec![[0.0f32; 2]; (seconds * rate as f32) as usize];
        for &t in at {
            let start = (t * rate as f32) as usize;
            for k in 0..2400 {
                let v = (k as f32 * 0.3).sin() * (1.0 - k as f32 / 2400.0);
                frames[start + k] = [v, v];
            }
        }
        Sample::new("clicks", rate, frames)
    }

    mod when_chopping_hits {
        use super::*;

        #[test]
        fn finds_each_hit() {
            let s = clicks(&[0.1, 0.5, 0.9, 1.4], 2.0);
            let found: Vec<f32> = onsets(&s).iter().map(|&i| i as f32 / 48000.0).collect();
            assert_eq!(found.len(), 4, "{found:?}");
            for (f, t) in found.iter().zip([0.1, 0.5, 0.9, 1.4]) {
                assert!((f - t).abs() < 0.02, "{f} vs {t}");
            }
        }

        #[test]
        fn makes_slices_that_meet() {
            let s = clicks(&[0.1, 0.5, 0.9], 1.5);
            let cuts = slices(&s);
            assert_eq!(cuts.len(), 3);
            assert_eq!(cuts[0].1, cuts[1].0);
            assert_eq!(cuts[2].1, 1.0);
        }
    }

    mod when_nothing_stands_out {
        use super::*;

        #[test]
        fn cuts_even_slices() {
            let s = Sample::new("hum", 48000, vec![[0.5, 0.5]; 48000]);
            let cuts = slices(&s);
            assert_eq!(cuts.len(), PADS);
            assert_eq!(cuts[0], (0.0, 1.0 / 16.0));
        }
    }
}
