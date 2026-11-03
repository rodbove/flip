use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender as Channel, channel};

use flip_engine::{Command, PADS};
use midir::{MidiInput, MidiInputConnection};

use crate::audio::Sender;

const CLIENT: &str = "flip";
const FIRST_PAD: u8 = 36;
const MIDDLE_C: i32 = 60;

#[derive(Default)]
pub struct Route {
    pub group: AtomicUsize,
    pub pad: AtomicUsize,
    pub keys: AtomicBool,
}

const SUSTAIN: u8 = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Note {
        group: usize,
        pad: usize,
        note: i8,
        velocity: f32,
        on: bool,
        melodic: bool,
        key: u8,
    },
    Control {
        cc: u8,
        value: u8,
    },
}

pub fn pad_for(note: u8) -> Option<usize> {
    let n = note.checked_sub(FIRST_PAD)? as usize;
    if n >= PADS {
        return None;
    }
    let row = 3 - n / 4;
    Some(row * 4 + n % 4)
}

pub fn melodic(route: &Route, note: u8) -> bool {
    route.keys.load(Ordering::Relaxed) || pad_for(note).is_none()
}

pub fn map(route: &Route, note: u8) -> (usize, usize, i8) {
    let group = route.group.load(Ordering::Relaxed);
    let selected = route.pad.load(Ordering::Relaxed);
    if !route.keys.load(Ordering::Relaxed)
        && let Some(pad) = pad_for(note)
    {
        return (group, pad, 0);
    }
    let offset = (note as i32 - MIDDLE_C).clamp(-48, 48) as i8;
    (group, selected, offset)
}

pub fn parse(route: &Route, bytes: &[u8]) -> Option<Event> {
    let (&status, rest) = bytes.split_first()?;
    match (status & 0xF0, rest) {
        (0x90, [note, vel, ..]) | (0x80, [note, vel, ..]) => {
            let on = status & 0xF0 == 0x90 && *vel > 0;
            let key = *note;
            let melodic = melodic(route, *note);
            let (group, pad, note) = map(route, *note);
            Some(Event::Note {
                group,
                pad,
                note,
                velocity: *vel as f32 / 127.0,
                on,
                melodic,
                key,
            })
        }
        (0xB0, [cc, value, ..]) => Some(Event::Control {
            cc: *cc,
            value: *value,
        }),
        _ => None,
    }
}

#[derive(Default)]
pub struct Sustain {
    down: bool,
    held: Vec<Event>,
}

impl Sustain {
    pub fn filter(&mut self, event: Event) -> Vec<Event> {
        match event {
            Event::Control { cc: SUSTAIN, value } => {
                self.down = value >= 64;
                if self.down {
                    Vec::new()
                } else {
                    std::mem::take(&mut self.held)
                }
            }
            Event::Note {
                group,
                pad,
                note,
                on: true,
                ..
            } => {
                self.held.retain(|h| !same_key(h, group, pad, note));
                vec![event]
            }
            Event::Note { on: false, .. } if self.down => {
                self.held.push(event);
                Vec::new()
            }
            _ => vec![event],
        }
    }
}

fn same_key(e: &Event, g: usize, p: usize, n: i8) -> bool {
    matches!(*e, Event::Note { group, pad, note, .. } if group == g && pad == p && note == n)
}

pub struct Midi {
    connection: Option<MidiInputConnection<()>>,
    pub port: Option<String>,
    events: Receiver<Event>,
    channel: Channel<Event>,
}

impl Default for Midi {
    fn default() -> Self {
        let (channel, events) = channel();
        Self {
            connection: None,
            port: None,
            events,
            channel,
        }
    }
}

pub fn ports() -> Vec<String> {
    let Ok(input) = MidiInput::new(CLIENT) else {
        return Vec::new();
    };
    input
        .ports()
        .iter()
        .filter_map(|p| input.port_name(p).ok())
        .collect()
}

pub fn wanted(ports: &[String]) -> Option<String> {
    ports
        .iter()
        .find(|p| !p.starts_with("Midi Through"))
        .cloned()
}

