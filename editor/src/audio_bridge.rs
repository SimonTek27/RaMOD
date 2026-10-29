//! Transport Play → ramod_runtime (audible via cpal when available).
//!
//! The bridge is the only place the editor touches the engine: it registers the
//! project's events with the [`Runtime`], drives rendering from the UI frame
//! clock ([`AudioBridge::pump`]), and exposes the mixer / profiler state the
//! events are authored against.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;

use ramod_runtime::{
    Attributes3d, BufferHandle, BusId, BusMeterInfo, EmitterDebug, InsertEffect, PlayParams,
    Runtime, RuntimeConfig, Vec3, VoiceHandle,
};
use ramod_waveform::WaveBuffer;

use crate::project::{AssetEntry, EventEntry, Project};

/// One row per insert slot, for the profiler's FX editor.
#[derive(Debug, Clone, PartialEq)]
pub struct BusInsertInfo {
    pub bus_id: u32,
    pub index: usize,
    /// `Gain` | `LowPass` | `HighPass` | `Compressor` | `Sidechain` | `Reverb`.
    pub kind: String,
    pub p0: f32,
    pub p1: f32,
    pub p2: f32,
    /// Side-chain key bus (only meaningful for `Sidechain`).
    pub source_bus: String,
}

/// Everything the profiler panel draws, captured without holding the runtime.
#[derive(Debug, Clone, Default)]
pub struct ProfilerSnapshot {
    pub session_frames: usize,
    pub cpu_render_us: u32,
    pub avg_cpu_us: f32,
    pub active_voices: usize,
    pub peak_voices: usize,
    pub cpu_history: Vec<u32>,
    pub voice_history: Vec<u32>,
    pub bus_meters: Vec<BusMeterInfo>,
    pub insert_counts: Vec<(u32, usize)>,
    pub bus_inserts: Vec<BusInsertInfo>,
    pub snapshot_names: Vec<String>,
    /// Listener position `(x, y, z)` in metres.
    pub listener: (f32, f32, f32),
    pub emitters: Vec<EmitterDebug>,
}

