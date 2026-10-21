//! S3.1 / S5 — Project model: events, banks, assets, tracks, automation, markers.

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
    pub target_event: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankManifestJson {
    pub name: String,
    pub events: Vec<EventManifestRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventManifestRow {
    pub path: String,
    pub asset: Option<String>,
    pub looping: bool,
    pub is_3d: bool,
    pub priority: i32,
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
                            name: "beep_b".into(),
                            asset_index: Some(0),
                            start_time: 0.0,
                            length: 0.5,
                            probability: 1.0,
                            looping: false,
                        }],
                        mute: false,
                        solo: false,
                    }],
                    length: 3.0,
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
        }
    }
}

impl Project {
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
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
            tracks: vec![TrackEntry::new("Audio 1")],
            length: 4.0,
            automation: Vec::new(),
            markers: Vec::new(),
            output_bus: "SFX".into(),
        });
    }

    pub fn add_track(&mut self, event_index: usize, name: String) -> Result<(), String> {
        let e = self
            .events
            .get_mut(event_index)
            .ok_or_else(|| "bad event".into())?;
        e.tracks.push(TrackEntry::new(name));
        Ok(())
    }

    pub fn add_instrument(
        &mut self,
        event_index: usize,
        track_index: usize,
        name: String,
        asset_index: Option<usize>,
    ) -> Result<(), String> {
        let e = self
            .events
            .get_mut(event_index)
            .ok_or_else(|| "bad event".into())?;
        let t = e
            .tracks
            .get_mut(track_index)
            .ok_or_else(|| "bad track".into())?;
        t.instruments.push(InstrumentEntry {
            name,
            asset_index,
            start_time: 0.0,
            length: 0.5,
            probability: 1.0,
            looping: false,
        });
        Ok(())
    }

    pub fn add_asset_file(&mut self, path: &Path) -> usize {
        let name = path
            .file_stem()
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

    pub fn assign_asset_to_event(&mut self, event_index: usize, asset_index: usize) -> Result<(), String> {
        let e = self
            .events
            .get_mut(event_index)
            .ok_or_else(|| "bad event".into())?;
        if asset_index >= self.assets.len() {
            return Err("bad asset".into());
        }
        e.asset_index = Some(asset_index);
        Ok(())
    }

    pub fn export_bank_build(&self, bank_name: &str, out_dir: &Path) -> Result<PathBuf, String> {
        let bank_dir = out_dir.join(bank_name);
        let assets_dir = bank_dir.join("assets");
        std::fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
        let mut rows = Vec::new();
        for ev in self.events.iter().filter(|e| e.bank == bank_name) {
            let asset_name = ev
                .asset_index
                .and_then(|i| self.assets.get(i))
                .map(|a| a.name.clone());
            rows.push(EventManifestRow {
                path: ev.path.clone(),
                asset: asset_name,
                looping: ev.looping,
                is_3d: ev.is_3d,
                priority: ev.priority,
            });
        }
        let manifest = BankManifestJson {
            name: bank_name.to_string(),
            events: rows,
        };
        let manifest_path = bank_dir.join("bank.json");
        let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
        std::fs::write(&manifest_path, json).map_err(|e| e.to_string())?;
        let _ = self.export_fsb_bank(bank_name, &bank_dir.join(format!("{bank_name}.fsb")));
        Ok(manifest_path)
    }

    pub fn export_fsb_bank(&self, _bank_name: &str, _out_path: &Path) -> Result<usize, String> {
        Ok(0)
    }
}
