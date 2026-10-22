//! Renzora Engine integration (`feature = "renzora"`).
//!
//! [Renzora](https://github.com/renzora/engine) is a Bevy 0.19 game engine with a
//! Kira-based default audio path. RaMOD can:
//!
//! 1. **Run beside / instead of Kira** via [`RamodPlugin`] (recommended).
//! 2. **Act as the Renzora mixer Backend** via [`backend::RamodRenzoraBackend`].
//!
//! See `docs/RENZORA.md`.

#[path = "renzora_backend.rs"]
pub mod backend;

use bevy::prelude::*;

use crate::plugin::RamodPlugin;

pub use backend::{
    buses, BackendInfo, BusState, Caps, ClipInfo, DeviceList, EmitterState, ListenerState,
    PlayRequest, RamodRenzoraBackend, StopRequest, StopTarget, UpdateReply, UpdateRequest,
};

/// Marker resource: this app targets Renzora (Bevy 0.19).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct RenzoraHost {
    /// Live Update expected (editor ↔ game on :17420).
    pub live_update: bool,
    /// When true, host should **not** register a Kira backend (RaMOD is primary).
    pub replace_kira: bool,
}

/// Extension trait for Renzora-oriented setup.
pub trait RenzoraRamodExt {
    /// Insert [`RenzoraHost`] markers.
    fn enable_renzora_ramod_markers(&mut self) -> &mut Self;

    /// `RamodPlugin` + markers (device + live-update on). Keeps Kira optional.
    fn add_ramod_for_renzora(&mut self) -> &mut Self;

    /// **Replace** Renzora/Kira audio with RaMOD only.
    ///
    /// Installs [`RamodPlugin`], sets `RenzoraHost { replace_kira: true }`, and
    /// logs that the host must not call `add_audio_backend` for Kira.
    fn replace_renzora_audio_with_ramod(&mut self) -> &mut Self;
}

impl RenzoraRamodExt for App {
    fn enable_renzora_ramod_markers(&mut self) -> &mut Self {
        self.insert_resource(RenzoraHost {
            live_update: true,
            replace_kira: false,
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

    fn replace_renzora_audio_with_ramod(&mut self) -> &mut Self {
        self.add_plugins(RamodPlugin {
            open_device: true,
            enable_live_update: true,
        });
        self.insert_resource(RenzoraHost {
            live_update: true,
            replace_kira: true,
        });
        bevy::log::info!(
            "bevy_ramod: RaMOD is the primary audio path — do not register Kira \
             (RenzoraHost.replace_kira = true, bridge {})",
            bevy_ramod_renzora_bridge_version()
        );
        self
    }
}

/// Version string for host diagnostics.
pub fn bevy_ramod_renzora_bridge_version() -> &'static str {
    "0.2.0-bevy0.19-renzora-backend"
}
