use anyhow::{Context, Result, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use crossbeam_queue::ArrayQueue;
use std::f32::consts::TAU;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::midi::MidiMessage;
use crate::music::MidiNote;

pub const MAX_VOICES: usize = 32;
pub const COMMAND_QUEUE_CAPACITY: usize = 512;

#[derive(Clone, Copy, Debug)]
pub enum AudioCommand {
    NoteOn { note: MidiNote, velocity: u8 },
    NoteOff { note: MidiNote },
    Sustain(bool),
    SetVolume(f32),
    Click { accent: bool },
    AllNotesOff,
}

impl AudioCommand {
    pub fn from_midi(message: MidiMessage) -> Option<Self> {
        match message {
            MidiMessage::NoteOn { note, velocity, .. } => Some(Self::NoteOn { note, velocity }),
            MidiMessage::NoteOff { note, .. } => Some(Self::NoteOff { note }),
            MidiMessage::Sustain { down, .. } => Some(Self::Sustain(down)),
            MidiMessage::ControlChange { .. } | MidiMessage::Ignored { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnvelopeStage {
    Off,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug)]
struct Voice {
    note: u8,
    velocity: f32,
    phase: f32,
    transient_phase: f32,
    transient: f32,
    envelope: f32,
    release_step: f32,
    stage: EnvelopeStage,
    key_down: bool,
    started: u64,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            note: 0,
            velocity: 0.0,
            phase: 0.0,
            transient_phase: 0.0,
            transient: 0.0,
            envelope: 0.0,
            release_step: 0.0,
            stage: EnvelopeStage::Off,
            key_down: false,
            started: 0,
        }
    }
}

impl Voice {
    fn begin(&mut self, note: MidiNote, velocity: u8, serial: u64) {
        *self = Self {
            note: note.value(),
            velocity: f32::from(velocity) / 127.0,
            stage: EnvelopeStage::Attack,
            key_down: true,
            transient: 1.0,
            started: serial,
            ..Self::default()
        };
    }

    fn release(&mut self, sample_rate: f32) {
        if self.stage != EnvelopeStage::Off && self.stage != EnvelopeStage::Release {
            self.stage = EnvelopeStage::Release;
            self.release_step =
                (self.envelope / (0.28 * sample_rate)).max(1.0 / (sample_rate * 2.0));
        }
    }

    fn sample(&mut self, sample_rate: f32) -> f32 {
        if self.stage == EnvelopeStage::Off {
            return 0.0;
        }

        match self.stage {
            EnvelopeStage::Attack => {
                self.envelope += 1.0 / (0.008 * sample_rate);
                if self.envelope >= 1.0 {
                    self.envelope = 1.0;
                    self.stage = EnvelopeStage::Decay;
                }
            }
            EnvelopeStage::Decay => {
                self.envelope -= (1.0 - 0.52) / (0.32 * sample_rate);
                if self.envelope <= 0.52 {
                    self.envelope = 0.52;
                    self.stage = EnvelopeStage::Sustain;
                }
            }
            EnvelopeStage::Sustain => {}
            EnvelopeStage::Release => {
                self.envelope -= self.release_step;
                if self.envelope <= 0.0001 {
                    self.envelope = 0.0;
                    self.stage = EnvelopeStage::Off;
                    return 0.0;
                }
            }
            EnvelopeStage::Off => return 0.0,
        }

        let frequency = MidiNote::new(self.note).map_or(440.0, MidiNote::frequency);
        self.phase = (self.phase + frequency / sample_rate).fract();
        self.transient_phase = (self.transient_phase + frequency * 5.97 / sample_rate).fract();
        let velocity_curve = self.velocity.sqrt();
        let fundamental = (self.phase * TAU).sin();
        let second = (self.phase * 2.0 * TAU).sin() * 0.24;
        let third = (self.phase * 3.0 * TAU).sin() * 0.09;
        let warmth = (self.phase * 0.5 * TAU).sin() * 0.05;
        let hammer =
            (self.transient_phase * TAU).sin() * self.transient * (0.08 + self.velocity * 0.12);
        self.transient *= (-1.0 / (0.055 * sample_rate)).exp();
        (fundamental + second + third + warmth + hammer) * self.envelope * velocity_curve * 0.21
    }

    fn active(self) -> bool {
        self.stage != EnvelopeStage::Off
    }
}

pub struct SynthEngine {
    voices: [Voice; MAX_VOICES],
    sample_rate: f32,
    sustain: bool,
    master_volume: f32,
    serial: u64,
    click_phase: f32,
    click_remaining: u32,
    click_accent: bool,
}

impl SynthEngine {
    pub fn new(sample_rate: f32, master_volume: f32) -> Self {
        Self {
            voices: [Voice::default(); MAX_VOICES],
            sample_rate: sample_rate.max(8_000.0),
            sustain: false,
            master_volume: master_volume.clamp(0.0, 1.0),
            serial: 0,
            click_phase: 0.0,
            click_remaining: 0,
            click_accent: false,
        }
    }

    pub fn process(&mut self, command: AudioCommand) {
        match command {
            AudioCommand::NoteOn { note, velocity } if velocity > 0 => {
                self.serial = self.serial.wrapping_add(1);
                let index = self.allocate_voice();
                self.voices[index].begin(note, velocity, self.serial);
            }
            AudioCommand::NoteOn { note, velocity: 0 } | AudioCommand::NoteOff { note } => {
                for voice in &mut self.voices {
                    if voice.active() && voice.note == note.value() && voice.key_down {
                        voice.key_down = false;
                        if !self.sustain {
                            voice.release(self.sample_rate);
                        }
                    }
                }
            }
            AudioCommand::Sustain(down) => {
                let was_down = self.sustain;
                self.sustain = down;
                if was_down && !down {
                    for voice in &mut self.voices {
                        if voice.active() && !voice.key_down {
                            voice.release(self.sample_rate);
                        }
                    }
                }
            }
            AudioCommand::SetVolume(volume) => self.master_volume = volume.clamp(0.0, 1.0),
            AudioCommand::Click { accent } => {
                self.click_remaining = (self.sample_rate * 0.035) as u32;
                self.click_phase = 0.0;
                self.click_accent = accent;
            }
            AudioCommand::AllNotesOff => {
                self.sustain = false;
                for voice in &mut self.voices {
                    voice.key_down = false;
                    voice.release(self.sample_rate);
                }
            }
            AudioCommand::NoteOn { .. } => {}
        }
    }

    fn allocate_voice(&self) -> usize {
        self.voices
            .iter()
            .position(|v| !v.active())
            .unwrap_or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, voice)| voice.started)
                    .map_or(0, |(index, _)| index)
            })
    }

    pub fn next_sample(&mut self) -> f32 {
        let mut mixed = 0.0;
        for voice in &mut self.voices {
            mixed += voice.sample(self.sample_rate);
        }
        if self.click_remaining > 0 {
            let frequency = if self.click_accent { 1_760.0 } else { 1_320.0 };
            self.click_phase = (self.click_phase + frequency / self.sample_rate).fract();
            let decay = self.click_remaining as f32 / (self.sample_rate * 0.035);
            mixed += (self.click_phase * TAU).sin() * decay * 0.18;
            self.click_remaining -= 1;
        }
        let scaled = mixed * self.master_volume;
        let limited = scaled / (1.0 + scaled.abs());
        if limited.is_finite() { limited } else { 0.0 }
    }

    #[cfg(test)]
    fn active_count(&self) -> usize {
        self.voices.iter().filter(|v| v.active()).count()
    }
}

