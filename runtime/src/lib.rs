//! RaMOD runtime — voices, mixer, designer, profiler, live update.

mod automation;
mod designer;
mod dsp;
mod envelope;
mod events;
mod mixer;
mod profiler;
mod spatial;
mod spectrum;
mod voice;

#[cfg(feature = "live-update")]
mod live_update;

pub use automation::AutomationCurve;
pub use designer::{
    DesignedEvent, InstrumentDef, InstrumentKind, LoopRegion, TimelineTrack, TransitionMarker,
};
pub use dsp::InsertEffect;
pub use envelope::Ahdsr;
pub use events::{EventDescription, EventInstance};
pub use mixer::{BusId, BusSend, Meter, Mixer, Snapshot, SnapshotPlayer};
pub use profiler::{ProfilerFrame, ProfilerSession};
pub use spatial::{distance, Attenuation, Attributes3d, Vec3};
pub use spectrum::SpectrumAnalyzer;
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
