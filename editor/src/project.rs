//! S3.1 / S5 / S7 — Project model: events, banks, assets, tracks, FX, AHDSR.

use std::path::{Path, PathBuf};

use ramod_runtime::{
    Ahdsr, Attenuation, AutomationCurve, BankManifest, DesignedEvent, InstrumentDef,
    InstrumentKind, TimelineTrack, TransitionMarker,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub events: Vec<EventEntry>,
    pub banks: Vec<BankEntry>,
    pub assets: Vec<AssetEntry>,
    #[serde(default)]
    pub fx_presets: Vec<FxPreset>,
    /// Last save / open location, if the project has been written to disk.
    #[serde(default)]
    pub path: Option<PathBuf>,
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

/// Named insert chain captured from the live mixer (S7.12).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxPreset {
    pub name: String,
    /// Encoded inserts — see `AudioBridge::capture_fx_chain` for the format.
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

    #[allow(dead_code)] // authoring API: discrete params are not exposed in the UI yet
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

/// Instance stealing policy when max_instances is reached (S7.22).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum StealingMode {
    #[default]
    Oldest,
    Quietest,
    LowestPriority,
    None,
}

fn default_max_instances() -> u32 {
    8
}

/// Multi / Single / Scatterer instrument kinds (S7.23 / S7.24).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum InstrumentKindEntry {
    #[default]
    Single,
    Multi {
        playlist: Vec<PlaylistEntry>,
    },
    Scatterer {
        count: u32,
        time_spread: f32,
        pitch_spread: f32,
        gain_spread: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaylistEntry {
    pub asset_index: Option<usize>,
    #[serde(default = "default_weight")]
    pub weight: f32,
}

fn default_weight() -> f32 {
    1.0
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
    /// Max concurrent instances of this event (S7.22).
    #[serde(default = "default_max_instances")]
    pub max_instances: u32,
    /// Stealing policy when limit reached.
    #[serde(default)]
    pub stealing: StealingMode,
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

impl EventEntry {
    /// Authoring entry → runtime designed event (shared by playback and bank build).
    pub fn to_designed(&self) -> DesignedEvent {
        let mut designed = DesignedEvent::new(&self.path);
        designed.is_3d = self.is_3d;
        designed.attenuation = Attenuation {
            min_distance: self.min_distance.max(0.01),
            max_distance: self.max_distance.max(self.min_distance.max(0.01) + 0.01),
        };
        designed.priority = self.priority;
        designed.length = self.length;
        designed.output_bus = self.output_bus.clone();
        designed.parameters = self.parameters.clone();
        designed.automation = self
            .automation
            .iter()
            .map(|a| {
                let mut curve = AutomationCurve::new(&a.parameter);
                for (time, value) in &a.keys {
                    curve.add_key(*time, *value);
                }
                curve
            })
            .collect();
        designed.markers = self
            .markers
            .iter()
            .map(|m| TransitionMarker {
                name: m.name.clone(),
                time: m.time,
                target_event: m.target_event.clone(),
            })
            .collect();
        designed.tracks = self.tracks.iter().map(TrackEntry::to_designed).collect();
        designed
    }
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

impl TrackEntry {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            instruments: Vec::new(),
            mute: false,
            solo: false,
        }
    }

    pub fn to_designed(&self) -> TimelineTrack {
        let mut track = TimelineTrack::new(&self.name);
        track.mute = self.mute;
        track.solo = self.solo;
        track.instruments = self
            .instruments
            .iter()
            .map(InstrumentEntry::to_designed)
            .collect();
        track
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
    #[serde(default)]
    pub kind: InstrumentKindEntry,
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

impl InstrumentEntry {
    pub fn to_designed(&self) -> InstrumentDef {
        let primary = self.asset_index.unwrap_or(0);
        let mut def = InstrumentDef::single(&self.name, primary);
        def.start_time = self.start_time;
        def.length = self.length;
        def.probability = self.probability;
        def.looping = self.looping;
        def.envelope = Ahdsr {
            attack: self.attack,
            hold: self.hold,
            decay: self.decay,
            sustain: self.sustain,
            release: self.release,
        };
        match &self.kind {
            InstrumentKindEntry::Single => {}
            InstrumentKindEntry::Multi { playlist } => {
                def.kind = InstrumentKind::Multi;
                let slots: Vec<usize> = playlist.iter().filter_map(|p| p.asset_index).collect();
                if !slots.is_empty() {
                    def.buffer_slots = slots;
                }
            }
            InstrumentKindEntry::Scatterer { count, .. } => {
                def.kind = InstrumentKind::Scatter;
                def.scatter_count = (*count as usize).max(1);
            }
        }
        def
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationEntry {
    pub parameter: String,
    pub keys: Vec<(f32, f32)>,
}

impl AutomationEntry {
    #[allow(dead_code)] // authoring helper kept for automation presets
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
                            kind: InstrumentKindEntry::Single,
                        }],
                        mute: false,
                        solo: false,
                    }],
                    length: 2.0,
                    automation: Vec::new(),
                    markers: Vec::new(),
                    output_bus: "SFX".into(),
                    max_instances: 8,
                    stealing: StealingMode::Oldest,
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
                            kind: InstrumentKindEntry::Single,
                        }],
                        mute: false,
                        solo: false,
                    }],
                    length: 4.0,
                    automation: Vec::new(),
                    markers: Vec::new(),
                    output_bus: "SFX".into(),
                    max_instances: 8,
                    stealing: StealingMode::Oldest,
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
            path: None,
        }
    }
}