pub struct AudioBridge {
    rt: Runtime,
    /// Signature of the event definition last handed to the runtime.
    registered: HashMap<String, u64>,
    /// Decoded PCM per asset, keyed by source path or demo tone.
    asset_cache: HashMap<String, BufferHandle>,
    playing_id: Option<u64>,
    preview_voice: Option<VoiceHandle>,
    /// Parameter values re-applied to every new instance.
    parameters: HashMap<String, f32>,
    listener_pos: (f32, f32, f32),
    listener_vel: (f32, f32, f32),
    emitter_pos: (f32, f32, f32),
    emitter_vel: (f32, f32, f32),
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
            registered: HashMap::new(),
            asset_cache: HashMap::new(),
            playing_id: None,
            preview_voice: None,
            parameters: HashMap::new(),
            listener_pos: (0.0, 0.0, 0.0),
            listener_vel: (0.0, 0.0, 0.0),
            emitter_pos: (0.0, 0.0, 10.0),
            emitter_vel: (0.0, 0.0, 0.0),
        };
        bridge.rt.profiler_enable(true);
        Ok(bridge)
    }

    #[allow(dead_code)]
    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.rt
    }

    #[allow(dead_code)]
    pub fn runtime(&self) -> &Runtime {
        &self.rt
    }

    // ---------------------------------------------------------------- mixing

    pub fn set_bus_volume_db(&mut self, bus_id: u32, db: f32) {
        self.rt
            .set_bus_volume_db(BusId(bus_id), db.clamp(-60.0, 24.0));
    }

    #[allow(dead_code)] // mixer API exposed for the send UI
    pub fn set_bus_send_reverb(&mut self, bus_id: u32, level: f32) {
        self.rt
            .mixer_mut()
            .set_send_level(BusId(bus_id), "Reverb", level.clamp(0.0, 1.0));
    }

    pub fn toggle_bus_mute(&mut self, bus_id: u32) {
        let muted = self
            .rt
            .mixer()
            .bus(BusId(bus_id))
            .map(|b| b.mute)
            .unwrap_or(false);
        self.rt.set_bus_mute(BusId(bus_id), !muted);
    }

    pub fn toggle_bus_solo(&mut self, bus_id: u32) {
        let solo = self
            .rt
            .mixer()
            .bus(BusId(bus_id))
            .map(|b| b.solo)
            .unwrap_or(false);
        self.rt.set_bus_solo(BusId(bus_id), !solo);
    }

    // ------------------------------------------------------------- transport

    pub fn stop(&mut self) {
        if let Some(id) = self.playing_id.take() {
            self.rt.release_instance(id);
        }
        if let Some(v) = self.preview_voice.take() {
            self.rt.stop(v);
        }
    }

    /// Register every event of `project` with the runtime.
    ///
    /// Asset PCM is decoded once and cached; an event is only re-registered when
    /// its authored definition actually changed.
    pub fn sync_project(&mut self, project: &Project) {
        let mut slot_of: Vec<Option<usize>> = Vec::with_capacity(project.assets.len());
        let mut handles: Vec<BufferHandle> = Vec::new();
        for asset in &project.assets {
            let key = asset_key(asset);
            let handle = match self.asset_cache.get(&key).copied() {
                Some(h) => Some(h),
                None => {
                    let loaded = self.load_asset(asset);
                    if let Some(h) = loaded {
                        self.asset_cache.insert(key, h);
                    }
                    loaded
                }
            };
            match handle {
                Some(h) => {
                    handles.push(h);
                    slot_of.push(Some(handles.len() - 1));
                }
                None => slot_of.push(None),
            }
        }

        for event in &project.events {
            let signature = event_signature(event);
            if self.registered.get(&event.path) == Some(&signature) {
                continue;
            }
            let Some(designed) = remap_slots(event, &slot_of) else {
                continue;
            };
            if self.rt.register_designed_event(designed, &handles).is_ok() {
                self.registered.insert(event.path.clone(), signature);
            }
        }
    }

    pub fn play_event(&mut self, path: &str) -> Result<(), String> {
        if let Some(id) = self.playing_id.take() {
            self.rt.release_instance(id);
        }
        if let Some(v) = self.preview_voice.take() {
            self.rt.stop(v);
        }
        let id = self.rt.play_event(path).map_err(|e| e.to_string())?;
        self.playing_id = Some(id);
        let parameters: Vec<(String, f32)> = self
            .parameters
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (name, value) in parameters {
            self.rt.set_instance_parameter(id, &name, value);
        }
        self.apply_emitter();
        Ok(())
    }

    /// Set a named parameter on the live instance (and remember it for replays).
    pub fn set_parameter(&mut self, name: &str, value: f32) {
        self.parameters.insert(name.to_string(), value);
        if let Some(id) = self.playing_id {
            self.rt.set_instance_parameter(id, name, value);
        }
        if let (Some(voice), true) = (self.preview_voice, name.eq_ignore_ascii_case("Intensity")) {
            self.rt.set_voice_gain(voice, value.clamp(0.0, 4.0));
        }
    }

    /// Bus name for a mixer id, so live-update messages can address it by name.
    pub fn bus_name(&self, bus_id: u32) -> Option<String> {
        self.rt.mixer().bus(BusId(bus_id)).map(|b| b.name.clone())
    }

    /// Current `(mute, solo)` of a bus.
    pub fn bus_flags(&self, bus_id: u32) -> Option<(bool, bool)> {
        self.rt.mixer().bus(BusId(bus_id)).map(|b| (b.mute, b.solo))
    }

    pub fn master_levels(&self) -> (f32, f32) {
        let mixer = self.rt.mixer();
        mixer
            .bus(mixer.master_id())
            .map(|b| (b.meter.peak, b.meter.rms))
            .unwrap_or((0.0, 0.0))
    }

    /// Preview raw PCM (waveform editor) on its own voice.
    pub fn play_pcm(
        &mut self,
        samples: Vec<f32>,
        channels: u16,
        sample_rate: u32,
    ) -> Result<(), String> {
        if samples.is_empty() {
            return Err("buffer is empty".into());
        }
        if let Some(v) = self.preview_voice.take() {
            self.rt.stop(v);
        }
        if let Some(id) = self.playing_id.take() {
            self.rt.release_instance(id);
        }
        let buffer = self
            .rt
            .load_pcm(samples, channels, sample_rate)
            .map_err(|e| e.to_string())?;
        let voice = self
            .rt
            .play(
                buffer,
                PlayParams {
                    gain: 1.0,
                    looped: false,
                },
            )
            .map_err(|e| e.to_string())?;
        self.preview_voice = Some(voice);
        Ok(())
    }

    /// Render one UI frame of audio; the runtime pushes it to the device.
    pub fn pump(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.25);
        let channels = self.rt.channels().max(1) as usize;
        let frames = (dt * self.rt.sample_rate() as f32).round() as usize;
        if frames == 0 {
            return;
        }
        let mut out = vec![0.0f32; frames * channels];
        self.rt.render(&mut out);
    }

    // ------------------------------------------------------------- profiler

    pub fn profiler_snapshot(&self) -> ProfilerSnapshot {
        let session = self.rt.profiler_session();
        let frames = &session.frames;
        const HISTORY: usize = 120;
        let start = frames.len().saturating_sub(HISTORY);
        let window = &frames[start..];

        let mut bus_meters: Vec<BusMeterInfo> = self
            .rt
            .mixer()
            .buses()
            .map(|b| BusMeterInfo {
                id: b.id.0,
                name: b.name.clone(),
                peak: b.meter.peak,
                rms: b.meter.rms,
                mute: b.mute,
                solo: b.solo,
                volume_db: b.volume_db,
                send_reverb: self.rt.mixer().send_level(b.id, "Reverb"),
                is_return: b.is_return,
                parent_id: b.parent.map(|p| p.0),
            })
            .collect();
        bus_meters.sort_by_key(|b| b.id);

        let mut insert_counts = Vec::new();
        let mut bus_inserts = Vec::new();
        for bus in self.rt.mixer().buses() {
            insert_counts.push((bus.id.0, bus.inserts.len()));
            for (index, fx) in bus.inserts.iter().enumerate() {
                bus_inserts.push(insert_info(bus.id.0, index, fx));
            }
        }
        insert_counts.sort_by_key(|(id, _)| *id);
        bus_inserts.sort_by(|a, b| a.bus_id.cmp(&b.bus_id).then(a.index.cmp(&b.index)));

        let mut emitters = session
            .latest()
            .map(|f| f.emitters.clone())
            .unwrap_or_default();
        if emitters.is_empty() && self.playing_id.is_some() {
            emitters.push(EmitterDebug {
                position: Vec3::new(self.emitter_pos.0, self.emitter_pos.1, self.emitter_pos.2),
                velocity: Vec3::new(self.emitter_vel.0, self.emitter_vel.1, self.emitter_vel.2),
                audibility: 1.0,
            });
        }

        ProfilerSnapshot {
            session_frames: session.len(),
            cpu_render_us: self.rt.last_render_cpu_us(),
            avg_cpu_us: session.avg_cpu_us(),
            active_voices: self.rt.active_voices(),
            peak_voices: session.peak_voices(),
            cpu_history: window.iter().map(|f| f.cpu_render_us).collect(),
            voice_history: window.iter().map(|f| f.active_voices as u32).collect(),
            bus_meters,
            insert_counts,
            bus_inserts,
            snapshot_names: self.rt.snapshot_names(),
            listener: self.listener_pos,
            emitters,
        }
    }

    pub fn profiler_clear(&mut self) {
        self.rt.profiler_clear();
    }

    pub fn profiler_ndjson(&self) -> String {
        self.rt.profiler_session().to_ndjson()
    }

    // ------------------------------------------------------------ snapshots

    pub fn capture_snapshot_base(&mut self) {
        self.rt.capture_snapshot_base();
    }

    pub fn capture_named_snapshot(&mut self, name: &str) {
        let name = name.trim();
        if !name.is_empty() {
            self.rt.capture_named_snapshot(name);
        }
    }

    pub fn trigger_snapshot(&mut self, name: &str, weight: f32) {
        self.rt.trigger_snapshot(name, weight);
    }

    pub fn clear_snapshot(&mut self) {
        self.rt.clear_snapshot();
    }

    // ------------------------------------------------------------------- 3D

    pub fn set_emitter_xz(&mut self, x: f32, z: f32, vx: f32, vz: f32) {
        self.emitter_pos = (x, 0.0, z);
        self.emitter_vel = (vx, 0.0, vz);
        self.apply_emitter();
    }

    pub fn set_listener_xz(&mut self, x: f32, z: f32, vx: f32, vz: f32) {
        self.listener_pos = (x, 0.0, z);
        self.listener_vel = (vx, 0.0, vz);
        self.rt.set_listener(Attributes3d {
            position: Vec3::new(x, 0.0, z),
            velocity: Vec3::new(vx, 0.0, vz),
            ..Default::default()
        });
    }

    fn apply_emitter(&mut self) {
        let Some(id) = self.playing_id else {
            return;
        };
        self.rt.set_instance_3d(
            id,
            Attributes3d {
                position: Vec3::new(self.emitter_pos.0, self.emitter_pos.1, self.emitter_pos.2),
                velocity: Vec3::new(self.emitter_vel.0, self.emitter_vel.1, self.emitter_vel.2),
                ..Default::default()
            },
        );
    }

    // ------------------------------------------------------------- bus FX

    pub fn add_bus_lowpass(&mut self, bus_id: u32) {
        self.add_insert(bus_id, InsertEffect::lowpass(8_000.0));
    }

    pub fn add_bus_highpass(&mut self, bus_id: u32) {
        self.add_insert(bus_id, InsertEffect::highpass(80.0));
    }

    pub fn add_bus_compressor(&mut self, bus_id: u32) {
        self.add_insert(bus_id, InsertEffect::compressor(-18.0, 4.0, 6.0));
    }

    pub fn add_bus_sidechain(&mut self, bus_id: u32) {
        self.add_insert(
            bus_id,
            InsertEffect::sidechain_compressor(-20.0, 6.0, 4.0, "SFX"),
        );
    }

    pub fn add_bus_reverb(&mut self, bus_id: u32) {
        self.add_insert(bus_id, InsertEffect::reverb(55.0, 0.4, 0.35));
    }

    fn add_insert(&mut self, bus_id: u32, effect: InsertEffect) {
        self.rt.mixer_mut().add_insert(BusId(bus_id), effect);
    }

    pub fn clear_bus_inserts(&mut self, bus_id: u32) {
        self.rt.mixer_mut().clear_inserts(BusId(bus_id));
    }

    pub fn remove_bus_insert(&mut self, bus_id: u32, index: usize) {
        self.rt.mixer_mut().remove_insert(BusId(bus_id), index);
    }

    /// Apply edited parameters to an existing insert slot.
    pub fn set_bus_insert_info(&mut self, info: &BusInsertInfo) {
        let Some(effect) = info_to_insert(info) else {
            return;
        };
        self.rt
            .mixer_mut()
            .set_insert(BusId(info.bus_id), info.index, effect);
    }

    /// Capture every bus insert as `bus|Kind|p0|p1|p2|source` strings.
    pub fn capture_fx_chain(&self) -> Vec<String> {
        let mut chain: Vec<String> = self
            .rt
            .mixer()
            .buses()
            .flat_map(|bus| {
                bus.inserts
                    .iter()
                    .enumerate()
                    .map(move |(index, fx)| encode_insert(bus.id.0, index, fx))
            })
            .collect();
        chain.sort();
        chain
    }

    /// Replace the whole FX graph with a captured chain.
    pub fn apply_fx_chain(&mut self, chain: &[String]) {
        let bus_ids: Vec<u32> = self.rt.mixer().buses().map(|b| b.id.0).collect();
        for id in bus_ids {
            self.rt.mixer_mut().clear_inserts(BusId(id));
        }
        for entry in chain {
            let Some(info) = decode_insert(entry) else {
                continue;
            };
            if let Some(effect) = info_to_insert(&info) {
                self.rt.mixer_mut().add_insert(BusId(info.bus_id), effect);
            }
        }
    }

    // -------------------------------------------------------------- assets

    fn load_asset(&mut self, asset: &AssetEntry) -> Option<BufferHandle> {
        let (samples, channels, sample_rate) = match &asset.source_path {
            Some(path) => {
                let buffer = WaveBuffer::from_file(Path::new(path)).ok()?;
                (buffer.samples, buffer.channels, buffer.sample_rate)
            }
            None => {
                let rate = self.rt.sample_rate();
                (
                    synth_tone(asset.demo_freq_hz.unwrap_or(440.0), 0.5, rate),
                    1,
                    rate,
                )
            }
        };
        self.rt.load_pcm(samples, channels, sample_rate).ok()
    }
}

