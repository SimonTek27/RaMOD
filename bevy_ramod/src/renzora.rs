//! Renzora Engine integration (`feature = "renzora"`).
//!
//! [Renzora](https://github.com/renzora/engine) is a Bevy 0.19 game engine with a
//! Kira-based default audio path. RaMOD can:
//!
//! 1. **Replace Kira entirely** via [`RenzoraRamodExt::replace_renzora_audio_with_ramod`].
//! 2. **Act as the Renzora mixer Backend** via [`backend::RamodRenzoraBackend`]
//!    (see `docs/renzora_backend_glue.rs` for zero-glue `impl Backend`).
//!
//! Full guide: `docs/RENZORA.md`.

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
    /// When true, host must **not** register a Kira backend (RaMOD is primary).
    pub replace_kira: bool,
}

/// Non-send holder for [`RamodRenzoraBackend`] when using Option B without
/// implementing Renzora's `Backend` trait in this crate (cpal / Runtime are not
/// `Sync`). Prefer the glue in `docs/renzora_backend_glue.rs` for `add_audio_backend`.
#[derive(Debug)]
pub struct RamodBackendCell {
    pub backend: RamodRenzoraBackend,
}

/// Extension trait for Renzora-oriented setup.
pub trait RenzoraRamodExt {
    /// Insert [`RenzoraHost`] markers (Kira still allowed).
    fn enable_renzora_ramod_markers(&mut self) -> &mut Self;

    /// `RamodPlugin` + markers (device + live-update on).
    fn add_ramod_for_renzora(&mut self) -> &mut Self;

    /// **Replace** Renzora/Kira audio with RaMOD only (Option A).
    ///
    /// Installs [`RamodPlugin`], sets `RenzoraHost { replace_kira: true }`.
    /// Do **not** call Renzora's `add_audio_backend` for Kira.
    fn replace_renzora_audio_with_ramod(&mut self) -> &mut Self;

    /// Insert a [`RamodBackendCell`] (NonSend) for hosts that drive the
    /// backend manually or via the glue in `docs/renzora_backend_glue.rs`.
    fn insert_ramod_renzora_backend(&mut self, open_device: bool) -> Result<&mut Self, String>;
}

impl RenzoraRamodExt for App {
    fn enable_renzora_ramod_markers(&mut self) -> &mut Self {
        self.insert_resource(RenzoraHost {
            live_update: true,
            replace_kira: false,
        });
        bevy::log::info!(
            "bevy_ramod: Renzora host markers enabled ({})",
            bevy_ramod_renzora_bridge_version()
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
             (replace_kira=true, {})",
            bevy_ramod_renzora_bridge_version()
        );
        self
    }

    fn insert_ramod_renzora_backend(&mut self, open_device: bool) -> Result<&mut Self, String> {
        let backend = RamodRenzoraBackend::new(open_device)?;
        self.insert_non_send_resource(RamodBackendCell { backend });
        bevy::log::info!(
            "bevy_ramod: RamodRenzoraBackend inserted as NonSend ({})",
            bevy_ramod_renzora_bridge_version()
        );
        Ok(self)
    }
}

/// Bridge version for host diagnostics / profiler panels.
pub fn bevy_ramod_renzora_bridge_version() -> &'static str {
    "0.2.1-bevy0.19-renzora"
}
