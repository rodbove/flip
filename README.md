# flip

A pad sampler and synthesizer that runs on your computer. Record sounds, chop them, play them on pads and make loops with a step sequencer.

## Features

- 4 groups (A to D) of 16 pads.
- Each pad plays a sample or a simple synth with two oscillators.
- Each pad has a start and end point, pitch, an envelope, a filter, pan and a choke group.
- Play modes: one shot, gate and loop.
- 16 patterns for each group. Each pattern has up to 64 steps.
- Each step has a velocity, a note and a chance to play.
- A song mode chains patterns.
- Record from a microphone or line input. Recording starts when the sound is louder than the threshold.
- Resample the master output or one group into a pad.
- Bounce the current pattern into a free pad.
- Chop a sample into slices at each hit.
- Drop audio files (WAV, AIFF, MP3, FLAC, OGG, M4A) on the window to load them.
- Play from the computer keyboard or a MIDI controller.
- Map MIDI controls to the knobs.
- Export a pattern or the song as a WAV file.

## Run

1. Install Rust.
2. Run `cargo run --release`.

On macOS, build an app bundle to use the microphone:

1. Run `./scripts/bundle.sh`.
2. Open `target/Flip.app`.
3. Allow microphone access when macOS asks.

## Controls

Read `docs/tutorial.html` for a short tutorial.

The keys work by position, so all keyboard layouts use the same keys. The table names the keys by their place on a QWERTY keyboard. After you press a pad key one time, the pad shows the letter of your layout.

| Key | Action |
| --- | --- |
| 1-4, Q-R, A-F, Z-V | Play pads |
| Shift + pad | Select a pad without sound |
| Tab | Keys mode: play the selected pad as a piano |
| Z / X | Octave down / up in keys mode |
| Space | Play / stop |
| Enter | Record pad hits into the pattern |
| B | Start or stop a sample recording |
| M | Metronome |
| [ / ] | Previous / next pattern |
| Left / Right | Previous / next group |
| Up / Down | Tempo |
| Backspace | Clear the steps of the selected pad |
| 5 / 6 / 7 | Screen: pad / steps / song |
| Cmd + Z, S, O, E, N | Undo, save, open, export, new |
| / | Show the key list |

Mouse:

- Drag a knob up or down. Hold Shift for small changes.
- Double-click a knob to reset it.
- Right-click a knob, then move a MIDI control to map it.
- Right-click a group to mute it.
- Right-click a pattern to copy the current pattern into it.
- In the step grid, right-click a step to edit its velocity, note and chance.

## MIDI

- Notes 36 to 51 play the 16 pads. Note 36 is the bottom-left pad.
- Other notes play the selected pad at that pitch.
- In keys mode, all notes play the selected pad.

## Projects

A project is a folder. It contains `project.json` and a `samples` folder with WAV files. Projects save to `~/Music/Flip` by default.
