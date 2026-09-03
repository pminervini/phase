use crate::music::{MidiNote, PitchClass, Scale, ScaleKind};
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

pub const RHYTHM_PERFECT_MS: i64 = 35;
pub const RHYTHM_GOOD_MS: i64 = 80;
pub const RHYTHM_HIT_WINDOW_MS: i64 = 180;
pub const STAFF_LINE_LENGTH: usize = 7;
pub const STAFF_SONG_MENU_COLUMNS: usize = 3;
const STAFF_COMPLETE_HOLD: Duration = Duration::from_millis(700);
pub const TWINKLE_TREBLE: [u8; 42] = [
    67, 67, 74, 74, 76, 76, 74, // Twinkle, twinkle, little star
    72, 72, 71, 71, 69, 69, 67, // How I wonder what you are
    74, 74, 72, 72, 71, 71, 69, // Up above the world so high
    74, 74, 72, 72, 71, 71, 69, // Like a diamond in the sky
    67, 67, 74, 74, 76, 76, 74, // Twinkle, twinkle, little star
    72, 72, 71, 71, 69, 69, 67, // How I wonder what you are
];
const MARYS_LAMB_TREBLE: [u8; 26] = [
    71, 69, 67, 69, 71, 71, 71, // Mary had a little lamb
    69, 69, 69, 71, 74, 74, // Little lamb, little lamb
    71, 69, 67, 69, 71, 71, 71, 71, // Mary had a little lamb
    69, 69, 71, 69, 67, // Its fleece was white as snow
];
const FRERE_JACQUES_TREBLE: [u8; 32] = [
    67, 69, 71, 67, 67, 69, 71, 67, // Frère Jacques, dormez-vous?
    71, 72, 74, 71, 72, 74, // Sonnez les matines
    74, 76, 74, 72, 71, 67, 74, 76, 74, 72, 71, 67, // Ding, dang, dong
    67, 74, 67, 67, 74, 67,
];
const ODE_TO_JOY_TREBLE: [u8; 30] = [
    71, 71, 72, 74, 74, 72, 71, 69, 67, 67, 69, 71, 71, 69, 69, // First phrase
    71, 71, 72, 74, 74, 72, 71, 69, 67, 67, 69, 71, 69, 67, 67, // Second phrase
];
const LONDON_BRIDGE_TREBLE: &[u8] = &[
    74, 76, 74, 72, 71, 72, 74, 69, 71, 72, 71, 72, 74, 74, 76, 74, 72, 71, 72, 74, 69, 74, 71, 67,
];
const HOT_CROSS_BUNS_TREBLE: &[u8] = &[
    71, 69, 67, 71, 69, 67, 67, 67, 67, 67, 69, 69, 69, 69, 71, 69, 67,
];
const THREE_BLIND_MICE_TREBLE: &[u8] = &[
    71, 69, 67, 71, 69, 67, 74, 72, 71, 74, 72, 71, 74, 74, 72, 71, 74, 74, 72, 71, 71, 69, 67,
];
const ROW_YOUR_BOAT_TREBLE: &[u8] = &[
    65, 65, 65, 67, 69, 69, 67, 69, 70, 72, 77, 77, 77, 72, 72, 72, 69, 69, 69, 65, 65, 65, 72, 70,
    69, 67, 65,
];
const THIS_OLD_MAN_TREBLE: &[u8] = &[
    74, 71, 74, 74, 71, 74, 74, 76, 74, 72, 71, 69, 71, 72, 71, 72, 74, 67, 67, 67, 67, 67, 69, 71,
    72, 74, 74, 69, 69, 72, 71, 69, 67,
];
const SKIP_TO_MY_LOU_TREBLE: &[u8] = &[
    74, 71, 74, 74, 71, 74, 76, 72, 76, 76, 72, 76, 74, 71, 74, 74, 71, 74, 72, 71, 69, 67,
];
const AU_CLAIR_DE_LA_LUNE_TREBLE: &[u8] = &[
    67, 67, 67, 69, 71, 69, 67, 71, 69, 69, 67, 67, 67, 67, 69, 71, 69, 67, 71, 69, 69, 67,
];
const LIGHTLY_ROW_TREBLE: &[u8] = &[
    74, 71, 71, 72, 69, 69, 67, 69, 71, 72, 74, 74, 74, 74, 71, 71, 72, 69, 69, 67, 71, 74, 74, 67,
];
const YANKEE_DOODLE_TREBLE: &[u8] = &[
    72, 72, 74, 76, 72, 76, 74, 72, 72, 74, 76, 72, 71, 72, 72, 74, 76, 77, 76, 74, 72, 71, 67, 69,
    71, 72, 72,
];
const AMAZING_GRACE_TREBLE: &[u8] = &[
    64, 69, 73, 69, 73, 71, 69, 66, 64, 64, 69, 73, 69, 73, 71, 76, 73, 76, 73, 69,
];
const JINGLE_BELLS_TREBLE: &[u8] = &[
    71, 71, 71, 71, 71, 71, 71, 74, 67, 69, 71, 72, 72, 72, 72, 72, 71, 71, 71, 71, 69, 69, 71, 69,
    74, 71, 71, 71, 71, 71, 71, 71, 74, 67, 69, 71, 72, 72, 72, 72, 72, 71, 71, 74, 74, 72, 69, 67,
];
const OLD_MACDONALD_TREBLE: &[u8] = &[
    67, 67, 67, 74, 76, 76, 74, 71, 71, 69, 69, 67, 74, 67, 67, 67, 74, 76, 76, 74, 71, 71, 69, 69,
    67,
];
const HAPPY_BIRTHDAY_TREBLE: &[u8] = &[
    65, 65, 67, 65, 70, 69, 65, 65, 67, 65, 72, 70, 65, 65, 77, 74, 70, 69, 67, 75, 75, 74, 70, 72,
    70,
];
const SILENT_NIGHT_TREBLE: &[u8] = &[
    67, 69, 67, 64, 67, 69, 67, 64, 74, 74, 71, 72, 72, 67, 69, 69, 72, 71, 69, 67, 69, 67, 64,
];
const OH_SUSANNA_TREBLE: &[u8] = &[
    67, 69, 71, 74, 74, 76, 74, 71, 67, 69, 71, 71, 69, 67, 69, 67, 69, 71, 74, 74, 76, 74, 71, 67,
    69, 71, 71, 69, 69, 67,
];
const POP_GOES_THE_WEASEL_TREBLE: &[u8] = &[
    67, 67, 69, 69, 71, 74, 71, 67, 67, 67, 69, 72, 71, 67, 67, 67, 69, 69, 71, 74, 71, 67, 76, 69,
    72, 71, 67,
];
const FACCETTA_NERA_TREBLE: &[u8] = &[
    69, 69, 69, 68, 66, 64, 74, 74, 74, 73, 71, 69, 76, 76, 76, 76, 73, 73, 69, 69, 71, 71, 69, 68,
    66, 64, 64, 68, 71, 74, 74, 76, 74, 73, 71, 64, 68, 71, 74, 71, 73, 74, 76, 74, 73, 71, 69,
];

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StaffSong {
    Twinkle,
    MarysLamb,
    FrereJacques,
    OdeToJoy,
    LondonBridge,
    HotCrossBuns,
    ThreeBlindMice,
    RowYourBoat,
    ThisOldMan,
    SkipToMyLou,
    AuClairDeLaLune,
    LightlyRow,
    YankeeDoodle,
    AmazingGrace,
    JingleBells,
    OldMacDonald,
    HappyBirthday,
    SilentNight,
    OhSusanna,
    PopGoesTheWeasel,
    FaccettaNera,
}

