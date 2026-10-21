//! S3.1 / S5 — Project model: events, banks, assets, tracks, automation, markers.

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
    #[serde(default)]
    pub tracks: Vec<TrackEntry>,
    #[serde(default = "default_event_len")]
    pub length: f32,
    #[serde(default)]
    pub automation: Vec<AutomationEntry>,
    #[serde(default)]
    pub markers: Vec<MarkerEntry>,
}

fn default_priority() -> i32 { 128 }
fn default_min_dist() -> f32 { 1.0 }
fn default_max_dist() -> f32 { 50.0 }
fn default_event_len() -> f32 { 4.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankEntry { pub name: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    pub name: String,
    pub source_path: Option<String>,
    pub demo_freq_hz: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentEntry {
    pub name: String,
    #[serde(default)]
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

fn default_inst_len() -> f32 { 2.0 }
fn default_prob() -> f32 { 1.0 }

impl InstrumentEntry {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            asset_index: None,
            start_time: 0.0,
            length: 2.0,
            probability: 1.0,
            looping: false,
        }
    }
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
pub struct AutomationEntry {
    pub parameter: String,
    #[serde(default)]
    pub keys: Vec<(f32, f32)>,
}

impl AutomationEntry {
    pub fn intensity_fade(start: f32, end: f32, length: f32) -> Self {
        Self {
            parameter: "Intensity".into(),
            keys: vec![(0.0, start), (length.max(0.01), end)],
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

impl MarkerEntry {
    pub fn new(name: impl Into<String>, time: f32) -> Self {
        Self {
            name: name.into(),
            time,
            target_event: None,
        }
    }
}

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
    #[serde(default)]
    pub tracks: Vec<TrackEntry>,
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub automation: Vec<AutomationEntry>,
    #[serde(default)]
    pub markers: Vec<MarkerEntry>,
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
                markers: vec![MarkerEntry::new("end", 2.0)],
            }],
            banks: vec![BankEntry { name: "Master".into() }],
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
            bank: self.banks.first().map(|b| b.name.clone()).unwrap_or_else(|| "Master".into()),
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
        });
    }

    pub fn add_track(&mut self, event_index: usize, name: impl Into<String>) -> Result<(), String> {
        let ev = self.events.get_mut(event_index).ok_or_else(|| "event index out of range".to_string())?;
        ev.tracks.push(TrackEntry::new(name));
        Ok(())
    }

    pub fn add_instrument(
        &mut self,
        event_index: usize,
        track_index: usize,
        name: impl Into<String>,
        asset_index: Option<usize>,
    ) -> Result<(), String> {
        let ev = self.events.get_mut(event_index).ok_or_else(|| "event index out of range".to_string())?;
        let track = ev.tracks.get_mut(track_index).ok_or_else(|| "track index out of range".to_string())?;
        let mut inst = InstrumentEntry::new(name);
        inst.asset_index = asset_index;
        track.instruments.push(inst);
        Ok(())
    }

    pub fn add_asset_demo(&mut self, name: String, freq: f32) {
        self.assets.push(AssetEntry { name, source_path: None, demo_freq_hz: Some(freq) });
    }

    pub fn add_asset_file(&mut self, path: &Path) -> usize {
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset").to_string();
        self.assets.push(AssetEntry {
            name,
            source_path: Some(path.to_string_lossy().into_owned()),
            demo_freq_hz: None,
        });
        self.assets.len() - 1
    }

    pub fn assign_asset_to_event(&mut self, event_index: usize, asset_index: usize) -> Result<(), String> {
        if event_index >= self.events.len() { return Err("event index out of range".into()); }
        if asset_index >= self.assets.len() { return Err("asset index out of range".into()); }
        self.events[event_index].asset_index = Some(asset_index);
        Ok(())
    }

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
                        let dest_name = src.file_name().and_then(|s| s.to_str()).unwrap_or("sample.wav");
                        let dest = assets_dir.join(dest_name);
                        if src.exists() {
                            std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
                            Some(format!("assets/{dest_name}"))
                        } else { None }
                    } else { None };
                    (Some(name), file)
                } else { (None, None) }
            } else { (None, None) };
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
                tracks: ev.tracks.clone(),
                length: ev.length,
                automation: ev.automation.clone(),
                markers: ev.markers.clone(),
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
        let _ = self.export_fsb_bank(bank_name, &bank_dir.join(format!("{bank_name}.fsb")));
        Ok(manifest_path)
    }

    pub fn export_fsb_bank(&self, bank_name: &str, out_path: &Path) -> Result<usize, String> {
        use ramod::{Fsb5Builder, PcmSample, SoundFormat};
        use ramod_waveform::WaveBuffer;
        let mut builder = Fsb5Builder::new(SoundFormat::Pcm16);
        let mut count = 0usize;
        for ev in self.events.iter().filter(|e| e.bank == bank_name) {
            let Some(ai) = ev.asset_index else { continue };
            let Some(asset) = self.assets.get(ai) else { continue };
            let Some(ref sp) = asset.source_path else { continue };
            let path = Path::new(sp);
            if !path.exists() { continue; }
            let wave = WaveBuffer::from_file(path).map_err(|e| e.to_string())?;
            let pcm_i16 = f32_to_pcm16(&wave.samples);
            let mut sample = PcmSample::new(asset.name.clone(), wave.sample_rate, wave.channels as u32, pcm_i16);
            if ev.looping {
                let frames = wave.frames() as u32;
                sample = sample.with_loop(0, frames.saturating_sub(1));
            }
            builder.push(sample);
            count += 1;
        }
        if count == 0 { return Err("no file-backed assets to put in FSB".into()); }
        let bytes = builder.build().map_err(|e| format!("FSB build: {e}"))?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(out_path, bytes).map_err(|e| e.to_string())?;
        Ok(count)
    }
}

fn f32_to_pcm16(samples: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}
