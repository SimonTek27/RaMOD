//! # bevy_ramod — Bevy plugin for RaMOD runtime
//!
//! Open-source alternative path to `bevy_fmod`, powered by pure-Rust `ramod_runtime`.
//!
//! ## Quick path
//!
//! ```ignore
//! commands.spawn(LoadBank::new("build/Master/bank.json"));
//! commands.spawn((
//!     SpatialAudioBundle::new("event:/sfx/beep", Vec3::ZERO),
//!     PlayOnSpawn,
//! ));
//! ```

mod assets;
mod components;
mod plugin;

pub mod prelude {
    pub use crate::assets::{BankEventAsset, RamodBankAsset, RamodBankHandle};
    pub use crate::components::{
        AudioEmitter, AudioListener, LoadBank, PlayOnSpawn, SpatialAudioBundle, Velocity,
    };
    pub use crate::plugin::{RamodAudio, RamodPlugin};
    pub use ramod_runtime::{
        Attributes3d, Attenuation, EventDescription, PlayParams, RuntimeConfig, Vec3,
    };
}

pub use assets::{BankEventAsset, RamodBankAsset, RamodBankHandle};
pub use components::{
    AudioEmitter, AudioListener, LoadBank, PlayOnSpawn, SpatialAudioBundle, Velocity,
};
pub use plugin::{RamodAudio, RamodPlugin};
