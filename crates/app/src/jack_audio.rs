use flip_engine::{Project, Runner};
use jack::{AudioIn, AudioOut, Client, ClientOptions, Control, PortFlags, ProcessScope};
use rtrb::{Producer, RingBuffer};

use crate::audio::{INPUT_SECONDS, Input, Output, Sender};

const AUDIO: &str = "32 bit float mono audio";
const SCRATCH: usize = 4096;
const BUFFER: u32 = 256;

type Devices = Vec<(String, Vec<String>)>;

pub fn client(name: &str) -> Option<Client> {
    Client::new(name, ClientOptions::NO_START_SERVER)
        .ok()
        .map(|(c, _)| c)
}

fn devices(client: &Client, capture: bool) -> Devices {
    let (flags, prefix) = if capture {
        (PortFlags::IS_OUTPUT, "capture_")
    } else {
        (PortFlags::IS_INPUT, "playback_")
    };
    let mut out: Devices = Vec::new();
    for port in client.ports(None, Some(AUDIO), flags) {
        let Some((device, short)) = port.split_once(':') else {
            continue;
        };
        if !short.starts_with(prefix) {
            continue;
        }
        match out.iter_mut().find(|(d, _)| d == device) {
            Some((_, ports)) => ports.push(port),
            None => out.push((device.to_string(), vec![port])),
        }
    }
    out
}

pub fn names(input: bool) -> Option<Vec<String>> {
    let c = client("flip-scan")?;
    Some(devices(&c, input).into_iter().map(|(n, _)| n).collect())
}

