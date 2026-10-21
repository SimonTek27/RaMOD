//! S3.1 / S5 / S7.15 — Project model: events, banks, assets, tracks, FX presets.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    #[serde(skip)]
    pub path: Option<PathBuf>,
    pub events: Vec<EventEntry>,
    pub banks: Vec<BankEntry>,
    pub assets: Vec<AssetEntry>,
    /// Named bus FX chains (S7.15).
    #[serde(default)]
    pub fx_presets: Vec<FxPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxPreset {
    pub name: String,
    #[serde(default)]
    pub inserts: Vec<FxInsertSpec>,
}

/// One insert in a preset (bus by name + params).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxInsertSpec {
    pub bus: String,
    pub kind: String,
    #[serde(default)]
    pub p0: f32,
    #[serde(default)]
    pub p1: f32,
    #[serde(default)]
    pub p2: f32,
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
    #[serde(default)]
    pub tracks: Vec<TrackEntry>,
    #[serde(default = "default_event_len")]
    pub length: f32,
    #[serde(default)]
    pub automation: Vec<AutomationEntry>,
    #[serde(default)]
    pub markers: Vec<MarkerEntry>,
    #[serde(default = "default_output_bus")]
    pub output_bus: String,
}

fn default_output_bus() -> String {
    "SFX".into()
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
fn default_event_len() -> f32 {
    4.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankEntry {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    pub name: String,
    pub source_path: Option<String>,
    pub demo_freq_hz: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackEntry {
    pub name: String,
    #[serde(default)]
    pub instruments: Vec<InstrumentEntry>,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub solo: bool,
}

impl TrackEntry {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            instruments: Vec::new(),
            mute: false,
            solo: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentEntry {
    pub name: String,
    pub asset_index: Option<usize>,
    #[serde(default)]
    pub start_time: f32,
    #[serde(default = "default_inst_len")]
    pub length: f32,
    #[serde(default = "default_prob")]
    pub probability: f32,
    #[serde(default)]
    pub looping: bool,
}

fn default_inst_len() -> f32 {
    0.5
}
fn default_prob() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationEntry {
    pub parameter: String,
    pub keys: Vec<(f32, f32)>,
}

impl AutomationEntry {
    pub fn intensity_fade(start: f32, end: f32, at: f32) -> Self {
        Self {
            parameter: "Intensity".into(),
            keys: vec![(0.0, start), (at, end)],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkerEntry {
    pub name: String,
    pub time: f32,
    #[serde(default)]
    pub target_event: Option<String>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            name: "Demo Project".into(),
            path: None,
            events: vec![
                EventEntry {
                    path: "event:/sfx/beep".into(),
                    bank: "Master".into(),
                    asset_index: Some(0),
                    looping: false,
                    is_3d: false,
                    priority: 128,
                    min_distance: 1.0,
                    max_distance: 50.0,
                    parameters: vec![("Intensity".into(), 1.0)],
                    tracks: vec![TrackEntry {
                        name: "Audio 1".into(),
                        instruments: vec![InstrumentEntry {
                            name: "beep".into(),
                            asset_index: Some(0),
                            start_time: 0.0,
                            length: 0.5,
                            probability: 1.0,
                            looping: false,
                        }],
                        mute: false,
                        solo: false,
                    }],
                    length: 4.0,
                    automation: vec![AutomationEntry::intensity_fade(1.0, 0.2, 2.0)],
                    markers: vec![MarkerEntry {
                        name: "to_b".into(),
                        time: 1.5,
                        target_event: Some("event:/sfx/loop_b".into()),
                    }],
                    output_bus: "SFX".into(),
                },
                EventEntry {
                    path: "event:/sfx/loop_b".into(),
                    bank: "Master".into(),
                    asset_index: Some(0),
                    looping: false,
                    is_3d: false,
                    priority: 128,
                    min_distance: 1.0,
                    max_distance: 50.0,
                    parameters: vec![("Intensity".into(), 0.8)],
                    tracks: vec![TrackEntry {
                        name: "Audio 1".into(),
                        instruments: vec![InstrumentEntry {
                            name: "loop".into(),
                            asset_index: Some(0),
                            start_time: 0.0,
                            length: 1.0,
                            probability: 1.0,
                            looping: true,
                        }],
                        mute: false,
                        solo: false,
                    }],
                    length: 4.0,
                    automation: Vec::new(),
                    markers: Vec::new(),
                    output_bus: "SFX".into(),
                },
            ],
            banks: vec![BankEntry {
                name: "Master".into(),
            }],
            assets: vec![AssetEntry {
                name: "demo_beep".into(),
                source_path: None,
                demo_freq_hz: Some(440.0),
            }],
            fx_presets: Vec::new(),
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

    pub fn add_asset_file(&mut self, path: &Path) -> usize {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("asset")
            .to_string();
        self.assets.push(AssetEntry {
            name,
            source_path: Some(path.display().to_string()),
            demo_freq_hz: None,
        });
        self.assets.len() - 1
    }

    pub fn assign_asset_to_event(&mut self, event_i: usize, asset_i: usize) -> Result<(), String> {
        if event_i >= self.events.len() {
            return Err("invalid event".into());
        }
        if asset_i >= self.assets.len() {
            return Err("invalid asset".into());
        }
        self.events[event_i].asset_index = Some(asset_i);
        if let Some(track) = self.events[event_i].tracks.first_mut() {
            if let Some(inst) = track.instruments.first_mut() {
                inst.asset_index = Some(asset_i);
            }
        }
        Ok(())
    }

    pub fn export_bank_build(&self, bank: &str, out_dir: &Path) -> Result<PathBuf, String> {
        let dir = out_dir.join(bank);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let events: Vec<_> = self
            .events
            .iter()
            .filter(|e| e.bank == bank)
            .map(|e| {
                serde_json::json!({
                    "path": e.path,
                    "asset": e.asset_index.and_then(|i| self.assets.get(i).map(|a| a.name.clone())),
                    "output_bus": e.output_bus,
                    "is_3d": e.is_3d,
                    "looping": e.looping,
                })
            })
            .collect();
        let manifest = serde_json::json!({
            "bank": bank,
            "events": events,
        });
        let path = dir.join("bank.json");
        let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(path)
    }
}