impl StaffSong {
    pub const ALL: [Self; 21] = [
        Self::Twinkle,
        Self::MarysLamb,
        Self::FrereJacques,
        Self::OdeToJoy,
        Self::LondonBridge,
        Self::HotCrossBuns,
        Self::ThreeBlindMice,
        Self::RowYourBoat,
        Self::ThisOldMan,
        Self::SkipToMyLou,
        Self::AuClairDeLaLune,
        Self::LightlyRow,
        Self::YankeeDoodle,
        Self::AmazingGrace,
        Self::JingleBells,
        Self::OldMacDonald,
        Self::HappyBirthday,
        Self::SilentNight,
        Self::OhSusanna,
        Self::PopGoesTheWeasel,
        Self::FaccettaNera,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Twinkle => "Twinkle",
            Self::MarysLamb => "Mary's Lamb",
            Self::FrereJacques => "Frère Jacques",
            Self::OdeToJoy => "Ode to Joy",
            Self::LondonBridge => "London Bridge",
            Self::HotCrossBuns => "Hot Cross Buns",
            Self::ThreeBlindMice => "Three Blind Mice",
            Self::RowYourBoat => "Row Your Boat",
            Self::ThisOldMan => "This Old Man",
            Self::SkipToMyLou => "Skip to My Lou",
            Self::AuClairDeLaLune => "Au Clair de la Lune",
            Self::LightlyRow => "Lightly Row",
            Self::YankeeDoodle => "Yankee Doodle",
            Self::AmazingGrace => "Amazing Grace",
            Self::JingleBells => "Jingle Bells",
            Self::OldMacDonald => "Old MacDonald",
            Self::HappyBirthday => "Happy Birthday",
            Self::SilentNight => "Silent Night",
            Self::OhSusanna => "Oh! Susanna",
            Self::PopGoesTheWeasel => "Pop Goes the Weasel",
            Self::FaccettaNera => "Faccetta Nera",
        }
    }

    pub const fn treble_notes(self) -> &'static [u8] {
        match self {
            Self::Twinkle => &TWINKLE_TREBLE,
            Self::MarysLamb => &MARYS_LAMB_TREBLE,
            Self::FrereJacques => &FRERE_JACQUES_TREBLE,
            Self::OdeToJoy => &ODE_TO_JOY_TREBLE,
            Self::LondonBridge => LONDON_BRIDGE_TREBLE,
            Self::HotCrossBuns => HOT_CROSS_BUNS_TREBLE,
            Self::ThreeBlindMice => THREE_BLIND_MICE_TREBLE,
            Self::RowYourBoat => ROW_YOUR_BOAT_TREBLE,
            Self::ThisOldMan => THIS_OLD_MAN_TREBLE,
            Self::SkipToMyLou => SKIP_TO_MY_LOU_TREBLE,
            Self::AuClairDeLaLune => AU_CLAIR_DE_LA_LUNE_TREBLE,
            Self::LightlyRow => LIGHTLY_ROW_TREBLE,
            Self::YankeeDoodle => YANKEE_DOODLE_TREBLE,
            Self::AmazingGrace => AMAZING_GRACE_TREBLE,
            Self::JingleBells => JINGLE_BELLS_TREBLE,
            Self::OldMacDonald => OLD_MACDONALD_TREBLE,
            Self::HappyBirthday => HAPPY_BIRTHDAY_TREBLE,
            Self::SilentNight => SILENT_NIGHT_TREBLE,
            Self::OhSusanna => OH_SUSANNA_TREBLE,
            Self::PopGoesTheWeasel => POP_GOES_THE_WEASEL_TREBLE,
            Self::FaccettaNera => FACCETTA_NERA_TREBLE,
        }
    }

    pub fn transpose_for(self, clef: StaffClef) -> i8 {
        match clef {
            StaffClef::Treble => 0,
            StaffClef::Bass => {
                let lowest = self
                    .treble_notes()
                    .iter()
                    .copied()
                    .min()
                    .expect("staff songs contain notes");
                43 - lowest as i8
            }
        }
    }
}