fn system_default(input: bool) -> Option<String> {
    let (get, list) = if input {
        ("get-default-source", "sources")
    } else {
        ("get-default-sink", "sinks")
    };
    let run = |args: &[&str]| {
        std::process::Command::new("pactl")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let name = run(&[get])?;
    describe(&run(&["-f", "json", "list", list])?, &name)
}

fn describe(json: &str, name: &str) -> Option<String> {
    let nodes: Vec<serde_json::Value> = serde_json::from_str(json).ok()?;
    nodes
        .iter()
        .find(|n| n["name"] == name)
        .and_then(|n| n["description"].as_str())
        .map(str::to_string)
}

fn pick(
    list: &Devices,
    wanted: Option<&str>,
    fallback: Option<&str>,
) -> Option<(String, Vec<String>)> {
    let named = |n: &str| list.iter().find(|(d, _)| d == n);
    wanted
        .and_then(named)
        .or_else(|| fallback.and_then(named))
        .or_else(|| list.first())
        .cloned()
}

fn realtime_budget_ok() -> bool {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    let ok = unsafe { libc::getrlimit(libc::RLIMIT_RTTIME, &mut limit) } == 0;
    !ok || limit.rlim_cur != 0
}

fn check_realtime() -> Result<(), String> {
    if realtime_budget_ok() {
        Ok(())
    } else {
        Err("the desktop portal grants no realtime time. Log out and back in, or run: systemctl --user restart xdg-desktop-portal".into())
    }
}

fn choose(
    client: &Client,
    capture: bool,
    wanted: Option<&str>,
) -> Result<(String, Vec<String>), String> {
    let list = devices(client, capture);
    let fallback = if wanted.is_some_and(|w| list.iter().any(|(d, _)| d == w)) {
        None
    } else {
        system_default(capture)
    };
    pick(&list, wanted, fallback.as_deref()).ok_or_else(|| {
        if capture {
            "no input device".to_string()
        } else {
            "no output device".to_string()
        }
    })
}

struct Play {
    runner: Runner,
    left: jack::Port<AudioOut>,
    right: jack::Port<AudioOut>,
    scratch: Vec<f32>,
}

impl jack::ProcessHandler for Play {
    fn process(&mut self, _: &Client, ps: &ProcessScope) -> Control {
        let left = self.left.as_mut_slice(ps);
        let right = self.right.as_mut_slice(ps);
        let mut at = 0;
        while at < left.len() {
            let n = (left.len() - at).min(SCRATCH);
            let buf = &mut self.scratch[..n * 2];
            self.runner.process(buf, 2);
            for (i, frame) in buf.chunks_exact(2).enumerate() {
                left[at + i] = frame[0];
                right[at + i] = frame[1];
            }
            at += n;
        }
        Control::Continue
    }
}

struct Record {
    left: jack::Port<AudioIn>,
    right: jack::Port<AudioIn>,
    frames: Producer<[f32; 2]>,
}

impl jack::ProcessHandler for Record {
    fn process(&mut self, _: &Client, ps: &ProcessScope) -> Control {
        for (l, r) in self.left.as_slice(ps).iter().zip(self.right.as_slice(ps)) {
            let _ = self.frames.push([*l, *r]);
        }
        Control::Continue
    }
}

fn connect(client: &Client, from: &str, to: &str) -> Result<(), String> {
    client
        .connect_ports_by_name(from, to)
        .map_err(|e| format!("cannot connect {from} to {to}: {e}"))
}

pub fn open_output(
    client: Client,
    project: Project,
    wanted: Option<&str>,
    sender: &Sender,
) -> Result<Output, String> {
    check_realtime()?;
    let (device, ports) = choose(&client, false, wanted)?;
    let _ = client.set_buffer_size(BUFFER);
    let rate = client.sample_rate();
    let left = client
        .register_port("out_L", AudioOut::default())
        .map_err(|e| e.to_string())?;
    let right = client
        .register_port("out_R", AudioOut::default())
        .map_err(|e| e.to_string())?;
    let names = [left.name(), right.name()].map(|n| n.unwrap_or_default());
    let (runner, remote) = flip_engine::live(project, rate);
    let play = Play {
        runner,
        left,
        right,
        scratch: vec![0.0; SCRATCH * 2],
    };
    let active = client.activate_async((), play).map_err(|e| e.to_string())?;
    let c = active.as_client();
    let out_l = &ports[0];
    connect(c, &names[0], out_l)?;
    connect(c, &names[1], ports.get(1).unwrap_or(out_l))?;
    sender.replace(remote.commands);
    Ok(Output {
        rate,
        device,
        status: remote.status,
        trash: remote.trash,
        tap: remote.tap,
        _stream: Box::new(active),
    })
}

pub fn open_input(client: Client, wanted: Option<&str>) -> Result<Input, String> {
    check_realtime()?;
    let (device, ports) = choose(&client, true, wanted)?;
    let rate = client.sample_rate();
    let left = client
        .register_port("in_L", AudioIn::default())
        .map_err(|e| e.to_string())?;
    let right = client
        .register_port("in_R", AudioIn::default())
        .map_err(|e| e.to_string())?;
    let names = [left.name(), right.name()].map(|n| n.unwrap_or_default());
    let (producer, consumer) = RingBuffer::new(rate as usize * INPUT_SECONDS);
    let record = Record {
        left,
        right,
        frames: producer,
    };
    let active = client
        .activate_async((), record)
        .map_err(|e| e.to_string())?;
    let c = active.as_client();
    let in_l = &ports[0];
    connect(c, in_l, &names[0])?;
    connect(c, ports.get(1).unwrap_or(in_l), &names[1])?;
    Ok(Input {
        rate,
        device,
        frames: consumer,
        _stream: Box::new(active),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Devices {
        vec![
            (
                "Built-in Audio Analog Stereo".into(),
                vec!["a:playback_FL".into()],
            ),
            (
                "Scarlett Solo 4th Gen Pro".into(),
                vec!["s:playback_AUX0".into()],
            ),
        ]
    }

    mod when_picking_a_device {
        use super::*;

        mod with_a_known_wanted_device {
            use super::*;

            #[test]
            fn uses_it() {
                let p = pick(&list(), Some("Scarlett Solo 4th Gen Pro"), None);
                assert_eq!(p.unwrap().0, "Scarlett Solo 4th Gen Pro");
            }
        }

        mod with_an_unknown_wanted_device {
            use super::*;

            #[test]
            fn uses_the_system_default() {
                let p = pick(&list(), Some("gone"), Some("Scarlett Solo 4th Gen Pro"));
                assert_eq!(p.unwrap().0, "Scarlett Solo 4th Gen Pro");
            }
        }

        mod without_a_system_default {
            use super::*;

            #[test]
            fn uses_the_first_device() {
                let p = pick(&list(), None, None);
                assert_eq!(p.unwrap().0, "Built-in Audio Analog Stereo");
            }
        }
    }

    mod when_reading_the_system_default {
        use super::*;

        #[test]
        fn finds_the_description_of_the_named_node() {
            let json = r#"[{"name":"alsa_output.a","description":"Built-in"},{"name":"alsa_output.b","description":"Scarlett"}]"#;
            assert_eq!(describe(json, "alsa_output.b").as_deref(), Some("Scarlett"));
            assert_eq!(describe(json, "missing"), None);
        }
    }
}
