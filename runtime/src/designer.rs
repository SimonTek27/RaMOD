//! Multi-track designed events (FMOD Studio-style).

use crate::automation::AutomationCurve;
use crate::envelope::Ahdsr;
use crate::spatial::Attenuation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentKind {
    Single,
    Multi,
    Scatter,
}

#[derive(Debug, Clone)]
pub struct InstrumentDef {
    pub name: String,
    pub kind: InstrumentKind,
    pub buffer_slots: Vec<usize>,
    pub start_time: f32,
    pub length: f32,
    pub probability: f32,
    pub looping: bool,
    pub scatter_count: usize,
    pub envelope: Ahdsr,
}

impl InstrumentDef {
    pub fn single(name: impl Into<String>, slot: usize) -> Self {
        Self {
            name: name.into(),
            kind: InstrumentKind::Single,
            buffer_slots: vec![slot],
            start_time: 0.0,
            length: 0.0,
            probability: 1.0,
            looping: false,
            scatter_count: 1,
            envelope: Ahdsr::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimelineTrack {
    pub name: String,
    pub instruments: Vec<InstrumentDef>,
    pub mute: bool,
    pub solo: bool,
}

impl TimelineTrack {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            instruments: Vec::new(),
            mute: false,
            solo: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoopRegion {
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Clone)]
pub struct TransitionMarker {
    pub name: String,
    pub time: f32,
    pub target_event: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DesignedEvent {
    pub path: String,
    pub tracks: Vec<TimelineTrack>,
    pub automation: Vec<AutomationCurve>,
    pub markers: Vec<TransitionMarker>,
    pub is_3d: bool,
    pub attenuation: Attenuation,
    pub priority: i32,
    pub length: f32,
    pub parameters: Vec<(String, f32)>,
    pub output_bus: String,
}

impl DesignedEvent {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            tracks: Vec::new(),
            automation: Vec::new(),
            markers: Vec::new(),
            is_3d: false,
            attenuation: Attenuation::default(),
            priority: 128,
            length: 4.0,
            parameters: vec![("Intensity".into(), 1.0)],
            output_bus: "SFX".into(),
        }
    }

    pub fn with_track(mut self, track: TimelineTrack) -> Self {
        self.tracks.push(track);
        self
    }
}