impl StaffClef {
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
    pub song: StaffSong,
    pub sequence: Vec<MidiNote>,
    pub index: usize,
    pub metrics: SessionMetrics,
    pub completed: u32,
    pub last_completion: Option<Duration>,
    target_since: Instant,
    song_started: Instant,
    complete_since: Option<Instant>,
}

impl StaffExercise {
    pub fn new(now: Instant) -> Self {
        let mut exercise = Self {
            clef: StaffClef::Treble,
            song: StaffSong::Twinkle,
            sequence: Vec::with_capacity(TWINKLE_TREBLE.len()),
            index: 0,
            metrics: SessionMetrics::default(),
            completed: 0,
            last_completion: None,
            target_since: now,
            song_started: now,
            complete_since: None,
        };
        exercise.load_song(now);
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
                self.last_completion = Some(now.saturating_duration_since(self.song_started));
                self.complete_since = Some(now);
            }
        }
        Some(attempt)
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        if self.complete_since.is_some_and(|completed| {
            now.saturating_duration_since(completed) >= STAFF_COMPLETE_HOLD
        }) {
            self.load_song(now);
            true
        } else {
            false
        }
    }

    pub fn restart(&mut self, now: Instant) {
        self.metrics = SessionMetrics::default();
        self.completed = 0;
        self.last_completion = None;
        self.load_song(now);
    }

    pub fn resume(&mut self, now: Instant) {
        if self.complete_since.is_some() {
            self.load_song(now);
        } else {
            self.target_since = now;
            self.song_started = now;
        }
    }

    pub fn toggle_clef(&mut self, now: Instant) {
        self.clef = self.clef.toggle();
        self.load_song(now);
    }

    pub fn select_song(&mut self, song: StaffSong, now: Instant) {
        self.song = song;
        self.load_song(now);
    }

    fn load_song(&mut self, now: Instant) {
        let transpose = self.song.transpose_for(self.clef);
        self.sequence = self
            .song
            .treble_notes()
            .iter()
            .map(|value| {
                MidiNote::new(*value)
                    .expect("staff melody contains valid MIDI notes")
                    .transpose(transpose)
                    .expect("clef transposition remains in the MIDI range")
            })
            .collect();
        self.index = 0;
        self.target_since = now;
        self.song_started = now;
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
        let mut exercise = StaffExercise::new(now);
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
        let mut exercise = StaffExercise::new(now);
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
    fn staff_clef_uses_twinkle_twinkle_in_the_staff_register() {
        let now = Instant::now();
        let mut exercise = StaffExercise::new(now);
        assert_eq!(exercise.sequence.len(), 42);
        assert_eq!(
            exercise.sequence[..STAFF_LINE_LENGTH]
                .iter()
                .map(|note| note.value())
                .collect::<Vec<_>>(),
            vec![67, 67, 74, 74, 76, 76, 74]
        );
        let lines = exercise
            .sequence
            .chunks(STAFF_LINE_LENGTH)
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 6);
        assert_eq!(
            lines[1].iter().map(|note| note.value()).collect::<Vec<_>>(),
            vec![72, 72, 71, 71, 69, 69, 67]
        );
        assert_eq!(lines[2], lines[3]);
        assert_eq!(lines[0], lines[4]);
        assert_eq!(lines[1], lines[5]);

        exercise.toggle_clef(now);
        assert_eq!(exercise.clef, StaffClef::Bass);
        assert_eq!(
            exercise.sequence[..STAFF_LINE_LENGTH]
                .iter()
                .map(|note| note.value())
                .collect::<Vec<_>>(),
            vec![43, 43, 50, 50, 52, 52, 50]
        );
    }

    #[test]
    fn staff_song_selection_loads_each_beginner_melody() {
        let now = Instant::now();
        let mut exercise = StaffExercise::new(now);
        let expected = [
            (StaffSong::MarysLamb, 26, vec![71, 69, 67, 69]),
            (StaffSong::FrereJacques, 32, vec![67, 69, 71, 67]),
            (StaffSong::OdeToJoy, 30, vec![71, 71, 72, 74]),
            (StaffSong::FaccettaNera, 47, vec![69, 69, 69, 68]),
        ];

        for (song, length, opening) in expected {
            exercise.select_song(song, now);
            assert_eq!(exercise.song, song);
            assert_eq!(exercise.sequence.len(), length);
            assert_eq!(
                exercise.sequence[..opening.len()]
                    .iter()
                    .map(|note| note.value())
                    .collect::<Vec<_>>(),
                opening
            );
            assert!(
                exercise
                    .sequence
                    .iter()
                    .all(|note| (64..=77).contains(&note.value()))
            );
        }

        exercise.select_song(StaffSong::Twinkle, now);
        assert_eq!(exercise.song, StaffSong::Twinkle);
    }

    #[test]
    fn staff_library_has_twenty_one_distinct_playable_songs() {
        let now = Instant::now();
        let mut exercise = StaffExercise::new(now);
        let labels = StaffSong::ALL
            .iter()
            .map(|song| song.label())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(StaffSong::ALL.len(), 21);
        assert_eq!(labels.len(), StaffSong::ALL.len());

        for song in StaffSong::ALL {
            exercise.clef = StaffClef::Treble;
            exercise.select_song(song, now);
            assert!(!exercise.sequence.is_empty(), "{} is empty", song.label());
            assert!(
                exercise
                    .sequence
                    .iter()
                    .all(|note| (64..=77).contains(&note.value())),
                "{} leaves the treble staff",
                song.label()
            );

            exercise.clef = StaffClef::Bass;
            exercise.select_song(song, now);
            assert!(
                exercise
                    .sequence
                    .iter()
                    .all(|note| (43..=57).contains(&note.value())),
                "{} leaves the bass staff",
                song.label()
            );
        }
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
