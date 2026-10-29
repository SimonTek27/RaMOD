//! # renzora_ramod — RaMOD audio for [Renzora Engine](https://github.com/renzora/engine)
//!
//! Native plugin that installs [`bevy_ramod`] as the game audio path (Option A
//! in the RaMOD guide: replace Kira with RaMOD events, banks, Live Update).
//!
//! Full docs: https://github.com/SimonTek27/RaMOD/blob/main/docs/RENZORA.md

use bevy::prelude::*;
use bevy_ramod::prelude::*;
use bevy_ramod::renzora::RenzoraRamodExt;

/// Configuration for the RaMOD ↔ Renzora bridge.
///
/// Defaults match **Option A**: open the audio device, enable Live Update
/// (:17420), and mark Kira as replaced.
#[derive(Clone, Debug)]
pub struct RamodRenzoraSettings {
    /// Open the default cpal device.
    pub open_device: bool,
    /// Bind Live Update TCP port (editor → game).
    pub live_update: bool,
    /// When true, log that Kira must not register a backend.
    pub replace_kira: bool,
}

impl Default for RamodRenzoraSettings {
    fn default() -> Self {
        Self {
            open_device: true,
            live_update: true,
            replace_kira: true,
        }
    }
}

/// Renzora native plugin: RaMOD as primary audio middleware.
///
/// Registered with [`renzora::plugin!`] as **Runtime** so copy-based game
/// exports include the library.
#[derive(Default)]
pub struct RamodRenzoraPlugin {
    pub settings: RamodRenzoraSettings,
}

impl Plugin for RamodRenzoraPlugin {
    fn build(&self, app: &mut App) {
        let s = &self.settings;
        info!(
            "renzora_ramod: installing RaMOD (open_device={}, live_update={}, replace_kira={})",
            s.open_device, s.live_update, s.replace_kira
        );

        if s.replace_kira {
            // Option A — full RaMOD path; host must not register Kira.
            app.replace_renzora_audio_with_ramod();
        } else {
            app.add_plugins(RamodPlugin {
                open_device: s.open_device,
                enable_live_update: s.live_update,
            })
            .add_ramod_for_renzora();
        }

        app.add_systems(Startup, log_ramod_ready);
    }
}

fn log_ramod_ready() {
    info!(
        "renzora_ramod: ready — use RamodAudio / LoadBank / SpatialAudioBundle \
         (see docs/RENZORA.md). Live Update :17420 when enabled."
    );
}

// Runtime: ship with exported games (not editor-only).
renzora::plugin!(RamodRenzoraPlugin, Runtime);
