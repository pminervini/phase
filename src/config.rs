use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Cyberpunk,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub preferred_midi_port: Option<String>,
    pub preferred_audio_device: Option<String>,
    pub master_volume: f32,
    pub default_bpm: u16,
    pub training_midi_low: u8,
    pub training_midi_high: u8,
    pub theme: Theme,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            preferred_midi_port: None,
            preferred_audio_device: None,
            master_volume: 0.72,
            default_bpm: 100,
            training_midi_low: 48,
            training_midi_high: 72,
            theme: Theme::Cyberpunk,
        }
    }
}

impl Config {
    pub fn sanitize(mut self) -> Self {
        self.version = CONFIG_VERSION;
        if !self.master_volume.is_finite() {
            self.master_volume = Self::default().master_volume;
        }
        self.master_volume = self.master_volume.clamp(0.0, 1.0);
        self.default_bpm = self.default_bpm.clamp(30, 300);
        self.training_midi_low = self.training_midi_low.min(127);
        self.training_midi_high = self.training_midi_high.min(127);
        if self.training_midi_low > self.training_midi_high {
            std::mem::swap(&mut self.training_midi_low, &mut self.training_midi_high);
        }
        self
    }
}

pub fn config_path() -> Result<PathBuf> {
    let dirs =
        ProjectDirs::from("", "", "phase").context("resolve phase configuration directory")?;
    Ok(dirs.config_dir().join("config.toml"))
}

pub fn load() -> Result<(Config, Option<String>)> {
    load_from(&config_path()?)
}

pub fn load_from(path: &Path) -> Result<(Config, Option<String>)> {
    if !path.exists() {
        let config = Config::default();
        save_to(path, &config)?;
        return Ok((config, None));
    }
    let contents = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    match toml::from_str::<Config>(&contents) {
        Ok(config) => Ok((config.sanitize(), None)),
        Err(error) => Ok((
            Config::default(),
            Some(format!(
                "Malformed config at {}: {error}; using defaults",
                path.display()
            )),
        )),
    }
}

pub fn save(config: &Config) -> Result<()> {
    save_to(&config_path()?, config)
}

pub fn save_to(path: &Path, config: &Config) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let serialized = toml::to_string_pretty(&config.clone().sanitize())
        .context("serialize phase configuration")?;
    fs::write(path, serialized).with_context(|| format!("write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "phase-{name}-{}-{}.toml",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn defaults_are_safe() {
        let config = Config::default();
        assert_eq!(config.version, CONFIG_VERSION);
        assert!((0.0..=1.0).contains(&config.master_volume));
        assert!(config.training_midi_low <= config.training_midi_high);
    }

    #[test]
    fn round_trip_preserves_configuration() {
        let path = temp_path("roundtrip");
        let config = Config {
            master_volume: 0.4,
            default_bpm: 144,
            ..Config::default()
        };
        save_to(&path, &config).unwrap();
        let (loaded, warning) = load_from(&path).unwrap();
        assert_eq!(loaded, config);
        assert!(warning.is_none());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn malformed_config_recovers_nonfatally() {
        let path = temp_path("malformed");
        fs::write(&path, "this = [is not valid").unwrap();
        let (loaded, warning) = load_from(&path).unwrap();
        assert_eq!(loaded, Config::default());
        assert!(warning.is_some());
        fs::remove_file(path).unwrap();
    }
}
