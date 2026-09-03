use crate::chord::{self, Chord};
use crate::controls::{ControlAction, PatchSettings, describe_cc, map_cc};
use crate::midi::{MidiEvent, MidiMessage};
use crate::music::MidiNote;
use crate::trainer::{
    Attempt, NoteExercise, RhythmGrade, ScaleExercise, SessionMetrics, classify_rhythm,
    nearest_beat_offset,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};

pub const RECENT_EVENT_CAPACITY: usize = 8;
pub const KEYBOARD_SPAN: u8 = 24;
const DEFAULT_KEYBOARD_BASE: u8 = 48;
const HIGHEST_KEYBOARD_BASE: u8 = 96;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Freeplay,
    Notes,
    Scales,
    Rhythm,
}

impl Mode {
    const ALL: [Self; 4] = [Self::Freeplay, Self::Notes, Self::Scales, Self::Rhythm];

    fn shift(self, amount: i8) -> Self {
        let index = Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0) as i8;
        Self::ALL[(index + amount).rem_euclid(Self::ALL.len() as i8) as usize]
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Freeplay => "freeplay",
            Self::Notes => "notes",
            Self::Scales => "scales",
            Self::Rhythm => "rhythm",
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoteState {
    pub velocity: u8,
    pub held: bool,
}

pub struct App {
    pub mode: Mode,
    pub notes: [NoteState; 128],
    pub keyboard_base: u8,
    pub sustain: bool,
    pub volume: f32,
    pub muted: bool,
    pub bpm: u16,
    pub paused: bool,
    pub help: bool,
    pub midi_name: String,
    pub audio_name: String,
    pub audio_ok: bool,
    pub warning: Option<String>,
    pub recent: VecDeque<String>,
    pub chord: Option<Chord>,
    pub patch: PatchSettings,
    pub last_control: Option<String>,
    pub note_exercise: NoteExercise,
    pub scale_exercise: ScaleExercise,
    pub rhythm_metrics: SessionMetrics,
    pub rhythm_note: MidiNote,
    pub last_feedback: String,
    pub started: Instant,
    rhythm_epoch: Instant,
    rhythm_last_beat: u64,
}

impl App {
    pub fn new(now: Instant, volume: f32, bpm: u16, range: (u8, u8)) -> Self {
        Self {
            mode: Mode::Freeplay,
            notes: [NoteState::default(); 128],
            keyboard_base: DEFAULT_KEYBOARD_BASE,
            sustain: false,
            volume,
            muted: false,
            bpm,
            paused: false,
            help: false,
            midi_name: "not connected".into(),
            audio_name: "disabled".into(),
            audio_ok: false,
            warning: None,
            recent: VecDeque::with_capacity(RECENT_EVENT_CAPACITY),
            chord: None,
            patch: PatchSettings::default(),
            last_control: None,
            note_exercise: NoteExercise::new(range, now),
            scale_exercise: ScaleExercise::new(now),
            rhythm_metrics: SessionMetrics::default(),
            rhythm_note: MidiNote::new(60).expect("middle C is a valid MIDI note"),
            last_feedback: "ready".into(),
            started: now,
            rhythm_epoch: now,
            rhythm_last_beat: 0,
        }
    }

    pub fn handle_midi(&mut self, event: MidiEvent) {
        let recent = match event.message {
            MidiMessage::ControlChange {
                channel,
                controller,
                value,
            } => describe_cc(controller, value).map_or_else(
                || event.message.to_string(),
                |description| format!("ch {} {description} [{value}]", channel + 1),
            ),
            _ => event.message.to_string(),
        };
        self.push_recent(recent);
        match event.message {
            MidiMessage::NoteOn { note, velocity, .. } => {
                self.follow_keyboard_note(note);
                self.notes[usize::from(note.value())] = NoteState {
                    velocity,
                    held: true,
                };
                if !self.paused {
                    self.score_onset(note, event.at);
                }
            }
            MidiMessage::NoteOff { note, .. } => {
                let state = &mut self.notes[usize::from(note.value())];
                state.held = false;
                if !self.sustain {
                    state.velocity = 0;
                }
            }
            MidiMessage::Sustain { down, .. } => {
                self.sustain = down;
                if !down {
                    for state in &mut self.notes {
                        if !state.held {
                            state.velocity = 0;
                        }
                    }
                }
            }
            MidiMessage::ControlChange {
                controller, value, ..
            } => {
                if let Some(action) = map_cc(controller, value) {
                    self.apply_control(action);
                    self.last_control = describe_cc(controller, value);
                }
            }
            MidiMessage::Ignored { .. } => {}
        }
        self.refresh_chord();
    }

