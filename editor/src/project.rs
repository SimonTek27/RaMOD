//! S3.1 — Project model: events, banks, assets. Save/load JSON.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub events: Vec<EventEntry>,
    pub banks: Vec<BankEntry>,
    pub assets: Vec<AssetEntry>,
    #[serde(default)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    pub path: String,
    pub bank: String,
    /// Index into project.assets, if any.
    pub asset_index: Option<usize>,
    #[serde(default)]
    pub looping: bool,
    #[serde(default)]
    pub is_3d: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default = "default_min_dist")]
    pub min_distance: f32,
    #[serde(default = "default_max_dist")]
    pub max_distance: f32,
    /// Named parameters (name → default).
    #[serde(default)]
    pub parameters: Vec<(String, f32)>,
}

fn default_priority() -> i32 {
    128
}
fn default_min_dist() -> f32 {
    1.0
}
fn default_max_dist() -> f32 {
    50.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankEntry {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    pub name: String,
    /// Optional path to a WAV on disk (editor resolves for Play).
    pub source_path: Option<String>,
    /// Synthetic tone for demo when no file (Hz). None = silence buffer.
    pub demo_freq_hz: Option<f32>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            events: vec![EventEntry {
                path: "event:/sfx/beep".into(),
                bank: "Master".into(),
                asset_index: Some(0),
                looping: false,
                is_3d: false,
                priority: 128,
                min_distance: 1.0,
                max_distance: 50.0,
                parameters: vec![("Intensity".into(), 1.0)],
            }],
            banks: vec![BankEntry {
                name: "Master".into(),
            }],
            assets: vec![AssetEntry {
                name: "beep_440".into(),
                source_path: None,
                demo_freq_hz: Some(440.0),
            }],
            path: None,
        }
    }
}

impl Project {
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut p: Project = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        p.path = Some(path.to_path_buf());
        Ok(p)
    }

    pub fn add_event(&mut self, path: String) {
        self.events.push(EventEntry {
            path,
            bank: self
                .banks
                .first()
                .map(|b| b.name.clone())
                .unwrap_or_else(|| "Master".into()),
            asset_index: self.assets.first().map(|_| 0),
            looping: false,
            is_3d: false,
            priority: 128,
            min_distance: 1.0,
            max_distance: 50.0,
            parameters: vec![("Intensity".into(), 1.0)],
        });
    }

    pub fn add_asset_demo(&mut self, name: String, freq: f32) {
        self.assets.push(AssetEntry {
            name,
            source_path: None,
            demo_freq_hz: Some(freq),
        });
    }
}