#[derive(Debug)]
pub struct AudioDeviceInventory {
    pub outputs: Vec<String>,
    pub default: Option<String>,
}

pub fn inventory() -> Result<AudioDeviceInventory> {
    let host = cpal::default_host();
    let outputs = host
        .output_devices()
        .context("enumerate CoreAudio output devices")?
        .map(|device| device.to_string())
        .collect();
    let default = host
        .default_output_device()
        .map(|device| device.to_string());
    Ok(AudioDeviceInventory { outputs, default })
}

pub struct AudioEngine {
    _stream: cpal::Stream,
    pub queue: Arc<ArrayQueue<AudioCommand>>,
    pub device_name: String,
    healthy: Arc<AtomicBool>,
}

impl AudioEngine {
    pub fn start(requested: Option<&str>, volume: f32) -> Result<Self> {
        let host = cpal::default_host();
        let device = if let Some(needle) = requested {
            let needle_lower = needle.to_lowercase();
            host.output_devices()
                .context("enumerate CoreAudio output devices")?
                .find(|device| device.to_string().to_lowercase().contains(&needle_lower))
                .with_context(|| format!("no audio output contains '{needle}'"))?
        } else {
            host.default_output_device()
                .context("no default audio output device")?
        };
        let device_name = device.to_string();
        let supported = device
            .default_output_config()
            .context("query default audio format")?;
        let sample_format = supported.sample_format();
        let config = supported.config();
        let queue = Arc::new(ArrayQueue::new(COMMAND_QUEUE_CAPACITY));
        let healthy = Arc::new(AtomicBool::new(true));

        let stream = match sample_format {
            SampleFormat::F32 => {
                build_stream::<f32>(&device, &config, queue.clone(), healthy.clone(), volume)?
            }
            SampleFormat::I16 => {
                build_stream::<i16>(&device, &config, queue.clone(), healthy.clone(), volume)?
            }
            SampleFormat::U16 => {
                build_stream::<u16>(&device, &config, queue.clone(), healthy.clone(), volume)?
            }
            other => bail!("unsupported default audio sample format: {other:?}"),
        };
        stream.play().context("start CoreAudio output stream")?;
        Ok(Self {
            _stream: stream,
            queue,
            device_name,
            healthy,
        })
    }