impl Midi {
    pub fn connect(
        &mut self,
        wanted: &str,
        route: Arc<Route>,
        sender: Sender,
        ctx: eframe::egui::Context,
    ) -> Result<(), String> {
        self.disconnect();
        let input = MidiInput::new(CLIENT).map_err(|e| e.to_string())?;
        let port = input
            .ports()
            .into_iter()
            .find(|p| input.port_name(p).is_ok_and(|n| n == wanted))
            .ok_or("port not found")?;
        let channel = self.channel.clone();
        let mut sustain = Sustain::default();
        let connection = input
            .connect(
                &port,
                "flip-in",
                move |_, bytes, _| {
                    let Some(event) = parse(&route, bytes) else {
                        return;
                    };
                    for event in sustain.filter(event) {
                        if let Event::Note {
                            group,
                            pad,
                            note,
                            velocity,
                            on,
                            ..
                        } = event
                        {
                            sender.send(Command::Note {
                                group,
                                pad,
                                note,
                                velocity,
                                on,
                            });
                        }
                        let _ = channel.send(event);
                    }
                    ctx.request_repaint();
                },
                (),
            )
            .map_err(|e| e.to_string())?;
        self.connection = Some(connection);
        self.port = Some(wanted.to_string());
        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(c) = self.connection.take() {
            c.close();
        }
        self.port = None;
    }

    pub fn poll(&self) -> Vec<Event> {
        self.events.try_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod when_choosing_a_port_on_start {
        use super::*;

        #[test]
        fn skips_the_linux_through_port() {
            let ports = vec![
                "Midi Through:Midi Through Port-0 14:0".to_string(),
                "Digital Keyboard:Digital Keyboard MIDI 1 20:0".to_string(),
            ];
            assert_eq!(wanted(&ports), Some(ports[1].clone()));
            assert_eq!(wanted(&ports[..1]), None);
        }
    }

    mod when_mapping_pad_notes {
        use super::*;

        #[test]
        fn puts_the_first_note_bottom_left() {
            assert_eq!(pad_for(36), Some(12));
            assert_eq!(pad_for(39), Some(15));
            assert_eq!(pad_for(48), Some(0));
            assert_eq!(pad_for(51), Some(3));
            assert_eq!(pad_for(35), None);
            assert_eq!(pad_for(52), None);
        }

        #[test]
        fn plays_other_notes_on_the_selected_pad() {
            let route = Route::default();
            route.pad.store(5, Ordering::Relaxed);
            assert_eq!(map(&route, 64), (0, 5, 4));
            assert_eq!(map(&route, 36), (0, 12, 0));
        }
    }

    mod when_in_keys_mode {
        use super::*;

        #[test]
        fn plays_every_note_on_the_selected_pad() {
            let route = Route::default();
            route.keys.store(true, Ordering::Relaxed);
            route.pad.store(2, Ordering::Relaxed);
            assert_eq!(map(&route, 36), (0, 2, -24));
        }
    }

    mod when_parsing {
        use super::*;

        #[test]
        fn treats_zero_velocity_as_release() {
            let route = Route::default();
            let e = parse(&route, &[0x90, 36, 0]).unwrap();
            assert!(matches!(e, Event::Note { on: false, .. }));
        }

        #[test]
        fn marks_keyboard_notes_as_melodic() {
            let route = Route::default();
            assert!(matches!(
                parse(&route, &[0x90, 72, 90]),
                Some(Event::Note { melodic: true, .. })
            ));
            assert!(matches!(
                parse(&route, &[0x90, 40, 90]),
                Some(Event::Note { melodic: false, .. })
            ));
        }

        #[test]
        fn reads_controls() {
            let route = Route::default();
            assert_eq!(
                parse(&route, &[0xB3, 74, 100]),
                Some(Event::Control { cc: 74, value: 100 })
            );
        }
    }

    mod when_the_sustain_pedal_is_down {
        use super::*;

        fn note(n: i8, on: bool) -> Event {
            Event::Note {
                group: 0,
                pad: 1,
                note: n,
                velocity: 0.8,
                on,
                melodic: true,
                key: (60 + n as i32) as u8,
            }
        }

        fn pedal(value: u8) -> Event {
            Event::Control { cc: SUSTAIN, value }
        }

        #[test]
        fn holds_releases_until_the_pedal_lifts() {
            let mut s = Sustain::default();
            assert_eq!(s.filter(pedal(127)), vec![]);
            assert_eq!(s.filter(note(4, true)), vec![note(4, true)]);
            assert_eq!(s.filter(note(4, false)), vec![]);
            assert_eq!(s.filter(pedal(0)), vec![note(4, false)]);
            assert_eq!(s.filter(note(4, false)), vec![note(4, false)]);
        }

        #[test]
        fn forgets_a_held_release_when_the_key_is_struck_again() {
            let mut s = Sustain::default();
            s.filter(pedal(127));
            s.filter(note(4, true));
            s.filter(note(4, false));
            s.filter(note(4, true));
            assert_eq!(s.filter(pedal(0)), vec![]);
        }
    }
}
