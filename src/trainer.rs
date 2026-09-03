use crate::music::{MidiNote, PitchClass, Scale, ScaleKind};
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

pub const RHYTHM_PERFECT_MS: i64 = 35;
pub const RHYTHM_GOOD_MS: i64 = 80;
pub const RHYTHM_HIT_WINDOW_MS: i64 = 180;
pub const STAFF_PHRASE_LENGTH: usize = 8;
const STAFF_COMPLETE_HOLD: Duration = Duration::from_millis(700);

#[derive(Clone, Copy, Debug)]
pub struct Attempt {
    pub played: MidiNote,
    pub expected: MidiNote,
    pub correct: bool,
    pub response_time: Duration,
    pub timing_offset_ms: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct SessionMetrics {
    pub attempts: u64,
    pub correct: u64,
    pub streak: u32,
    pub best_streak: u32,
    pub response_time_total: Duration,
    pub timing_abs_total_ms: u64,
    pub timed_attempts: u64,
}

impl SessionMetrics {
    pub fn record(&mut self, attempt: Attempt) {
        self.attempts += 1;
        self.response_time_total += attempt.response_time;
        if attempt.correct {
            self.correct += 1;
            self.streak += 1;
            self.best_streak = self.best_streak.max(self.streak);
        } else {
            self.streak = 0;
        }
        if let Some(offset) = attempt.timing_offset_ms {
            self.timing_abs_total_ms += offset.unsigned_abs();
            self.timed_attempts += 1;
        }
    }

    pub fn accuracy(&self) -> f32 {
        if self.attempts == 0 {
            0.0
        } else {
            self.correct as f32 * 100.0 / self.attempts as f32
        }
    }

    pub fn mean_timing_error_ms(&self) -> Option<f32> {
        (self.timed_attempts > 0)
            .then(|| self.timing_abs_total_ms as f32 / self.timed_attempts as f32)
    }

    pub fn mean_response_time_ms(&self) -> Option<f32> {
        (self.attempts > 0)
            .then(|| self.response_time_total.as_secs_f32() * 1_000.0 / self.attempts as f32)
    }
}

pub fn score_note(played: MidiNote, expected: MidiNote, elapsed: Duration) -> Attempt {
    Attempt {
        played,
        expected,
        correct: played == expected,
        response_time: elapsed,
        timing_offset_ms: None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RhythmGrade {
    Perfect,
    Good,
    Early,
    Late,
    Miss,
}

impl std::fmt::Display for RhythmGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Perfect => "perfect",
            Self::Good => "good",
            Self::Early => "early",
            Self::Late => "late",
            Self::Miss => "miss",
        })
    }
}

pub fn classify_rhythm(offset_ms: i64) -> RhythmGrade {
    let absolute = offset_ms.abs();
    if absolute <= RHYTHM_PERFECT_MS {
        RhythmGrade::Perfect
    } else if absolute <= RHYTHM_GOOD_MS {
        RhythmGrade::Good
    } else if absolute > RHYTHM_HIT_WINDOW_MS {
        RhythmGrade::Miss
    } else if offset_ms < 0 {
        RhythmGrade::Early
    } else {
        RhythmGrade::Late
    }
}

pub struct NoteExercise {
    pub target: MidiNote,
    pub range: (u8, u8),
    pub hide_name: bool,
    pub metrics: SessionMetrics,
    pub weak_notes: BTreeMap<u8, u64>,
    target_since: Instant,
    rng: u64,
}

impl NoteExercise {
    pub fn new(range: (u8, u8), now: Instant) -> Self {
        let range = normalized_range(range);
        let mut exercise = Self {
            target: MidiNote::new(range.0).expect("normalized MIDI range"),
            range,
            hide_name: false,
            metrics: SessionMetrics::default(),
            weak_notes: BTreeMap::new(),
            target_since: now,
            rng: now.elapsed().as_nanos() as u64 ^ 0x0050_4841_5345,
        };
        exercise.next(now);
        exercise
    }