    fn score_onset(&mut self, note: MidiNote, now: Instant) {
        match self.mode {
            Mode::Freeplay => {}
            Mode::Notes => {
                let attempt = self.note_exercise.attempt(note, now);
                self.last_feedback = format_attempt(attempt);
            }
            Mode::Scales => {
                let attempt = self.scale_exercise.attempt(note, now);
                self.last_feedback = format_attempt(attempt);
            }
            Mode::Rhythm => {
                let beat = self.beat_duration();
                let offset = nearest_beat_offset(now, self.rhythm_epoch, beat);
                let grade = if note == self.rhythm_note {
                    classify_rhythm(offset)
                } else {
                    RhythmGrade::Miss
                };
                let correct = note == self.rhythm_note && grade != RhythmGrade::Miss;
                self.rhythm_metrics.record(Attempt {
                    played: note,
                    expected: self.rhythm_note,
                    correct,
                    response_time: now.saturating_duration_since(self.rhythm_epoch),
                    timing_offset_ms: Some(offset),
                });
                self.last_feedback = format!("{grade} {offset:+} ms");
            }
        }
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        if self.mode != Mode::Rhythm || self.paused {
            return false;
        }
        let beat_nanos = self.beat_duration().as_nanos().max(1);
        let beat = now.saturating_duration_since(self.rhythm_epoch).as_nanos() / beat_nanos;
        let beat = beat.min(u128::from(u64::MAX)) as u64;
        if beat > self.rhythm_last_beat {
            self.rhythm_last_beat = beat;
            true
        } else {
            false
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, now: Instant) -> bool {
        if key.code == KeyCode::Char('q') && key.modifiers.is_empty() {
            return true;
        }
        match key.code {
            KeyCode::Char('?') => self.help = !self.help,
            KeyCode::Esc if self.help => self.help = false,
            KeyCode::Tab => self.set_mode(self.mode.shift(1), now),
            KeyCode::BackTab => self.set_mode(self.mode.shift(-1), now),
            KeyCode::Char(' ') => self.paused = !self.paused,
            KeyCode::Char('r') => self.restart(now),
            KeyCode::Char('m') => self.muted = !self.muted,
            KeyCode::Char('[') => self.volume = (self.volume - 0.05).clamp(0.0, 1.0),
            KeyCode::Char(']') => self.volume = (self.volume + 0.05).clamp(0.0, 1.0),
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.bpm = self.bpm.saturating_add(5).min(300)
            }
            KeyCode::Char('-') => self.bpm = self.bpm.saturating_sub(5).max(30),
            KeyCode::Left if self.mode == Mode::Scales => self.scale_exercise.shift_root(-1, now),
            KeyCode::Right if self.mode == Mode::Scales => self.scale_exercise.shift_root(1, now),
            KeyCode::Up if self.mode == Mode::Scales => self.scale_exercise.shift_kind(1, now),
            KeyCode::Down if self.mode == Mode::Scales => self.scale_exercise.shift_kind(-1, now),
            KeyCode::Char('h') if self.mode == Mode::Notes => {
                self.note_exercise.hide_name = !self.note_exercise.hide_name
            }
            _ => {}
        }
        false
    }

    pub fn beat_duration(&self) -> Duration {
        Duration::from_secs_f64(60.0 / f64::from(self.bpm))
    }

    pub fn session_duration(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started)
    }

    pub fn active_notes(&self) -> impl Iterator<Item = (MidiNote, NoteState)> + '_ {
        self.notes
            .iter()
            .enumerate()
            .filter(|(_, state)| state.velocity > 0)
            .map(|(value, state)| {
                (
                    MidiNote::new(value as u8).expect("array is MIDI sized"),
                    *state,
                )
            })
    }

    pub const fn keyboard_high(&self) -> u8 {
        self.keyboard_base + KEYBOARD_SPAN
    }

    pub fn metrics(&self) -> Option<&SessionMetrics> {
        match self.mode {
            Mode::Freeplay => None,
            Mode::Notes => Some(&self.note_exercise.metrics),
            Mode::Scales => Some(&self.scale_exercise.metrics),
            Mode::Rhythm => Some(&self.rhythm_metrics),
        }
    }

    fn restart(&mut self, now: Instant) {
        match self.mode {
            Mode::Freeplay => {
                self.notes = [NoteState::default(); 128];
                self.sustain = false;
            }
            Mode::Notes => self.note_exercise.restart(now),
            Mode::Scales => self.scale_exercise.rebuild(now),
            Mode::Rhythm => {
                self.rhythm_metrics = SessionMetrics::default();
                self.rhythm_epoch = now;
                self.rhythm_last_beat = 0;
            }
        }
        self.last_feedback = "restarted".into();
    }

    fn set_mode(&mut self, mode: Mode, now: Instant) {
        self.mode = mode;
        if mode == Mode::Rhythm {
            self.rhythm_epoch = now;
            self.rhythm_last_beat = 0;
        }
        self.last_feedback = format!("{mode} mode");
    }

    fn push_recent(&mut self, message: String) {
        if self.recent.len() == RECENT_EVENT_CAPACITY {
            self.recent.pop_front();
        }
        self.recent.push_back(message);
    }

    fn refresh_chord(&mut self) {
        self.chord = chord::detect(self.active_notes().map(|(note, _)| note));
    }

    fn apply_control(&mut self, action: ControlAction) {
        match action {
            ControlAction::Volume(level) => {
                self.volume = level;
                self.muted = false;
            }
            ControlAction::Synth(parameter, value) => self.patch.set(parameter, value),
            ControlAction::Bpm(bpm) => self.bpm = bpm,
        }
    }

    fn follow_keyboard_note(&mut self, note: MidiNote) {
        while note.value() < self.keyboard_base && self.keyboard_base >= 12 {
            self.keyboard_base -= 12;
        }
        while note.value() > self.keyboard_high() && self.keyboard_base < HIGHEST_KEYBOARD_BASE {
            self.keyboard_base += 12;
        }
    }
}

