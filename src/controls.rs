#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SynthParameter {
    Attack,
    Decay,
    Sustain,
    Release,
    Brightness,
    Harmonics,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlAction {
    Volume(f32),
    Synth(SynthParameter, f32),
    Bpm(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PatchSettings {
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub sustain_level: f32,
    pub release_seconds: f32,
    pub brightness: f32,
    pub harmonic_mix: f32,
}

impl Default for PatchSettings {
    fn default() -> Self {
        Self {
            attack_seconds: 0.008,
            decay_seconds: 0.32,
            sustain_level: 0.52,
            release_seconds: 0.28,
            brightness: 0.5,
            harmonic_mix: 0.5,
        }
    }
}

impl PatchSettings {
    pub fn sanitize(mut self) -> Self {
        let defaults = Self::default();
        self.attack_seconds =
            finite_or(self.attack_seconds, defaults.attack_seconds).clamp(0.002, 1.5);
        self.decay_seconds = finite_or(self.decay_seconds, defaults.decay_seconds).clamp(0.03, 3.0);
        self.sustain_level = finite_or(self.sustain_level, defaults.sustain_level).clamp(0.0, 1.0);
        self.release_seconds =
            finite_or(self.release_seconds, defaults.release_seconds).clamp(0.03, 4.0);
        self.brightness = finite_or(self.brightness, defaults.brightness).clamp(0.0, 1.0);
        self.harmonic_mix = finite_or(self.harmonic_mix, defaults.harmonic_mix).clamp(0.0, 1.0);
        self
    }

    pub fn set(&mut self, parameter: SynthParameter, value: f32) {
        match parameter {
            SynthParameter::Attack => self.attack_seconds = value,
            SynthParameter::Decay => self.decay_seconds = value,
            SynthParameter::Sustain => self.sustain_level = value,
            SynthParameter::Release => self.release_seconds = value,
            SynthParameter::Brightness => self.brightness = value,
            SynthParameter::Harmonics => self.harmonic_mix = value,
        }
        *self = self.sanitize();
    }
}

pub fn map_cc(controller: u8, value: u8) -> Option<ControlAction> {
    let normalized = f32::from(value.min(127)) / 127.0;
    match controller {
        1 => Some(ControlAction::Volume(normalized)),
        2 => Some(ControlAction::Synth(
            SynthParameter::Attack,
            logarithmic(0.002, 1.5, normalized),
        )),
        3 => Some(ControlAction::Synth(
            SynthParameter::Decay,
            logarithmic(0.03, 3.0, normalized),
        )),
        4 => Some(ControlAction::Synth(SynthParameter::Sustain, normalized)),
        5 => Some(ControlAction::Synth(
            SynthParameter::Release,
            logarithmic(0.03, 4.0, normalized),
        )),
        6 => Some(ControlAction::Synth(SynthParameter::Brightness, normalized)),
        7 => Some(ControlAction::Synth(SynthParameter::Harmonics, normalized)),
        8 => Some(ControlAction::Bpm(
            (40.0 + normalized * 200.0).round() as u16
        )),
        _ => None,
    }
}

pub fn describe_cc(controller: u8, value: u8) -> Option<String> {
    let action = map_cc(controller, value)?;
    let description = match action {
        ControlAction::Volume(level) => format!("volume {:.0}%", level * 100.0),
        ControlAction::Synth(parameter, amount) => match parameter {
            SynthParameter::Attack => format!("attack {}", format_time(amount)),
            SynthParameter::Decay => format!("decay {}", format_time(amount)),
            SynthParameter::Sustain => format!("sustain {:.0}%", amount * 100.0),
            SynthParameter::Release => format!("release {}", format_time(amount)),
            SynthParameter::Brightness => format!("brightness {:.0}%", amount * 100.0),
            SynthParameter::Harmonics => format!("harmonics {:.0}%", amount * 100.0),
        },
        ControlAction::Bpm(bpm) => format!("tempo {bpm} BPM"),
    };
    Some(format!("K{controller} {description}"))
}

pub fn format_time(seconds: f32) -> String {
    if seconds < 1.0 {
        format!("{:.0}ms", seconds * 1_000.0)
    } else {
        format!("{seconds:.2}s")
    }
}

fn logarithmic(minimum: f32, maximum: f32, normalized: f32) -> f32 {
    minimum * (maximum / minimum).powf(normalized)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_eight_default_knob_ccs() {
        assert_eq!(map_cc(1, 0), Some(ControlAction::Volume(0.0)));
        assert_eq!(map_cc(1, 127), Some(ControlAction::Volume(1.0)));
        assert!(matches!(
            map_cc(2, 64),
            Some(ControlAction::Synth(SynthParameter::Attack, _))
        ));
        assert!(matches!(
            map_cc(3, 64),
            Some(ControlAction::Synth(SynthParameter::Decay, _))
        ));
        assert_eq!(
            map_cc(4, 127),
            Some(ControlAction::Synth(SynthParameter::Sustain, 1.0))
        );
        assert!(matches!(
            map_cc(5, 64),
            Some(ControlAction::Synth(SynthParameter::Release, _))
        ));
        assert_eq!(
            map_cc(6, 0),
            Some(ControlAction::Synth(SynthParameter::Brightness, 0.0))
        );
        assert_eq!(
            map_cc(7, 127),
            Some(ControlAction::Synth(SynthParameter::Harmonics, 1.0))
        );
        assert_eq!(map_cc(8, 0), Some(ControlAction::Bpm(40)));
        assert_eq!(map_cc(8, 127), Some(ControlAction::Bpm(240)));
    }

    #[test]
    fn leaves_sustain_and_unassigned_ccs_for_other_handlers() {
        assert_eq!(map_cc(64, 127), None);
        assert_eq!(map_cc(9, 64), None);
    }

    #[test]
    fn logarithmic_envelope_ranges_have_musical_endpoints() {
        assert_eq!(
            map_cc(2, 0),
            Some(ControlAction::Synth(SynthParameter::Attack, 0.002))
        );
        assert_eq!(
            map_cc(2, 127),
            Some(ControlAction::Synth(SynthParameter::Attack, 1.5))
        );
        assert_eq!(
            map_cc(3, 0),
            Some(ControlAction::Synth(SynthParameter::Decay, 0.03))
        );
        assert_eq!(
            map_cc(3, 127),
            Some(ControlAction::Synth(SynthParameter::Decay, 3.0))
        );
        assert_eq!(
            map_cc(5, 0),
            Some(ControlAction::Synth(SynthParameter::Release, 0.03))
        );
        assert_eq!(
            map_cc(5, 127),
            Some(ControlAction::Synth(SynthParameter::Release, 4.0))
        );
    }

    #[test]
    fn malformed_patch_values_recover_to_finite_ranges() {
        let patch = PatchSettings {
            attack_seconds: f32::NAN,
            decay_seconds: f32::INFINITY,
            sustain_level: -3.0,
            release_seconds: 99.0,
            brightness: f32::NEG_INFINITY,
            harmonic_mix: 4.0,
        }
        .sanitize();
        assert!(patch.attack_seconds.is_finite());
        assert!(patch.decay_seconds.is_finite());
        assert_eq!(patch.sustain_level, 0.0);
        assert_eq!(patch.release_seconds, 4.0);
        assert!(patch.brightness.is_finite());
        assert_eq!(patch.harmonic_mix, 1.0);
    }
}
