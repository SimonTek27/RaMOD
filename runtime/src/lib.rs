//! # ramod_runtime — open-source real-time audio engine
//!
//! S1–S9 stack with audible cpal output via ring buffer.

mod automation;
mod backend;
mod bank;
mod buffer;
mod bus;
mod designer;
mod dsp;
mod envelope;
mod error;
mod event;
mod export;
mod live_api;
#[cfg(feature = "live-update")]
pub mod live_update;
mod mixer;
mod parameter;
mod profiler;
mod spatial;
mod voice;

pub use automation::{AutomationCurve, Keyframe};
pub use backend::{NullBackend, OutputBackend};
pub use bank::{Bank, BankEventSpec, BankName, BankRegistry};
pub use buffer::{BufferHandle, PcmBuffer};
pub use bus::MasterBus;
pub use designer::{
    DesignedEvent, InstrumentDef, InstrumentKind, LoopRegion, TimelineTrack, TransitionMarker,
};
pub use dsp::InsertEffect;
pub use envelope::Ahdsr;
pub use error::RuntimeError;
pub use event::{EventDescription, EventInstance, EventPath};
pub use export::{BankManifest, EventManifestEntry};
pub use mixer::{Bus, BusId, BusSend, Meter, Mixer, Snapshot, SnapshotPlayer};
pub use parameter::{ParameterDef, ParameterKind, ParameterRegistry};
pub use profiler::{EmitterDebug, ProfilerFrame, ProfilerSession, SpectrumAnalyzer};
pub use spatial::{doppler_pitch, Attributes3d, Attenuation, Vec3};
pub use voice::{PlayParams, VoiceHandle, VoiceState};

/// Voice-stealing policy when an event hits its instance limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StealingMode {
    #[default]
    Oldest,
    Quietest,
    LowestPriority,
    None,
}

/// One bus row for the editor profiler.
#[derive(Debug, Clone)]
pub struct BusMeterInfo {
    pub id: u32,
    pub name: String,
    pub peak: f32,
    pub rms: f32,
    pub mute: bool,
    pub solo: bool,
    pub volume_db: f32,
    /// Linear send level to the Reverb return bus (0..=1).
    pub send_reverb: f32,
    pub is_return: bool,
    /// Parent bus id for hierarchy (None = top-level under Master).
    pub parent_id: Option<u32>,
}

#[cfg(feature = "live-update")]
pub use live_update::{LiveUpdateClient, LiveUpdateServer};

include!("runtime_core_a.rs");
include!("runtime_core_b.rs");