fn format_attempt(attempt: Attempt) -> String {
    if attempt.correct {
        format!("correct · {} ms", attempt.response_time.as_millis())
    } else {
        format!("expected {}, played {}", attempt.expected, attempt.played)
    }
}

pub fn ctrl_or_alt(key: KeyEvent) -> bool {
    key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(message: MidiMessage, at: Instant) -> MidiEvent {
        MidiEvent { message, at }
    }

    #[test]
    fn sustain_holds_visual_note_until_pedal_up() {
        let now = Instant::now();
        let mut app = App::new(now, 0.7, 100, (48, 72));
        let note = MidiNote::new(60).unwrap();
        app.handle_midi(event(
            MidiMessage::NoteOn {
                channel: 0,
                note,
                velocity: 90,
            },
            now,
        ));
        app.handle_midi(event(
            MidiMessage::Sustain {
                channel: 0,
                down: true,
                value: 127,
            },
            now,
        ));
        app.handle_midi(event(
            MidiMessage::NoteOff {
                channel: 0,
                note,
                velocity: 0,
            },
            now,
        ));
        assert_eq!(app.notes[60].velocity, 90);
        assert!(!app.notes[60].held);
        app.handle_midi(event(
            MidiMessage::Sustain {
                channel: 0,
                down: false,
                value: 0,
            },
            now,
        ));
        assert_eq!(app.notes[60].velocity, 0);
    }

    #[test]
    fn event_history_is_bounded() {
        let now = Instant::now();
        let mut app = App::new(now, 0.7, 100, (48, 72));
        for value in 40..80 {
            let note = MidiNote::new(value).unwrap();
            app.handle_midi(event(
                MidiMessage::NoteOn {
                    channel: 0,
                    note,
                    velocity: 1,
                },
                now,
            ));
        }
        assert_eq!(app.recent.len(), RECENT_EVENT_CAPACITY);
    }

    #[test]
    fn keyboard_window_follows_octave_shifted_notes() {
        let now = Instant::now();
        let mut app = App::new(now, 0.7, 100, (48, 72));
        let note_up = MidiNote::new(74).unwrap();
        app.handle_midi(event(
            MidiMessage::NoteOn {
                channel: 0,
                note: note_up,
                velocity: 90,
            },
            now,
        ));
        assert_eq!((app.keyboard_base, app.keyboard_high()), (60, 84));

        let note_down = MidiNote::new(47).unwrap();
        app.handle_midi(event(
            MidiMessage::NoteOn {
                channel: 0,
                note: note_down,
                velocity: 90,
            },
            now,
        ));
        assert_eq!((app.keyboard_base, app.keyboard_high()), (36, 60));
    }

    #[test]
    fn notes_on_visible_boundaries_do_not_move_keyboard_window() {
        let now = Instant::now();
        let mut app = App::new(now, 0.7, 100, (48, 72));
        for value in [48, 72] {
            let note = MidiNote::new(value).unwrap();
            app.handle_midi(event(
                MidiMessage::NoteOn {
                    channel: 0,
                    note,
                    velocity: 90,
                },
                now,
            ));
        }
        assert_eq!((app.keyboard_base, app.keyboard_high()), (48, 72));
    }

    #[test]
    fn mapped_knobs_update_patch_volume_and_tempo() {
        let now = Instant::now();
        let mut app = App::new(now, 0.7, 100, (48, 72));
        app.handle_midi(event(
            MidiMessage::ControlChange {
                channel: 0,
                controller: 2,
                value: 127,
            },
            now,
        ));
        assert_eq!(app.patch.attack_seconds, 1.5);
        assert!(app.recent.back().unwrap().contains("K2 attack"));

        app.handle_midi(event(
            MidiMessage::ControlChange {
                channel: 0,
                controller: 1,
                value: 0,
            },
            now,
        ));
        assert_eq!(app.volume, 0.0);
        app.handle_midi(event(
            MidiMessage::ControlChange {
                channel: 0,
                controller: 8,
                value: 127,
            },
            now,
        ));
        assert_eq!(app.bpm, 240);
    }
}
