use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PracticeStats {
    pub version: u32,
    pub aggregate_practice_seconds: u64,
    pub attempts: u64,
    pub correct: u64,
    pub weak_note_counts: BTreeMap<u8, u64>,
    pub best_streaks: BTreeMap<String, u32>,
}

fn path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "phase").context("resolve phase data directory")?;
    Ok(dirs.data_local_dir().join("practice.toml"))
}

impl PracticeStats {
    pub fn load() -> Result<(Self, Option<String>)> {
        let path = path()?;
        if !path.exists() {
            return Ok((
                Self {
                    version: 1,
                    ..Self::default()
                },
                None,
            ));
        }
        let data = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        match toml::from_str(&data) {
            Ok(stats) => Ok((stats, None)),
            Err(error) => Ok((
                Self {
                    version: 1,
                    ..Self::default()
                },
                Some(format!("Malformed practice data: {error}; starting fresh")),
            )),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        fs::write(
            &path,
            toml::to_string_pretty(self).context("serialize practice statistics")?,
        )
        .with_context(|| format!("write {}", path.display()))
    }
}