fn asset_key(asset: &AssetEntry) -> String {
    match (&asset.source_path, asset.demo_freq_hz) {
        (Some(path), _) => format!("file:{path}"),
        (None, Some(hz)) => format!("tone:{hz}"),
        (None, None) => format!("asset:{}", asset.name),
    }
}

/// FNV-style signature over the serialized definition.
fn event_signature(event: &EventEntry) -> u64 {
    let mut hasher = DefaultHasher::new();
    serde_json::to_string(event)
        .unwrap_or_default()
        .hash(&mut hasher);
    hasher.finish()
}

/// Copy the authored event and rewrite asset indices to packed buffer slots.
///
/// Returns `None` when the event has nothing playable (no tracks/instruments, or
/// an instrument whose asset never decoded).
fn remap_slots(
    event: &EventEntry,
    slot_of: &[Option<usize>],
) -> Option<ramod_runtime::DesignedEvent> {
    let mut designed = event.to_designed();
    if designed.tracks.is_empty() {
        return None;
    }
    for track in &mut designed.tracks {
        for instrument in &mut track.instruments {
            let mut slots = Vec::with_capacity(instrument.buffer_slots.len());
            for asset_index in &instrument.buffer_slots {
                slots.push(slot_of.get(*asset_index).copied().flatten()?);
            }
            if slots.is_empty() {
                return None;
            }
            instrument.buffer_slots = slots;
        }
    }
    Some(designed)
}

