//! S3.1 / S5 / S7 — Project model: events, banks, assets, tracks, FX, AHDSR.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub events: Vec<EventEntry>,
    pub banks: Vec<BankEntry>,
    pub assets: Vec<AssetEntry>,
    #[serde(default)]
    pub fx_presets: Vec<FxPresetEntry>,
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
pub struct FxPresetEntry {
    pub name: String,
    pub inserts: Vec<String>,
}

/// Authoring-side parameter kind (mirrors ramod_runtime::ParameterKind).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ParameterKindEntry {
    #[default]
    Continuous,
    Discrete {
        min_i: i32,
        max_i: i32,
    },
    Labeled {
        labels: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefEntry {
    pub name: String,
    pub default: f32,
    #[serde(default = "default_param_min")]
    pub min: f32,
    #[serde(default = "default_param_max")]
    pub max: f32,
    #[serde(default)]
    pub kind: ParameterKindEntry,
}

fn default_param_min() -> f32 {
    0.0
}
fn default_param_max() -> f32 {
    1.0
}

impl ParameterDefEntry {
    pub fn continuous(name: impl Into<String>, default: f32) -> Self {
        Self {
            name: name.into(),
            default,
            min: 0.0,
            max: 1.0,
            kind: ParameterKindEntry::Continuous,
        }
    }

    pub fn labeled(name: impl Into<String>, labels: &[&str], default_index: usize) -> Self {
        let labels: Vec<String> = labels.iter().map(|s| (*s).to_string()).collect();
        let n = labels.len().max(1);
        let idx = default_index.min(n - 1);
        Self {
            name: name.into(),
            default: idx as f32,
            min: 0.0,
            max: (n - 1) as f32,
            kind: ParameterKindEntry::Labeled { labels },
        }
    }

    pub fn discrete(name: impl Into<String>, default: i32, min_i: i32, max_i: i32) -> Self {
        Self {
            name: name.into(),
            default: default as f32,
            min: min_i as f32,
            max: max_i as f32,
            kind: ParameterKindEntry::Discrete { min_i, max_i },
        }
    }
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
    /// Rich parameter metadata (discrete / labeled).
    #[serde(default)]
    pub parameter_defs: Vec<ParameterDefEntry>,
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
    2.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackEntry {
    pub name: String,
    pub instruments: Vec<InstrumentEntry>,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub solo: bool,
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
    #[serde(default)]
    pub attack: f32,
    #[serde(default)]
    pub hold: f32,
    #[serde(default)]
    pub decay: f32,
    #[serde(default = "default_sustain")]
    pub sustain: f32,
    #[serde(default)]
    pub release: f32,
}

fn default_inst_len() -> f32 {
    0.5
}
fn default_prob() -> f32 {
    1.0
}
fn default_sustain() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationEntry {
    pub parameter: String,
    pub keys: Vec<(f32, f32)>,
}

impl AutomationEntry {
    pub fn intensity_fade(start: f32, end: f32, t_end: f32) -> Self {
        Self {
            parameter: "Intensity".into(),
            keys: vec![(0.0, start), (t_end, end)],
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
                    parameters: vec![("Intensity".into(), 1.0), ("Surface".into(), 0.0)],
                    parameter_defs: vec![
                        ParameterDefEntry::continuous("Intensity", 1.0),
                        ParameterDefEntry::labeled("Surface", &["Dirt", "Grass", "Metal"], 0),
                    ],
                    tracks: vec![TrackEntry {
                        name: "Audio 1".into(),
                        instruments: vec![InstrumentEntry {
                            name: "beep".into(),
                            asset_index: Some(0),
                            start_time: 0.0,
                            length: 0.5,
                            probability: 1.0,
                            looping: false,
                            attack: 0.0,
                            hold: 0.0,
                            decay: 0.0,
                            sustain: 1.0,
                            release: 0.05,
                        }],
                        mute: false,
                        solo: false,
                    }],
                    length: 2.0,
                    automation: Vec::new(),
                    markers: Vec::new(),
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
                    parameter_defs: vec![ParameterDefEntry::continuous("Intensity", 0.8)],
                    tracks: vec![TrackEntry {
                        name: "Audio 1".into(),
                        instruments: vec![InstrumentEntry {
                            name: "loop".into(),
                            asset_index: Some(0),
                            start_time: 0.0,
                            length: 1.0,
                            probability: 1.0,
                            looping: true,
                            attack: 0.0,
                            hold: 0.0,
                            decay: 0.0,
                            sustain: 1.0,
                            release: 0.05,
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
    pub fn add_event(&mut self, path: &str) {
        self.events.push(EventEntry {
            path: path.into(),
            bank: "Master".into(),
            asset_index: None,
            looping: false,
            is_3d: false,
            priority: 128,
            min_distance: 1.0,
            max_distance: 50.0,
            parameters: vec![("Intensity".into(), 1.0)],
            parameter_defs: vec![ParameterDefEntry::continuous("Intensity", 1.0)],
            tracks: vec![],
            length: 2.0,
            automation: Vec::new(),
            markers: Vec::new(),
            output_bus: "SFX".into(),
        });
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, s).map_err(|e| e.to_string())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&s).map_err(|e| e.to_string())
    }

    pub fn export_bank_build(&self, _bank_name: &str, _out_path: &Path) -> Result<usize, String> {
        Ok(0)
    }
}
