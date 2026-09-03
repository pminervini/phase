use crate::music::{MidiNote, PitchClass};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChordQuality {
    Major,
    Minor,
    Diminished,
    Augmented,
    DominantSeventh,
    MajorSeventh,
    MinorSeventh,
}

impl fmt::Display for ChordQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Diminished => "diminished",
            Self::Augmented => "augmented",
            Self::DominantSeventh => "7",
            Self::MajorSeventh => "maj7",
            Self::MinorSeventh => "min7",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Chord {
    pub root: PitchClass,
    pub quality: ChordQuality,
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.root, self.quality)
    }
}

const PATTERNS: &[(ChordQuality, &[u8])] = &[
    (ChordQuality::DominantSeventh, &[0, 4, 7, 10]),
    (ChordQuality::MajorSeventh, &[0, 4, 7, 11]),
    (ChordQuality::MinorSeventh, &[0, 3, 7, 10]),
    (ChordQuality::Major, &[0, 4, 7]),
    (ChordQuality::Minor, &[0, 3, 7]),
    (ChordQuality::Diminished, &[0, 3, 6]),
    (ChordQuality::Augmented, &[0, 4, 8]),
];

pub fn detect(notes: impl IntoIterator<Item = MidiNote>) -> Option<Chord> {
    let present: BTreeSet<u8> = notes.into_iter().map(|n| n.pitch_class().value()).collect();
    if present.len() < 3 || present.len() > 4 {
        return None;
    }
    for root in 0..12 {
        for &(quality, pattern) in PATTERNS {
            if pattern.len() != present.len() {
                continue;
            }
            let expected: BTreeSet<u8> = pattern.iter().map(|i| (root + i) % 12).collect();
            if expected == present {
                return Some(Chord {
                    root: PitchClass::new(root),
                    quality,
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes(values: &[u8]) -> Vec<MidiNote> {
        values.iter().map(|&n| MidiNote::new(n).unwrap()).collect()
    }

    #[test]
    fn detects_triad_in_any_inversion() {
        let root = detect(notes(&[60, 64, 67])).unwrap();
        let inversion = detect(notes(&[64, 67, 72])).unwrap();
        assert_eq!(root, inversion);
        assert_eq!(root.quality, ChordQuality::Major);
    }

    #[test]
    fn detects_required_chord_qualities() {
        let cases = [
            (&[60, 63, 67][..], ChordQuality::Minor),
            (&[60, 63, 66], ChordQuality::Diminished),
            (&[60, 64, 68], ChordQuality::Augmented),
            (&[60, 64, 67, 70], ChordQuality::DominantSeventh),
            (&[60, 64, 67, 71], ChordQuality::MajorSeventh),
            (&[60, 63, 67, 70], ChordQuality::MinorSeventh),
        ];
        for (values, expected) in cases {
            assert_eq!(detect(notes(values)).unwrap().quality, expected);
        }
    }
}