fn synth_tone(freq: f32, seconds: f32, sample_rate: u32) -> Vec<f32> {
    let frames = (seconds.max(0.01) * sample_rate as f32) as usize;
    let fade = (frames / 20).max(1);
    (0..frames)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let env = if i < fade {
                i as f32 / fade as f32
            } else if i + fade > frames {
                (frames - i) as f32 / fade as f32
            } else {
                1.0
            };
            0.3 * env * (2.0 * std::f32::consts::PI * freq.max(20.0) * t).sin()
        })
        .collect()
}

fn insert_info(bus_id: u32, index: usize, effect: &InsertEffect) -> BusInsertInfo {
    let (kind, p0, p1, p2, source_bus) = match effect {
        InsertEffect::Gain { gain_db } => ("Gain", *gain_db, 0.0, 0.0, String::new()),
        InsertEffect::LowPass { cutoff_hz } => ("LowPass", *cutoff_hz, 0.0, 0.0, String::new()),
        InsertEffect::HighPass { cutoff_hz } => ("HighPass", *cutoff_hz, 0.0, 0.0, String::new()),
        InsertEffect::Compressor {
            threshold_db,
            ratio,
            makeup_db,
        } => (
            "Compressor",
            *threshold_db,
            *ratio,
            *makeup_db,
            String::new(),
        ),
        InsertEffect::SidechainCompressor {
            threshold_db,
            ratio,
            makeup_db,
            source_bus,
        } => (
            "Sidechain",
            *threshold_db,
            *ratio,
            *makeup_db,
            source_bus.clone(),
        ),
        InsertEffect::Reverb {
            delay_ms,
            feedback,
            wet,
        } => ("Reverb", *delay_ms, *feedback, *wet, String::new()),
    };
    BusInsertInfo {
        bus_id,
        index,
        kind: kind.to_string(),
        p0,
        p1,
        p2,
        source_bus,
    }
}