    pub fn attempt(&mut self, played: MidiNote, now: Instant) -> Attempt {
        let attempt = score_note(
            played,
            self.target,
            now.saturating_duration_since(self.target_since),
        );
        self.metrics.record(attempt);
        if attempt.correct {
            self.next(now);
        } else {
            *self.weak_notes.entry(self.target.value()).or_default() += 1;
        }
        attempt
    }

    pub fn restart(&mut self, now: Instant) {
        self.metrics = SessionMetrics::default();
        self.next(now);
    }

    fn next(&mut self, now: Instant) {
        let previous = self.target.value();
        let width = u64::from(self.range.1 - self.range.0) + 1;
        for _ in 0..4 {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 7;
            self.rng ^= self.rng << 17;
            let value = self.range.0 + (self.rng % width) as u8;
            if value != previous || width == 1 {
                self.target = MidiNote::new(value).expect("value within normalized MIDI range");
                break;
            }
        }
        self.target_since = now;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StaffClef {
    Treble,
    Bass,
}

impl StaffClef {
    pub const fn midi_range(self) -> (u8, u8) {
        match self {
            // The five treble lines, E4 through F5.
            Self::Treble => (64, 77),
            // The five bass lines, G2 through A3.
            Self::Bass => (43, 57),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Treble => "treble",
            Self::Bass => "bass",
        }
    }

    pub const fn toggle(self) -> Self {
        match self {
            Self::Treble => Self::Bass,
            Self::Bass => Self::Treble,
        }
    }
}

pub struct StaffExercise {
    pub clef: StaffClef,
    pub sequence: Vec<MidiNote>,
    pub index: usize,
    pub metrics: SessionMetrics,
    pub completed: u32,
    pub last_completion: Option<Duration>,
    training_range: (u8, u8),
    target_since: Instant,
    phrase_started: Instant,
    complete_since: Option<Instant>,
    rng: u64,
}

impl StaffExercise {
    pub fn new(range: (u8, u8), now: Instant) -> Self {
        let mut exercise = Self {
            clef: StaffClef::Treble,
            sequence: Vec::with_capacity(STAFF_PHRASE_LENGTH),
            index: 0,
            metrics: SessionMetrics::default(),
            completed: 0,
            last_completion: None,
            training_range: normalized_range(range),
            target_since: now,
            phrase_started: now,
            complete_since: None,
            rng: now.elapsed().as_nanos() as u64 ^ 0x5354_4146_465f_4e4f,
        };
        exercise.new_phrase(now);
        exercise
    }

    pub fn expected(&self) -> Option<MidiNote> {
        self.sequence.get(self.index).copied()
    }

    pub fn attempt(&mut self, played: MidiNote, now: Instant) -> Option<Attempt> {
        let expected = self.expected()?;
        let attempt = score_note(
            played,
            expected,
            now.saturating_duration_since(self.target_since),
        );
        self.metrics.record(attempt);
        if attempt.correct {
            self.index += 1;
            self.target_since = now;
            if self.index == self.sequence.len() {
                self.completed += 1;
                self.last_completion = Some(now.saturating_duration_since(self.phrase_started));
                self.complete_since = Some(now);
            }
        }
        Some(attempt)
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        if self.complete_since.is_some_and(|completed| {
            now.saturating_duration_since(completed) >= STAFF_COMPLETE_HOLD
        }) {
            self.new_phrase(now);
            true
        } else {
            false
        }
    }

    pub fn restart(&mut self, now: Instant) {
        self.metrics = SessionMetrics::default();
        self.completed = 0;
        self.last_completion = None;
        self.new_phrase(now);
    }

    pub fn resume(&mut self, now: Instant) {
        if self.complete_since.is_some() {
            self.new_phrase(now);
        } else {
            self.target_since = now;
            self.phrase_started = now;
        }
    }

    pub fn toggle_clef(&mut self, now: Instant) {
        self.clef = self.clef.toggle();
        self.new_phrase(now);
    }

    fn new_phrase(&mut self, now: Instant) {
        self.sequence.clear();
        let staff_range = self.clef.midi_range();
        let low = self.training_range.0.max(staff_range.0);
        let high = self.training_range.1.min(staff_range.1);
        let (low, high) = if low <= high {
            (low, high)
        } else {
            staff_range
        };
        let width = u64::from(high - low) + 1;
        while self.sequence.len() < STAFF_PHRASE_LENGTH {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 7;
            self.rng ^= self.rng << 17;
            let value = low + (self.rng % width) as u8;
            if self
                .sequence
                .last()
                .is_some_and(|note| note.value() == value)
                && width > 1
            {
                continue;
            }
            self.sequence
                .push(MidiNote::new(value).expect("staff range contains valid MIDI notes"));
        }
        self.index = 0;
        self.target_since = now;
        self.phrase_started = now;
        self.complete_since = None;
    }
}

pub struct ScaleExercise {
    pub root: PitchClass,
    pub kind: ScaleKind,
    pub sequence: Vec<MidiNote>,
    pub index: usize,
    pub metrics: SessionMetrics,
    pub completed: u32,
    pub recent_mistakes: VecDeque<MidiNote>,
    pub last_completion: Option<Duration>,
    started: Instant,
}

impl ScaleExercise {
    pub fn new(now: Instant) -> Self {
        let mut exercise = Self {
            root: PitchClass::C,
            kind: ScaleKind::Major,
            sequence: Vec::new(),
            index: 0,
            metrics: SessionMetrics::default(),
            completed: 0,
            recent_mistakes: VecDeque::with_capacity(4),
            last_completion: None,
            started: now,
        };
        exercise.rebuild(now);
        exercise
    }

    pub fn expected(&self) -> MidiNote {
        self.sequence[self.index]
    }

    pub fn attempt(&mut self, played: MidiNote, now: Instant) -> Attempt {
        let attempt = score_note(
            played,
            self.expected(),
            now.saturating_duration_since(self.started),
        );
        self.metrics.record(attempt);
        if attempt.correct {
            self.index += 1;
            if self.index == self.sequence.len() {
                self.completed += 1;
                self.last_completion = Some(now.saturating_duration_since(self.started));
                self.index = 0;
                self.started = now;
            }
        } else {
            if self.recent_mistakes.len() == 4 {
                self.recent_mistakes.pop_front();
            }
            self.recent_mistakes.push_back(played);
        }
        attempt
    }

    pub fn shift_root(&mut self, amount: i8, now: Instant) {
        self.root = self.root.transpose(amount);
        self.rebuild(now);
    }

    pub fn shift_kind(&mut self, amount: i8, now: Instant) {
        let current = ScaleKind::ALL
            .iter()
            .position(|kind| *kind == self.kind)
            .unwrap_or(0) as i8;
        let index = (current + amount).rem_euclid(ScaleKind::ALL.len() as i8) as usize;
        self.kind = ScaleKind::ALL[index];
        self.rebuild(now);
    }

    pub fn rebuild(&mut self, now: Instant) {
        let root_note = MidiNote::new(60 + self.root.value()).expect("one octave above middle C");
        self.sequence = Scale {
            root: root_note,
            kind: self.kind,
        }
        .ascending_descending();
        self.index = 0;
        self.recent_mistakes.clear();
        self.started = now;
    }
}

pub fn nearest_beat_offset(now: Instant, epoch: Instant, beat: Duration) -> i64 {
    let elapsed = now.saturating_duration_since(epoch).as_secs_f64();
    let beat_secs = beat.as_secs_f64();
    let nearest = (elapsed / beat_secs).round() * beat_secs;
    ((elapsed - nearest) * 1_000.0).round() as i64
}

fn normalized_range(range: (u8, u8)) -> (u8, u8) {
    (range.0.min(range.1).min(127), range.0.max(range.1).min(127))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_scoring_tracks_accuracy_and_streak() {
        let c = MidiNote::new(60).unwrap();
        let d = MidiNote::new(62).unwrap();
        let mut metrics = SessionMetrics::default();
        metrics.record(score_note(c, c, Duration::from_millis(400)));
        metrics.record(score_note(d, c, Duration::from_millis(500)));
        assert_eq!(metrics.correct, 1);
        assert_eq!(metrics.accuracy(), 50.0);
        assert_eq!(metrics.streak, 0);
        assert_eq!(metrics.best_streak, 1);
    }

    #[test]
    fn staff_progress_advances_only_for_the_expected_note() {
        let now = Instant::now();
        let mut exercise = StaffExercise::new((48, 72), now);
        exercise.sequence = vec![MidiNote::new(64).unwrap(), MidiNote::new(67).unwrap()];
        exercise.index = 0;

        let miss = exercise
            .attempt(MidiNote::new(65).unwrap(), now + Duration::from_millis(100))
            .unwrap();
        assert!(!miss.correct);
        assert_eq!(exercise.index, 0);

        let hit = exercise
            .attempt(MidiNote::new(64).unwrap(), now + Duration::from_millis(200))
            .unwrap();
        assert!(hit.correct);
        assert_eq!(exercise.index, 1);
        assert_eq!(exercise.expected(), Some(MidiNote::new(67).unwrap()));
        assert_eq!(exercise.metrics.attempts, 2);
    }

    #[test]
    fn completed_staff_phrase_remains_visible_before_refreshing() {
        let now = Instant::now();
        let mut exercise = StaffExercise::new((48, 72), now);
        exercise.sequence = vec![MidiNote::new(64).unwrap()];
        exercise.index = 0;
        exercise.attempt(MidiNote::new(64).unwrap(), now).unwrap();

        assert!(exercise.expected().is_none());
        assert!(!exercise.tick(now + STAFF_COMPLETE_HOLD - Duration::from_millis(1)));
        assert_eq!(exercise.index, 1);
        assert!(exercise.tick(now + STAFF_COMPLETE_HOLD));
        assert_eq!(exercise.index, 0);
        assert!(exercise.expected().is_some());
    }

    #[test]
    fn staff_clef_uses_notes_that_fit_on_its_five_lines() {
        let now = Instant::now();
        let mut exercise = StaffExercise::new((48, 72), now);
        assert!(
            exercise
                .sequence
                .iter()
                .all(|note| (64..=72).contains(&note.value()))
        );

        exercise.toggle_clef(now);
        assert_eq!(exercise.clef, StaffClef::Bass);
        assert!(
            exercise
                .sequence
                .iter()
                .all(|note| (48..=57).contains(&note.value()))
        );
    }

    #[test]
    fn timing_classification_uses_central_thresholds() {
        assert_eq!(classify_rhythm(0), RhythmGrade::Perfect);
        assert_eq!(classify_rhythm(-35), RhythmGrade::Perfect);
        assert_eq!(classify_rhythm(70), RhythmGrade::Good);
        assert_eq!(classify_rhythm(-100), RhythmGrade::Early);
        assert_eq!(classify_rhythm(100), RhythmGrade::Late);
        assert_eq!(classify_rhythm(181), RhythmGrade::Miss);
    }

    #[test]
    fn nearest_beat_offset_is_signed() {
        let epoch = Instant::now();
        assert_eq!(
            nearest_beat_offset(
                epoch + Duration::from_millis(480),
                epoch,
                Duration::from_millis(500)
            ),
            -20
        );
        assert_eq!(
            nearest_beat_offset(
                epoch + Duration::from_millis(540),
                epoch,
                Duration::from_millis(500)
            ),
            40
        );
    }
}
