use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use flip_engine::{Command, Project, Runner, Status, Trash};
use rtrb::{Consumer, Producer, RingBuffer};

const SCRATCH: usize = 8192;
const INPUT_SECONDS: usize = 4;

#[derive(Clone, Default)]
pub struct Sender(Arc<Mutex<Option<Producer<Command>>>>);

impl Sender {
    pub fn send(&self, command: Command) {
        if let Ok(mut slot) = self.0.lock()
            && let Some(p) = slot.as_mut()
        {
            let _ = p.push(command);
        }
    }

    fn replace(&self, producer: Producer<Command>) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(producer);
        }
    }
}

pub struct Output {
    pub rate: u32,
    pub device: String,
    pub status: Arc<Status>,
    pub trash: Consumer<Trash>,
    pub tap: Consumer<[f32; 2]>,
    _stream: cpal::Stream,
}

pub struct Input {
    pub rate: u32,
    pub device: String,
    pub frames: Consumer<[f32; 2]>,
    _stream: cpal::Stream,
}

impl Input {
    pub fn drain(&mut self) {
        while self.frames.pop().is_ok() {}
    }
}

fn name(device: &cpal::Device) -> String {
    device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "unknown".into())
}

pub fn names(input: bool) -> Vec<String> {
    let host = cpal::default_host();
    let devices = if input {
        host.input_devices().map(|d| d.collect::<Vec<_>>())
    } else {
        host.output_devices().map(|d| d.collect::<Vec<_>>())
    };
    let mut out: Vec<String> = devices.unwrap_or_default().iter().map(name).collect();
    out.dedup();
    out
}

fn find(wanted: Option<&str>, input: bool) -> Option<cpal::Device> {
    let host = cpal::default_host();
    if let Some(w) = wanted {
        let devices = if input {
            host.input_devices().ok().map(|d| d.collect::<Vec<_>>())
        } else {
            host.output_devices().ok().map(|d| d.collect::<Vec<_>>())
        };
        if let Some(d) = devices
            .unwrap_or_default()
            .into_iter()
            .find(|d| name(d) == w)
        {
            return Some(d);
        }
    }
    if input {
        host.default_input_device()
    } else {
        host.default_output_device()
    }
}

pub fn open_output(
    project: Project,
    wanted: Option<&str>,
    sender: &Sender,
) -> Result<Output, String> {
    let device = find(wanted, false).ok_or("no output device")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let config = supported.config();
    let rate = config.sample_rate;
    let channels = config.channels as usize;
    let (runner, remote) = flip_engine::live(project, rate);
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => build_output::<f32>(&device, config, runner, channels),
        cpal::SampleFormat::I16 => build_output::<i16>(&device, config, runner, channels),
        cpal::SampleFormat::I32 => build_output::<i32>(&device, config, runner, channels),
        cpal::SampleFormat::U16 => build_output::<u16>(&device, config, runner, channels),
        other => return Err(format!("unsupported sample format {other:?}")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    sender.replace(remote.commands);
    Ok(Output {
        rate,
        device: name(&device),
        status: remote.status,
        trash: remote.trash,
        tap: remote.tap,
        _stream: stream,
    })
}

fn build_output<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut runner: Runner,
    channels: usize,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let mut scratch = vec![0.0f32; SCRATCH * channels];
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                for chunk in data.chunks_mut(scratch.len()) {
                    let buf = &mut scratch[..chunk.len()];
                    runner.process(buf, channels);
                    for (d, s) in chunk.iter_mut().zip(buf.iter()) {
                        *d = T::from_sample(*s);
                    }
                }
            },
            |e| eprintln!("output error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

pub fn open_input(wanted: Option<&str>) -> Result<Input, String> {
    let device = find(wanted, true).ok_or("no input device")?;
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let config = supported.config();
    let rate = config.sample_rate;
    let channels = config.channels as usize;
    let (producer, consumer) = RingBuffer::new(rate as usize * INPUT_SECONDS);
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => build_input::<f32>(&device, config, producer, channels),
        cpal::SampleFormat::I16 => build_input::<i16>(&device, config, producer, channels),
        cpal::SampleFormat::I32 => build_input::<i32>(&device, config, producer, channels),
        cpal::SampleFormat::U16 => build_input::<u16>(&device, config, producer, channels),
        other => return Err(format!("unsupported sample format {other:?}")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(Input {
        rate,
        device: name(&device),
        frames: consumer,
        _stream: stream,
    })
}

fn build_input<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut producer: Producer<[f32; 2]>,
    channels: usize,
) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = channels.max(1);
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                for frame in data.chunks(channels) {
                    let l: f32 = frame[0].to_sample();
                    let r: f32 = if channels > 1 {
                        frame[1].to_sample()
                    } else {
                        l
                    };
                    let _ = producer.push([l, r]);
                }
            },
            |e| eprintln!("input error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}
