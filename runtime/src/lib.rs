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
pub use mixer::{Bus, BusId, Meter, Mixer, Snapshot, SnapshotPlayer};
pub use profiler::{EmitterDebug, ProfilerFrame, ProfilerSession, SpectrumAnalyzer};
pub use spatial::{doppler_pitch, Attributes3d, Attenuation, Vec3};
pub use voice::{PlayParams, VoiceHandle, VoiceState};

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
}

#[cfg(feature = "live-update")]
pub use live_update::{LiveUpdateClient, LiveUpdateServer};

include!("runtime_core.rs");
