use anyhow::{Context, Result, bail};
use crossbeam_queue::ArrayQueue;
use midir::{MidiInput, MidiInputConnection, MidiOutput};
use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use crate::audio::AudioCommand;
use crate::music::MidiNote;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MidiMessage {
    NoteOn {
        channel: u8,
        note: MidiNote,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: MidiNote,
        velocity: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    Sustain {
        channel: u8,
        down: bool,
        value: u8,
    },
    Ignored {
        status: Option<u8>,
        length: usize,
    },
}

impl fmt::Display for MidiMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoteOn {
                channel,
                note,
                velocity,
            } => write!(
                f,
                "ch {} note on  {:>3} {:>3} vel {:>3}",
                channel + 1,
                note.value(),
                note,
                velocity
            ),
            Self::NoteOff {
                channel,
                note,
                velocity,
            } => write!(
                f,
                "ch {} note off {:>3} {:>3} vel {:>3}",
                channel + 1,
                note.value(),
                note,
                velocity
            ),
            Self::ControlChange {
                channel,
                controller,
                value,
            } => write!(
                f,
                "ch {} cc {:>3} value {:>3}",
                channel + 1,
                controller,
                value
            ),
            Self::Sustain {
                channel,
                down,
                value,
            } => write!(
                f,
                "ch {} sustain {} ({value})",
                channel + 1,
                if *down { "down" } else { "up" }
            ),
            Self::Ignored { status, length } => write!(
                f,
                "ignored MIDI data: status {} length {length}",
                status.map_or_else(|| "—".into(), |value| format!("0x{value:02x}"))
            ),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MidiEvent {
    pub message: MidiMessage,
    pub at: Instant,
}

pub fn parse(bytes: &[u8]) -> Option<MidiMessage> {
    let (&status, data) = bytes.split_first()?;
    if status < 0x80 {
        return None;
    }
    let channel = status & 0x0f;
    match status & 0xf0 {
        0x80 if data.len() >= 2 => Some(MidiMessage::NoteOff {
            channel,
            note: MidiNote::new(data[0])?,
            velocity: data[1].min(127),
        }),
        0x90 if data.len() >= 2 => {
            let note = MidiNote::new(data[0])?;
            let velocity = data[1].min(127);
            if velocity == 0 {
                Some(MidiMessage::NoteOff {
                    channel,
                    note,
                    velocity: 0,
                })
            } else {
                Some(MidiMessage::NoteOn {
                    channel,
                    note,
                    velocity,
                })
            }
        }
        0xb0 if data.len() >= 2 => {
            let controller = data[0];
            let value = data[1].min(127);
            if controller == 64 {
                Some(MidiMessage::Sustain {
                    channel,
                    down: value >= 64,
                    value,
                })
            } else {
                Some(MidiMessage::ControlChange {
                    channel,
                    controller,
                    value,
                })
            }
        }
        _ => None,
    }
}

#[derive(Debug, Default)]
pub struct DeviceInventory {
    pub midi_inputs: Vec<String>,
    pub midi_outputs: Vec<String>,
}

pub fn inventory() -> Result<DeviceInventory> {
    let input = MidiInput::new("phase-discovery").context("initialize CoreMIDI input")?;
    let output = MidiOutput::new("phase-discovery").context("initialize CoreMIDI output")?;
    let midi_inputs = input
        .ports()
        .iter()
        .map(|p| input.port_name(p).unwrap_or_else(|_| "<unnamed>".into()))
        .collect();
    let midi_outputs = output
        .ports()
        .iter()
        .map(|p| output.port_name(p).unwrap_or_else(|_| "<unnamed>".into()))
        .collect();
    Ok(DeviceInventory {
        midi_inputs,
        midi_outputs,
    })
}

pub fn choose_port(names: &[String], requested: Option<&str>) -> Option<usize> {
    if let Some(needle) = requested {
        let needle = needle.to_lowercase();
        return names
            .iter()
            .position(|n| n.to_lowercase().contains(&needle));
    }
    names
        .iter()
        .position(|n| n.to_lowercase().contains("mpkmini2"))
        .or_else(|| (names.len() == 1).then_some(0))
}

pub struct MidiConnection {
    _connection: MidiInputConnection<()>,
    pub name: String,
}

pub fn connect(
    requested: Option<&str>,
    events: Arc<ArrayQueue<MidiEvent>>,
    audio: Option<Arc<ArrayQueue<AudioCommand>>>,
    debug: bool,
) -> Result<Option<MidiConnection>> {
    let input = MidiInput::new("phase").context("initialize CoreMIDI input")?;
    let ports = input.ports();
    let names: Vec<_> = ports
        .iter()
        .map(|p| input.port_name(p).unwrap_or_else(|_| "<unnamed>".into()))
        .collect();
    if ports.is_empty() {
        return Ok(None);
    }
    let Some(index) = choose_port(&names, requested) else {
        if let Some(needle) = requested {
            bail!("no MIDI input contains '{needle}'; run `phase devices`");
        }
        bail!("multiple MIDI inputs found and MPKmini2 is absent; use --midi-port <substring>");
    };
    let name = names[index].clone();
    let port = ports[index].clone();
    let connection = input
        .connect(
            &port,
            "phase-input",
            move |_timestamp, bytes, ()| {
                if let Some(message) = parse(bytes) {
                    let _ = events.push(MidiEvent {
                        message,
                        at: Instant::now(),
                    });
                    if let (Some(queue), Some(command)) = (&audio, AudioCommand::from_midi(message))
                    {
                        let _ = queue.push(command);
                    }
                } else if debug {
                    let _ = events.push(MidiEvent {
                        message: MidiMessage::Ignored {
                            status: bytes.first().copied(),
                            length: bytes.len(),
                        },
                        at: Instant::now(),
                    });
                }
            },
            (),
        )
        .context("connect to CoreMIDI input")?;
    Ok(Some(MidiConnection {
        _connection: connection,
        name,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_note_messages_and_channels() {
        assert_eq!(
            parse(&[0x92, 60, 100]),
            Some(MidiMessage::NoteOn {
                channel: 2,
                note: MidiNote::new(60).unwrap(),
                velocity: 100
            })
        );
        assert_eq!(
            parse(&[0x81, 61, 45]),
            Some(MidiMessage::NoteOff {
                channel: 1,
                note: MidiNote::new(61).unwrap(),
                velocity: 45
            })
        );
    }

    #[test]
    fn zero_velocity_note_on_is_note_off() {
        assert_eq!(
            parse(&[0x90, 64, 0]),
            Some(MidiMessage::NoteOff {
                channel: 0,
                note: MidiNote::new(64).unwrap(),
                velocity: 0
            })
        );
    }

    #[test]
    fn parses_sustain_threshold_and_other_cc() {
        assert_eq!(
            parse(&[0xb0, 64, 63]),
            Some(MidiMessage::Sustain {
                channel: 0,
                down: false,
                value: 63
            })
        );
        assert_eq!(
            parse(&[0xb0, 64, 64]),
            Some(MidiMessage::Sustain {
                channel: 0,
                down: true,
                value: 64
            })
        );
        assert!(matches!(
            parse(&[0xb3, 1, 20]),
            Some(MidiMessage::ControlChange { channel: 3, .. })
        ));
    }

    #[test]
    fn malformed_and_unsupported_data_is_ignored() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x90, 60]).is_none());
        assert!(parse(&[0xf8]).is_none());
        assert!(parse(&[0x40, 10, 10]).is_none());
    }

    #[test]
    fn device_selection_prefers_requested_then_mpk_then_single() {
        let names = vec!["Other".into(), "AKAI MPKmini2".into()];
        assert_eq!(choose_port(&names, None), Some(1));
        assert_eq!(choose_port(&names, Some("other")), Some(0));
        assert_eq!(choose_port(&["Solo".into()], None), Some(0));
        assert_eq!(choose_port(&names, Some("missing")), None);
    }
}