fn info_to_insert(info: &BusInsertInfo) -> Option<InsertEffect> {
    Some(match info.kind.as_str() {
        "Gain" => InsertEffect::gain(info.p0),
        "LowPass" => InsertEffect::lowpass(info.p0),
        "HighPass" => InsertEffect::highpass(info.p0),
        "Compressor" => InsertEffect::compressor(info.p0, info.p1.max(1.0), info.p2),
        "Sidechain" => InsertEffect::sidechain_compressor(
            info.p0,
            info.p1.max(1.0),
            info.p2,
            if info.source_bus.is_empty() {
                "SFX"
            } else {
                info.source_bus.as_str()
            },
        ),
        "Reverb" => InsertEffect::reverb(info.p0, info.p1, info.p2),
        _ => return None,
    })
}

fn encode_insert(bus_id: u32, _index: usize, effect: &InsertEffect) -> String {
    let info = insert_info(bus_id, _index, effect);
    format!(
        "{}|{}|{}|{}|{}|{}",
        info.bus_id, info.kind, info.p0, info.p1, info.p2, info.source_bus
    )
}

fn decode_insert(entry: &str) -> Option<BusInsertInfo> {
    let mut parts = entry.split('|');
    let bus_id = parts.next()?.parse::<u32>().ok()?;
    let kind = parts.next()?.to_string();
    let p0 = parts.next()?.parse::<f32>().ok()?;
    let p1 = parts.next()?.parse::<f32>().ok()?;
    let p2 = parts.next()?.parse::<f32>().ok()?;
    let source_bus = parts.next().unwrap_or("").to_string();
    Some(BusInsertInfo {
        bus_id,
        index: 0,
        kind,
        p0,
        p1,
        p2,
        source_bus,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_round_trip_through_the_preset_format() {
        let effect = InsertEffect::sidechain_compressor(-20.0, 6.0, 4.0, "Music");
        let encoded = encode_insert(3, 0, &effect);
        let decoded = decode_insert(&encoded).expect("decodes");
        assert_eq!(decoded.bus_id, 3);
        assert_eq!(decoded.kind, "Sidechain");
        assert_eq!(decoded.source_bus, "Music");
        assert!((decoded.p0 + 20.0).abs() < 1e-6);
    }

    #[test]
    fn unknown_insert_kind_is_rejected() {
        assert!(info_to_insert(&BusInsertInfo {
            bus_id: 0,
            index: 0,
            kind: "Chorus".into(),
            p0: 0.0,
            p1: 0.0,
            p2: 0.0,
            source_bus: String::new(),
        })
        .is_none());
    }

    #[test]
    fn demo_tone_is_audible_and_click_free() {
        let tone = synth_tone(440.0, 0.05, 48_000);
        assert_eq!(tone.len(), 2_400);
        assert!(tone.iter().any(|s| s.abs() > 0.1));
        assert!(tone[0].abs() < 0.02, "starts at silence");
        assert!(tone[tone.len() - 1].abs() < 0.02, "ends at silence");
    }

    #[test]
    fn remap_skips_events_without_playable_assets() {
        let mut event = Project::default().events.remove(0);
        assert!(remap_slots(&event, &[None]).is_none());
        event.tracks[0].instruments[0].asset_index = Some(0);
        assert!(remap_slots(&event, &[Some(0)]).is_some());
        event.tracks.clear();
        assert!(remap_slots(&event, &[Some(0)]).is_none());
    }

    #[test]
    fn register_and_replay_through_the_bridge() {
        let Ok(mut bridge) = AudioBridge::new() else {
            return;
        };
        let project = Project::default();
        bridge.sync_project(&project);
        bridge
            .play_event("event:/sfx/beep")
            .expect("event registered");
        bridge.set_parameter("Intensity", 0.5);
        bridge.pump(0.01);
        let (peak, _) = bridge.master_levels();
        assert!(peak >= 0.0);
        bridge.stop();
    }
}