impl Project {
    #[allow(dead_code)] // authoring API used by tests and future New-Event menu
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
            max_instances: 8,
            stealing: StealingMode::Oldest,
        });
    }

    /// Append a track and return its index.
    pub fn add_track(&mut self, event_index: usize, name: impl Into<String>) -> Option<usize> {
        let event = self.events.get_mut(event_index)?;
        event.tracks.push(TrackEntry::new(name));
        Some(event.tracks.len() - 1)
    }

    /// Append an instrument to `track_index` and return `(track, instrument)`.
    pub fn add_instrument(
        &mut self,
        event_index: usize,
        track_index: usize,
        name: impl Into<String>,
        asset_index: Option<usize>,
    ) -> Option<(usize, usize)> {
        let event = self.events.get_mut(event_index)?;
        if event.tracks.is_empty() {
            event.tracks.push(TrackEntry::new("Audio 1"));
        }
        let track_index = track_index.min(event.tracks.len() - 1);
        let track = event.tracks.get_mut(track_index)?;
        track.instruments.push(InstrumentEntry {
            name: name.into(),
            asset_index,
            start_time: 0.0,
            length: 0.5,
            probability: 1.0,
            looping: false,
            attack: 0.0,
            hold: 0.0,
            decay: 0.0,
            sustain: 1.0,
            release: 0.05,
            kind: InstrumentKindEntry::Single,
        });
        Some((track_index, track.instruments.len() - 1))
    }

    /// Register a file as an asset and return its index.
    pub fn add_asset_file(&mut self, path: &Path) -> usize {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("asset")
            .to_string();
        if let Some(idx) = self
            .assets
            .iter()
            .position(|a| a.source_path.as_deref() == path.to_str())
        {
            return idx;
        }
        self.assets.push(AssetEntry {
            name,
            source_path: Some(path.to_string_lossy().into_owned()),
            demo_freq_hz: None,
        });
        self.assets.len() - 1
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        std::fs::write(path, s).map_err(|e| e.to_string())
    }

    #[allow(dead_code)] // authoring API: the Open menu item is not wired yet
    pub fn load(path: &Path) -> Result<Self, String> {
        let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut project: Self = serde_json::from_str(&s).map_err(|e| e.to_string())?;
        project.path = Some(path.to_path_buf());
        Ok(project)
    }

    /// Build a bank manifest for every event assigned to `bank_name`.
    ///
    /// Writes `<out_path>/<bank>.bank.json` and returns the manifest path.
    pub fn export_bank_build(&self, bank_name: &str, out_path: &Path) -> Result<PathBuf, String> {
        let designed: Vec<DesignedEvent> = self
            .events
            .iter()
            .filter(|e| e.bank.eq_ignore_ascii_case(bank_name))
            .map(EventEntry::to_designed)
            .collect();
        if designed.is_empty() {
            return Err(format!("bank '{bank_name}' has no events"));
        }
        let manifest = BankManifest::from_events(bank_name, &designed);
        std::fs::create_dir_all(out_path).map_err(|e| e.to_string())?;
        let safe: String = bank_name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        let path = out_path.join(format!("{safe}.bank.json"));
        std::fs::write(&path, manifest.to_json()).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_project_round_trips_through_json() {
        let project = Project::default();
        let json = serde_json::to_string(&project).expect("serialize");
        let back: Project = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.events.len(), project.events.len());
        assert!(back.path.is_none());
    }

    #[test]
    fn add_track_and_instrument_target_the_requested_event() {
        let mut project = Project::default();
        let track = project.add_track(1, "Audio 2").expect("event 1 exists");
        let (t, i) = project
            .add_instrument(1, track, "clip", Some(0))
            .expect("instrument added");
        assert_eq!((t, i), (track, 0));
        assert_eq!(project.events[1].tracks.len(), 2);
        assert_eq!(project.events[1].tracks[track].instruments.len(), 1);
    }

    #[test]
    fn add_asset_file_is_idempotent() {
        let mut project = Project::default();
        let p = Path::new("assets/kick.wav");
        let first = project.add_asset_file(p);
        let second = project.add_asset_file(p);
        assert_eq!(first, second);
        assert_eq!(project.assets[first].name, "kick");
    }

    #[test]
    fn designed_event_carries_timing_and_3d_fields() {
        let mut event = Project::default().events.remove(0);
        event.is_3d = true;
        event.min_distance = 2.0;
        event.max_distance = 20.0;
        let designed = event.to_designed();
        assert!(designed.is_3d);
        assert_eq!(designed.attenuation.min_distance, 2.0);
        assert_eq!(designed.tracks.len(), 1);
        assert_eq!(designed.tracks[0].instruments[0].buffer_slots, vec![0]);
    }

    #[test]
    fn bank_build_writes_a_manifest() {
        let project = Project::default();
        let out = std::env::temp_dir().join("ramod_bank_build_test");
        let path = project
            .export_bank_build("Master", &out)
            .expect("build succeeds");
        let text = std::fs::read_to_string(&path).expect("manifest written");
        assert!(text.contains("event:/sfx/beep"));
        let _ = std::fs::remove_dir_all(&out);
    }

    #[test]
    fn empty_bank_is_reported_not_silently_written() {
        let project = Project::default();
        let out = std::env::temp_dir().join("ramod_bank_build_empty");
        assert!(project.export_bank_build("Missing", &out).is_err());
    }
}