    pub fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    pub fn send(&self, command: AudioCommand) {
        let _ = self.queue.push(command);
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    queue: Arc<ArrayQueue<AudioCommand>>,
    healthy: Arc<AtomicBool>,
    volume: f32,
) -> Result<cpal::Stream>
where
    T: Sample + SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let mut synth = SynthEngine::new(config.sample_rate as f32, volume);
    let stream = device
        .build_output_stream(
            *config,
            move |data: &mut [T], _| {
                while let Some(command) = queue.pop() {
                    synth.process(command);
                }
                for frame in data.chunks_mut(channels) {
                    let value = T::from_sample(synth.next_sample());
                    for sample in frame {
                        *sample = value;
                    }
                }
            },
            move |_| {
                healthy.store(false, Ordering::Relaxed);
            },
            None,
        )
        .context("build CoreAudio output stream")?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(value: u8) -> MidiNote {
        MidiNote::new(value).unwrap()
    }

    #[test]
    fn allocation_is_deterministic_and_uses_free_slots() {
        let mut synth = SynthEngine::new(48_000.0, 0.8);
        assert_eq!(synth.allocate_voice(), 0);
        synth.process(AudioCommand::NoteOn {
            note: note(60),
            velocity: 100,
        });
        assert_eq!(synth.allocate_voice(), 1);
    }

    #[test]
    fn steals_oldest_voice_when_full() {
        let mut synth = SynthEngine::new(48_000.0, 0.8);
        for n in 0..MAX_VOICES {
            synth.process(AudioCommand::NoteOn {
                note: note(40 + n as u8),
                velocity: 100,
            });
        }
        assert_eq!(synth.active_count(), MAX_VOICES);
        assert_eq!(synth.allocate_voice(), 0);
        synth.process(AudioCommand::NoteOn {
            note: note(90),
            velocity: 100,
        });
        assert_eq!(synth.voices[0].note, 90);
    }

    #[test]
    fn sustain_defers_then_triggers_release() {
        let mut synth = SynthEngine::new(1_000.0, 1.0);
        synth.process(AudioCommand::NoteOn {
            note: note(60),
            velocity: 100,
        });
        for _ in 0..20 {
            synth.next_sample();
        }
        synth.process(AudioCommand::Sustain(true));
        synth.process(AudioCommand::NoteOff { note: note(60) });
        assert_ne!(synth.voices[0].stage, EnvelopeStage::Release);
        synth.process(AudioCommand::Sustain(false));
        assert_eq!(synth.voices[0].stage, EnvelopeStage::Release);
    }

    #[test]
    fn envelope_releases_to_silence() {
        let mut synth = SynthEngine::new(8_000.0, 1.0);
        synth.process(AudioCommand::NoteOn {
            note: note(60),
            velocity: 127,
        });
        for _ in 0..1_000 {
            synth.next_sample();
        }
        synth.process(AudioCommand::NoteOff { note: note(60) });
        for _ in 0..5_000 {
            synth.next_sample();
        }
        assert_eq!(synth.active_count(), 0);
        assert_eq!(synth.next_sample(), 0.0);
    }

    #[test]
    fn offline_output_is_finite_and_bounded_at_common_rates() {
        for rate in [44_100.0, 48_000.0, 96_000.0] {
            let mut synth = SynthEngine::new(rate, 1.0);
            for n in 48..80 {
                synth.process(AudioCommand::NoteOn {
                    note: note(n),
                    velocity: 127,
                });
            }
            for _ in 0..rate as usize {
                let sample = synth.next_sample();
                assert!(sample.is_finite());
                assert!((-1.0..=1.0).contains(&sample));
            }
        }
    }
}
