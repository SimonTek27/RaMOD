//! # bevy_ramod — Bevy plugin for RaMOD runtime
//!
//! Open-source alternative path to `bevy_fmod`, powered by pure-Rust `ramod_runtime`.
//!
//! **Bevy version:** 0.19 (aligned with [Renzora Engine](https://github.com/renzora/engine)).
//!
//! ## Quick path
//!
//! ```ignore
//! commands.spawn(LoadBank::new("build/Master/bank.json"));
//! commands.spawn((
//!     SpatialAudioBundle::new("event:/sfx/engine", Vec3::ZERO),
//!     PlayOnSpawn,
//!     AudioParameter::new("RPM", 0.4),
//! ));
//! ```
//!
//! ## Renzora
//!
//! Enable feature `renzora` for host markers, **replace Kira**, and
//! [`renzora::backend::RamodRenzoraBackend`]. See `docs/RENZORA.md`.

mod assets;
mod components;
mod plugin;

#[cfg(feature = "renzora")]
pub mod renzora;

pub mod prelude {
    pub use crate::assets::{BankEventAsset, RamodBankAsset, RamodBankHandle};
    pub use crate::components::{
        AudioEmitter, AudioListener, AudioParameter, AudioParameters, LoadBank, PlayOnSpawn,
        SpatialAudioBundle, Velocity,
    };
    pub use crate::plugin::{LiveUpdateHub, RamodAudio, RamodPlugin};
    pub use ramod_runtime::{
        Attributes3d, Attenuation, EventDescription, PlayParams, RuntimeConfig, Vec3,
    };

    #[cfg(feature = "renzora")]
    pub use crate::renzora::{RenzoraHost, RenzoraRamodExt};
}

pub use assets::{BankEventAsset, RamodBankAsset, RamodBankHandle};
pub use components::{
    AudioEmitter, AudioListener, AudioParameter, AudioParameters, LoadBank, PlayOnSpawn,
    SpatialAudioBundle, Velocity,
};
pub use plugin::{LiveUpdateHub, RamodAudio, RamodPlugin};
