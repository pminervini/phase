use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct MidiNote(u8);

impl MidiNote {
    pub const fn new(value: u8) -> Option<Self> {
        if value <= 127 {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    pub const fn pitch_class(self) -> PitchClass {
        PitchClass(self.0 % 12)
    }

    pub const fn octave(self) -> i8 {
        (self.0 / 12) as i8 - 1
    }

    pub fn frequency(self) -> f32 {
        440.0 * 2.0_f32.powf((f32::from(self.0) - 69.0) / 12.0)
    }

    pub fn transpose(self, semitones: i8) -> Option<Self> {
        let value = i16::from(self.0) + i16::from(semitones);
        (0..=127).contains(&value).then_some(Self(value as u8))
    }
}

impl fmt::Display for MidiNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.pitch_class(), self.octave())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum NoteName {
    C,
    Cs,
    D,
    Ds,
    E,
    F,
    Fs,
    G,
    Gs,
    A,
    As,
    B,
}

impl NoteName {
    pub const ALL: [Self; 12] = [
        Self::C,
        Self::Cs,
        Self::D,
        Self::Ds,
        Self::E,
        Self::F,
        Self::Fs,
        Self::G,
        Self::Gs,
        Self::A,
        Self::As,
        Self::B,
    ];
}

impl fmt::Display for NoteName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::C => "C",
            Self::Cs => "C#",
            Self::D => "D",
            Self::Ds => "D#",
            Self::E => "E",
            Self::F => "F",
            Self::Fs => "F#",
            Self::G => "G",
            Self::Gs => "G#",
            Self::A => "A",
            Self::As => "A#",
            Self::B => "B",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PitchClass(u8);

impl PitchClass {
    pub const C: Self = Self(0);

    pub const fn new(value: u8) -> Self {
        Self(value % 12)
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    pub const fn name(self) -> NoteName {
        NoteName::ALL[self.0 as usize]
    }

    pub const fn transpose(self, semitones: i8) -> Self {
        Self(((self.0 as i16 + semitones as i16).rem_euclid(12)) as u8)
    }
}

impl fmt::Display for PitchClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name().fmt(f)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ScaleKind {
    Chromatic,
    Major,
    NaturalMinor,
    MajorPentatonic,
    MinorPentatonic,
}

impl ScaleKind {
    pub const ALL: [Self; 5] = [
        Self::Chromatic,
        Self::Major,
        Self::NaturalMinor,
        Self::MajorPentatonic,
        Self::MinorPentatonic,
    ];

    pub const fn intervals(self) -> &'static [u8] {
        match self {
            Self::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            Self::Major => &[0, 2, 4, 5, 7, 9, 11, 12],
            Self::NaturalMinor => &[0, 2, 3, 5, 7, 8, 10, 12],
            Self::MajorPentatonic => &[0, 2, 4, 7, 9, 12],
            Self::MinorPentatonic => &[0, 3, 5, 7, 10, 12],
        }
    }
}

impl fmt::Display for ScaleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Chromatic => "chromatic",
            Self::Major => "major",
            Self::NaturalMinor => "natural minor",
            Self::MajorPentatonic => "major pentatonic",
            Self::MinorPentatonic => "minor pentatonic",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scale {
    pub root: MidiNote,
    pub kind: ScaleKind,
}

impl Scale {
    pub fn ascending(&self) -> Vec<MidiNote> {
        self.kind
            .intervals()
            .iter()
            .filter_map(|&interval| self.root.transpose(interval as i8))
            .collect()
    }

    pub fn ascending_descending(&self) -> Vec<MidiNote> {
        let mut notes = self.ascending();
        let descending: Vec<_> = notes.iter().rev().skip(1).copied().collect();
        notes.extend(descending);
        notes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_names_and_octaves_follow_midi_standard() {
        assert_eq!(MidiNote::new(60).unwrap().to_string(), "C4");
        assert_eq!(MidiNote::new(69).unwrap().to_string(), "A4");
        assert_eq!(MidiNote::new(0).unwrap().to_string(), "C-1");
    }

    #[test]
    fn frequency_is_a440_and_finite() {
        assert!((MidiNote::new(69).unwrap().frequency() - 440.0).abs() < 0.001);
        assert!((MidiNote::new(60).unwrap().frequency() - 261.625_55).abs() < 0.01);
        assert!((0..=127).all(|n| MidiNote::new(n).unwrap().frequency().is_finite()));
    }

    #[test]
    fn pitch_class_arithmetic_wraps() {
        assert_eq!(PitchClass::C.transpose(-1).name(), NoteName::B);
        assert_eq!(PitchClass::new(11).transpose(2).name(), NoteName::Cs);
    }

    #[test]
    fn midi_transposition_checks_bounds() {
        assert_eq!(
            MidiNote::new(60).unwrap().transpose(12).unwrap().value(),
            72
        );
        assert!(MidiNote::new(2).unwrap().transpose(-3).is_none());
        assert!(MidiNote::new(126).unwrap().transpose(2).is_none());
    }

    #[test]
    fn generates_major_scale_both_directions() {
        let scale = Scale {
            root: MidiNote::new(60).unwrap(),
            kind: ScaleKind::Major,
        };
        let ascending: Vec<_> = scale.ascending().into_iter().map(MidiNote::value).collect();
        assert_eq!(ascending, [60, 62, 64, 65, 67, 69, 71, 72]);
        let both = scale.ascending_descending();
        assert_eq!(both.len(), 15);
        assert_eq!(both.last().unwrap().value(), 60);
    }
}
