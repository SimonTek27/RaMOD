//! Event Designer data model (S5) — tracks, instruments, markers.

use crate::automation::AutomationCurve;
use crate::envelope::Ahdsr;
use crate::spatial::Attenuation;

/// How an instrument picks samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentKind {
    /// One buffer.
    Single,
    /// Round-robin / random among entries.
    Multi,
    /// Play N voices at once with scatter.
    Scatter,
}

/// One playable instrument on a track.
#[derive(Debug, Clone)]
pub struct InstrumentDef {
    pub name: String,
    pub kind: InstrumentKind,
    /// Buffer indices into the event's buffer list (resolved at register time).
    pub buffer_slots: Vec<usize>,
    pub start_time: f32,
    pub length: f32,
    pub probability: f32,
    pub looping: bool,
    pub envelope: Ahdsr,
    /// Scatter: number of voices.
    pub scatter_count: u32,
}

impl InstrumentDef {
    pub fn single(name: impl Into<String>, slot: usize) -> Self {
        Self {
            name: name.into(),
            kind: InstrumentKind::Single,
            buffer_slots: vec![slot],
            start_time: 0.0,
            length: 2.0,
            probability: 1.0,
            looping: false,
            envelope: Ahdsr::default(),
            scatter_count: 1,
        }
    }
}

/// One timeline track.
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

/// Transition marker for adaptive music (S5.6).
#[derive(Debug, Clone)]
pub struct TransitionMarker {
    pub name: String,
    pub time: f32,
    pub target_event: Option<String>,
}

/// Full designed event (multi-track).
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
    /// Parameter defaults.
    pub parameters: Vec<(String, f32)>,
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
        }
    }

    pub fn with_track(mut self, track: TimelineTrack) -> Self {
        self.tracks.push(track);
        self
    }
}

/// Loop region on the timeline (S5.4).
#[derive(Debug, Clone, Copy)]
pub struct LoopRegion {
    pub start: f32,
    pub end: f32,
}
