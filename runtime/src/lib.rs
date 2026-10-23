//! RaMOD runtime — voices, events, mixer, DSP, 3D, live-update.

pub mod buffer;
pub mod designed;
pub mod dsp;
pub mod error;
pub mod event;
pub mod mixer;
pub mod profiler;
pub mod spatial;
pub mod voice;

#[cfg(feature = "live-update")]
pub mod live_update;

pub use buffer::{BufferHandle, SampleBuffer};
pub use designed::{DesignedEvent, InstrumentDef, MarkerDef, TimelineTrack};
pub use dsp::{InsertEffect, SidechainCompressor};
pub use error::RuntimeError;
pub use event::{EventDescription, EventInstance, EventPath};
pub use mixer::{BusId, BusSend, Meter, Mixer};
pub use spatial::{Attenuation, Attributes3d};
pub use voice::{Ahdsr, VoicePool};

/// Per-bus meter snapshot for the profiler / editor.
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
    /// Parent bus id for hierarchy (None = top / Master child).
    pub parent_id: Option<u32>,
}

#[cfg(feature = "live-update")]
pub use live_update::{LiveUpdateClient, LiveUpdateServer};

include!("runtime_core_a.rs");
include!("runtime_core_b.rs");
