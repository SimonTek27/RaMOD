//! Transport Play → ramod_runtime (audible via cpal when available).

use std::path::Path;

use ramod_runtime::{
    Ahdsr, Attenuation, Attributes3d, AutomationCurve, BusId, BusMeterInfo, DesignedEvent,
    EmitterDebug, EventDescription, InsertEffect, InstrumentDef, PlayParams, Runtime,
    RuntimeConfig, TimelineTrack, TransitionMarker, Vec3,
};
use ramod_waveform::WaveBuffer;

use crate::project::{AssetEntry, EventEntry, Project};

pub struct AudioBridge {
    rt: Runtime,
    registered: Vec<String>,
    playing_id: Option<u64>,
    preview_voice: Option<ramod_runtime::VoiceHandle>,
}

impl AudioBridge {
    pub fn new() -> Result<Self, String> {
        let rt = match Runtime::new(RuntimeConfig {
            open_device: true,
            sample_rate: 48_000,
            channels: 2,
            max_voices: 32,
        }) {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("cpal open failed ({e}), using headless mix");
                Runtime::new(RuntimeConfig {
                    open_device: false,
                    sample_rate: 48_000,
                    channels: 2,
                    max_voices: 32,
                })
                .map_err(|e| e.to_string())?
            }
        };
        let mut bridge = Self {
            rt,
            registered: Vec::new(),
            playing_id: None,
            preview_voice: None,
        };
        bridge.rt.profiler_enable(true);
        Ok(bridge)
    }

    pub fn set_bus_volume_db(&mut self, bus_id: u32, db: f32) {
        self.rt.set_bus_volume_db(BusId(bus_id), db.clamp(-60.0, 24.0));
    }

    pub fn set_bus_send_reverb(&mut self, bus_id: u32, level: f32) {
        self.rt.set_bus_send_level(BusId(bus_id), "Reverb", level.clamp(0.0, 1.0));
    }

    pub fn toggle_bus_mute(&mut self, bus_id: u32) {
        let id = BusId(bus_id);
        let muted = self
            .rt
            .list_bus_meters()
            .iter()
            .find(|b| b.id == bus_id)
            .map(|b| b.mute)
            .unwrap_or(false);
        self.rt.set_bus_mute(id, !muted);
    }

    pub fn toggle_bus_solo(&mut self, bus_id: u32) {
        let id = BusId(bus_id);
        let solo = self
            .rt
            .list_bus_meters()
            .iter()
            .find(|b| b.id == bus_id)
            .map(|b| b.solo)
            .unwrap_or(false);
        self.rt.set_bus_solo(id, !solo);
    }

    // NOTE: remaining AudioBridge methods (sync_project, play, profiler, FX…)
    // live in the full historical revision — see commit f33d4403 if this
    // simplified bridge is incomplete for your tree.
    pub fn stop(&mut self) {
        if let Some(id) = self.playing_id.take() {
            self.rt.release_instance(id);
        }
        if let Some(v) = self.preview_voice.take() {
            self.rt.stop(v);
        }
        self.rt.clear_scheduled();
    }
}
