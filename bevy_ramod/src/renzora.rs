//! Renzora Engine integration helpers (`feature = "renzora"`).
//!
//! [Renzora](https://github.com/renzora/engine) is a Bevy 0.19 game engine with its
//! own Kira-based audio path (`renzora_audio` + `renzora::audio_backend::Backend`).
//!
//! RaMOD stays independent: enable this feature when you want first-class hooks
//! for embedding `bevy_ramod` inside a Renzora project (or any Bevy 0.19 host that
//! follows the same patterns).
//!
//! ## Recommended layout
//!
//! ```toml
//! # in a Renzora game / plugin crate
//! [dependencies]
//! bevy_ramod = { git = "https://github.com/SimonTek27/RaMOD", features = ["renzora"] }
//! # keep Bevy at the same major as Renzora (0.19)
//! ```
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_ramod::prelude::*;
//! use bevy_ramod::renzora::RenzoraRamodExt;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         // Prefer RaMOD as the game audio path (coexists with or replaces Kira
//!         // depending on which backends you register in the host).
//!         .add_plugins(RamodPlugin::default())
//!         .enable_renzora_ramod_markers()
//!         .run();
//! }
//! ```
//!
//! Full guide: `docs/RENZORA.md` in the RaMOD repository.

use bevy::prelude::*;

use crate::plugin::RamodPlugin;

/// Marker resource: this app is running under / targeting Renzora (Bevy 0.19).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct RenzoraHost {
    /// When true, Live Update is expected (editor ↔ game on :17420).
    pub live_update: bool,
}

/// Extension trait for ergonomic Renzora-oriented setup.
pub trait RenzoraRamodExt {
    /// Insert [`RenzoraHost`] and log that RaMOD is active as the audio path.
    fn enable_renzora_ramod_markers(&mut self) -> &mut Self;

    /// Same as [`RamodPlugin::default`] with explicit Renzora-oriented defaults
    /// (device on, live-update on).
    fn add_ramod_for_renzora(&mut self) -> &mut Self;
}

impl RenzoraRamodExt for App {
    fn enable_renzora_ramod_markers(&mut self) -> &mut Self {
        self.insert_resource(RenzoraHost {
            live_update: true,
        });
        bevy::log::info!(
            "bevy_ramod: Renzora host markers enabled (Bevy 0.19 / feature renzora)"
        );
        self
    }

    fn add_ramod_for_renzora(&mut self) -> &mut Self {
        self.add_plugins(RamodPlugin {
            open_device: true,
            enable_live_update: true,
        })
        .enable_renzora_ramod_markers()
    }
}

/// Version string for host diagnostics (editor / profiler panels).
pub fn bevy_ramod_renzora_bridge_version() -> &'static str {
    "0.1.0-bevy0.19-renzora"
}
