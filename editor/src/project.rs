//! S3.1 — Project model: events, banks, assets. Save/load JSON + bank build.

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
    /// Path to audio file on disk (wav/mp3/ogg/flac).
    pub source_path: Option<String>,
    /// Synthetic tone (Hz) when no file.
    pub demo_freq_hz: Option<f32>,
}

/// Built bank product (JSON) for runtime / bevy_ramod load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankBuildManifest {
    pub name: String,
    pub project: String,
    pub events: Vec<BankBuildEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankBuildEvent {
    pub path: String,
    pub asset: Option<String>,
    pub asset_file: Option<String>,
    pub looping: bool,
    pub is_3d: bool,
    pub priority: i32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub parameters: Vec<(String, f32)>,
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

    /// Register a file-backed asset. Returns index.
    pub fn add_asset_file(&mut self, path: &Path) -> usize {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("asset")
            .to_string();
        self.assets.push(AssetEntry {
            name,
            source_path: Some(path.to_string_lossy().into_owned()),
            demo_freq_hz: None,
        });
        self.assets.len() - 1
    }

    /// Link asset index to event index.
    pub fn assign_asset_to_event(
        &mut self,
        event_index: usize,
        asset_index: usize,
    ) -> Result<(), String> {
        if event_index >= self.events.len() {
            return Err("event index out of range".into());
        }
        if asset_index >= self.assets.len() {
            return Err("asset index out of range".into());
        }
        self.events[event_index].asset_index = Some(asset_index);
        Ok(())
    }

    /// Build a bank folder: `out_dir/<bank>/bank.json` + copied assets.
    pub fn export_bank_build(&self, bank_name: &str, out_dir: &Path) -> Result<PathBuf, String> {
        let bank_dir = out_dir.join(bank_name);
        let assets_dir = bank_dir.join("assets");
        std::fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;

        let mut built_events = Vec::new();
        for ev in self.events.iter().filter(|e| e.bank == bank_name) {
            let (asset_name, asset_file) = if let Some(ai) = ev.asset_index {
                if let Some(asset) = self.assets.get(ai) {
                    let name = asset.name.clone();
                    let file = if let Some(ref sp) = asset.source_path {
                        let src = Path::new(sp);
                        let dest_name = src
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("sample.wav");
                        let dest = assets_dir.join(dest_name);
                        if src.exists() {
                            std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
                            Some(format!("assets/{dest_name}"))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    (Some(name), file)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            built_events.push(BankBuildEvent {
                path: ev.path.clone(),
                asset: asset_name,
                asset_file,
                looping: ev.looping,
                is_3d: ev.is_3d,
                priority: ev.priority,
                min_distance: ev.min_distance,
                max_distance: ev.max_distance,
                parameters: ev.parameters.clone(),
            });
        }

        let manifest = BankBuildManifest {
            name: bank_name.to_string(),
            project: self.name.clone(),
            events: built_events,
        };
        let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
        let manifest_path = bank_dir.join("bank.json");
        std::fs::write(&manifest_path, json).map_err(|e| e.to_string())?;
        Ok(manifest_path)
    }
}
