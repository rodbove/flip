use crate::dsp::Rng;
use crate::project::Pluck;

pub const MIN_HZ: f32 = 20.0;
const BOW_GAIN: f32 = 0.04;

pub fn buffer_len(rate: f32) -> usize {
    (rate / MIN_HZ) as usize + 8
}

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    delay: f32,
    write: usize,
    last: f32,
    loss: f32,
    gain: f32,
}

pub fn start(p: &Pluck, buf: &mut [f32], hz: f32, rate: f32, rng: &mut Rng) -> State {
    let size = buf.len();
    let loss = 0.5 * (1.0 - 0.8 * p.brightness.clamp(0.0, 1.0));
    let delay = (rate / hz.max(MIN_HZ) - loss).clamp(2.0, (size - 4) as f32);
    let n = delay.ceil() as usize;
    buf.fill(0.0);
    let smooth = 0.05 + 0.95 * p.brightness.clamp(0.0, 1.0);
    let mut low = 0.0;
    for v in buf.iter_mut().take(n) {
        low += (rng.noise() - low) * smooth;
        *v = low;
    }
    let pick = (p.body.clamp(0.0, 1.0) * 0.5 * n as f32) as usize;
    if pick > 0 {
        for i in (pick..n).rev() {
            buf[i] -= buf[i - pick];
        }
    }
    let mean = buf[..n].iter().sum::<f32>() / n as f32;
    for v in &mut buf[..n] {
        *v -= mean;
    }
    super::peak_normalize(&mut buf[..n]);
    let seconds = 0.15 + (1.0 - p.damping.clamp(0.0, 1.0)).powi(2) * 8.0;
    let gain = (-6.9 * (delay / rate) / seconds).exp();
    State {
        delay,
        write: n % size,
        last: 0.0,
        loss,
        gain,
    }
}

pub fn next(p: &Pluck, buf: &mut [f32], s: &mut State, rng: &mut Rng) -> f32 {
    let size = buf.len();
    if size < 4 {
        return 0.0;
    }
    let read = s.write as f32 - s.delay + size as f32;
    let i = read.floor() as usize;
    let frac = read - read.floor();
    let a = buf[i % size];
    let b = buf[(i + 1) % size];
    let y = a + (b - a) * frac;
    let filtered = y * (1.0 - s.loss) + s.last * s.loss;
    s.last = y;
    buf[s.write] = filtered * s.gain + rng.noise() * p.bow.clamp(0.0, 1.0) * BOW_GAIN;
    s.write = (s.write + 1) % size;
    y
}

pub fn preview(p: &Pluck, n: usize) -> Vec<f32> {
    let rate = 8000.0;
    let mut buf = vec![0.0; buffer_len(rate)];
    let mut rng = super::noise_rng();
    let mut s = start(p, &mut buf, 2.0 * rate / n as f32, rate, &mut rng);
    (0..n)
        .map(|_| next(p, &mut buf, &mut s, &mut rng))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48000.0;

    fn energy(p: &Pluck, from: usize, to: usize) -> f32 {
        let mut buf = vec![0.0; buffer_len(RATE)];
        let mut rng = Rng::default();
        let mut s = start(p, &mut buf, 220.0, RATE, &mut rng);
        let out: Vec<f32> = (0..to)
            .map(|_| next(p, &mut buf, &mut s, &mut rng))
            .collect();
        out[from..].iter().map(|v| v * v).sum::<f32>() / (to - from) as f32
    }

    mod when_plucked {
        use super::*;

        #[test]
        fn rings_then_fades() {
            let p = Pluck::default();
            assert!(energy(&p, 0, 4800) > 0.01);
            assert!(energy(&p, 0, 4800) > energy(&p, 96000, 100800) * 10.0);
        }

        #[test]
        fn fades_faster_with_more_damping() {
            let soft = Pluck {
                damping: 0.1,
                ..Pluck::default()
            };
            let hard = Pluck {
                damping: 0.9,
                ..Pluck::default()
            };
            assert!(energy(&hard, 24000, 28800) < energy(&soft, 24000, 28800));
        }

        #[test]
        fn tunes_to_the_note() {
            let p = Pluck::default();
            let mut buf = vec![0.0; buffer_len(RATE)];
            let mut rng = Rng::default();
            let mut s = start(&p, &mut buf, 441.0, RATE, &mut rng);
            let out: Vec<f32> = (0..9600)
                .map(|_| next(&p, &mut buf, &mut s, &mut rng))
                .collect();
            let lag = |l: usize| -> f32 { (4800..9000).map(|i| out[i] * out[i + l]).sum() };
            let best = (90..130)
                .max_by(|a, b| lag(*a).total_cmp(&lag(*b)))
                .unwrap();
            assert!((best as i32 - 109).abs() <= 1, "{best}");
        }
    }

    mod when_bowed {
        use super::*;

        #[test]
        fn keeps_sounding() {
            let bowed = Pluck {
                bow: 1.0,
                damping: 0.6,
                ..Pluck::default()
            };
            let plucked = Pluck {
                damping: 0.6,
                ..Pluck::default()
            };
            assert!(energy(&bowed, 96000, 100800) > energy(&plucked, 96000, 100800));
        }
    }
}
