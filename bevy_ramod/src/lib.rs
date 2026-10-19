//! # bevy_ramod — Bevy plugin for RaMOD runtime
//!
//! Open-source alternative path to `bevy_fmod`, powered by pure-Rust `ramod_runtime`.
//!
//! ## Quick start
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_ramod::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(RamodPlugin)
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands, mut audio: ResMut<RamodAudio>) {
//!     commands.spawn(Camera3d::default());
//!     commands.spawn(AudioListener);
//!     // register + play after loading PCM / banks
//! }
//! ```

mod assets;
mod components;
mod plugin;

pub mod prelude {
    pub use crate::assets::{RamodBankAsset, RamodBankHandle};
    pub use crate::components::{
        AudioEmitter, AudioListener, PlayOnSpawn, SpatialAudioBundle, Velocity,
    };
    pub use crate::plugin::{RamodAudio, RamodPlugin};
    pub use ramod_runtime::{
        Attributes3d, Attenuation, EventDescription, PlayParams, RuntimeConfig, Vec3,
    };
}

pub use assets::{RamodBankAsset, RamodBankHandle};
pub use components::{
    AudioEmitter, AudioListener, PlayOnSpawn, SpatialAudioBundle, Velocity,
};
pub use plugin::{RamodAudio, RamodPlugin};
