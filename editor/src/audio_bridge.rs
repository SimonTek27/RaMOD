//! Transport Play → ramod_runtime (audible via cpal when available).

use std::path::Path;

use ramod_runtime::{
    Attenuation, Attributes3d, AutomationCurve, BusId, BusMeterInfo, DesignedEvent,
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

    pub fn sync_project(&mut self, project: &Project) {
        self.stop();
        self.registered.clear();
        if let Ok(rt) = Runtime::new(RuntimeConfig {
            open_device: true,
            sample_rate: 48_000,
            channels: 2,
            max_voices: 32,
        })
        .or_else(|_| {
            Runtime::new(RuntimeConfig {
                open_device: false,
                sample_rate: 48_000,
                channels: 2,
                max_voices: 32,
            })
        }) {
            self.rt = rt;
            self.rt.profiler_enable(true);
        }

        for ev in &project.events {
            if !ev.tracks.is_empty() {
                let mut buffers = Vec::new();
                let mut designed = DesignedEvent::new(&ev.path);
                designed.priority = ev.priority;
                designed.is_3d = ev.is_3d;
                designed.length = ev.length;
                designed.parameters = ev.parameters.clone();
                designed.output_bus = ev.output_bus.clone();
                for a in &ev.automation {
                    let mut curve = AutomationCurve::new(&a.parameter);
                    for &(t, v) in &a.keys {
                        curve.add_key(t, v);
                    }
                    designed.automation.push(curve);
                }
                for m in &ev.markers {
                    designed.markers.push(TransitionMarker {
                        name: m.name.clone(),
                        time: m.time,
                        target_event: m.target_event.clone(),
                    });
                }
                if ev.is_3d {
                    designed.attenuation = Attenuation {
                        min_distance: ev.min_distance,
                        max_distance: ev.max_distance,
                    };
                }
                for tr in &ev.tracks {
                    let mut track = TimelineTrack::new(&tr.name);
                    track.mute = tr.mute;
                    track.solo = tr.solo;
                    for inst in &tr.instruments {
                        let pcm = resolve_instrument_pcm(project, inst.asset_index, ev);
                        let Ok(buf) =
                            self.rt.load_pcm(pcm.samples, pcm.channels, pcm.sample_rate)
                        else {
                            continue;
                        };
                        let slot = buffers.len();
                        buffers.push(buf);
                        let mut idef = InstrumentDef::single(&inst.name, slot);
                        idef.start_time = inst.start_time;
                        idef.length = inst.length;
                        idef.probability = inst.probability;
                        idef.looping = inst.looping;
                        track.instruments.push(idef);
                    }
                    designed.tracks.push(track);
                }
                if !buffers.is_empty() {
                    if self.rt.register_designed_event(designed, &buffers).is_ok() {
                        self.registered.push(ev.path.clone());
                    }
                    continue;
                }
            }

            let pcm = resolve_asset_pcm(project, ev);
            let Ok(buf) = self.rt.load_pcm(pcm.samples, pcm.channels, pcm.sample_rate) else {
                continue;
            };
            let mut desc = EventDescription::new(&ev.path, buf)
                .with_loop(ev.looping)
                .with_priority(ev.priority);
            if ev.is_3d {
                desc = desc.with_3d(Attenuation {
                    min_distance: ev.min_distance,
                    max_distance: ev.max_distance,
                });
            }
            for (name, default) in &ev.parameters {
                desc = desc.with_parameter(name.clone(), *default);
            }
            desc = desc.with_output_bus(ev.output_bus.clone());
            self.rt.register_event(desc);
            self.registered.push(ev.path.clone());
        }
    }

    pub fn play_event(&mut self, path: &str) -> Result<(), String> {
        self.stop();
        self.rt.clear_fired_markers();
        let id = self.rt.play_event(path).map_err(|e| e.to_string())?;
        self.playing_id = Some(id);
        Ok(())
    }

    pub fn play_pcm(
        &mut self,
        samples: Vec<f32>,
        channels: u16,
        sample_rate: u32,
    ) -> Result<(), String> {
        self.stop();
        let buf = self
            .rt
            .load_pcm(samples, channels, sample_rate)
            .map_err(|e| e.to_string())?;
        let voice = self
            .rt
            .play(buf, PlayParams::default())
            .map_err(|e| e.to_string())?;
        self.preview_voice = Some(voice);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(id) = self.playing_id.take() {
            self.rt.release_instance(id);
        }
        if let Some(v) = self.preview_voice.take() {
            self.rt.stop(v);
        }
        self.rt.clear_scheduled();
    }

    pub fn set_parameter(&mut self, name: &str, value: f32) {
        if let Some(id) = self.playing_id {
            self.rt.set_instance_parameter(id, name, value);
        }
    }

    pub fn pump(&mut self, dt: f32) {
        self.rt.tick(dt);
        if let Some(id) = self.rt.current_designed_instance() {
            self.playing_id = Some(id);
        }
        let frames = ((dt * self.rt.sample_rate() as f32).max(1.0) as usize).min(4096);
        let mut out = vec![0.0f32; frames * self.rt.channels() as usize];
        self.rt.render(&mut out);
        let _ = out;
        if self.preview_voice.is_some() && self.rt.active_voices() == 0 {
            self.preview_voice = None;
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing_id.is_some() || self.preview_voice.is_some()
    }

    pub fn master_levels(&self) -> (f32, f32) {
        self.rt.master_levels()
    }

    pub fn last_fired_markers(&self) -> Vec<String> {
        self.rt.last_fired_markers()
    }

    pub fn enable_profiler(&mut self, on: bool) {
        self.rt.profiler_enable(on);
    }

    pub fn set_transition_fade(&mut self, secs: f32) {
        self.rt.set_transition_fade(secs);
    }

    pub fn profiler_snapshot(&self) -> ProfilerSnapshot {
        let frame = self.rt.profiler_latest();
        let (peak, rms) = self.rt.master_levels();
        ProfilerSnapshot {
            cpu_render_us: frame
                .map(|f| f.cpu_render_us)
                .unwrap_or(self.rt.last_render_cpu_us()),
            active_voices: frame
                .map(|f| f.active_voices)
                .unwrap_or_else(|| self.rt.active_voices()),
            active_instances: frame
                .map(|f| f.active_instances)
                .unwrap_or_else(|| self.rt.instance_count()),
            master_peak: frame.map(|f| f.master_peak).unwrap_or(peak),
            master_rms: frame.map(|f| f.master_rms).unwrap_or(rms),
            playing_events: frame.map(|f| f.playing_events.clone()).unwrap_or_default(),
            spectrum_bands: self.rt.spectrum().levels.clone(),
            session_frames: self.rt.profiler_session().len(),
            avg_cpu_us: self.rt.profiler_session().avg_cpu_us(),
            peak_voices: self.rt.profiler_session().peak_voices(),
            bus_meters: self.rt.list_bus_meters(),
            cpu_history: {
                let sess = self.rt.profiler_session();
                let n = sess.len().min(120);
                sess.frames[sess.len().saturating_sub(n)..]
                    .iter()
                    .map(|f| f.cpu_render_us)
                    .collect()
            },
            voice_history: {
                let sess = self.rt.profiler_session();
                let n = sess.len().min(120);
                sess.frames[sess.len().saturating_sub(n)..]
                    .iter()
                    .map(|f| f.active_voices as u32)
                    .collect()
            },
            snapshot_names: self.rt.snapshot_names(),
            active_snapshot: self.rt.active_snapshot(),
            emitters: frame.map(|f| f.emitters.clone()).unwrap_or_default(),
            listener: {
                let l = self.rt.listener();
                (l.position.x, l.position.y, l.position.z)
            },
            insert_counts: self
                .rt
                .list_bus_meters()
                .iter()
                .map(|b| (b.id, self.rt.bus_insert_count(BusId(b.id))))
                .collect(),
            bus_inserts: {
                let mut out = Vec::new();
                for b in self.rt.list_bus_meters() {
                    for (i, fx) in self.rt.list_bus_inserts(BusId(b.id)).into_iter().enumerate()
                    {
                        out.push(BusInsertInfo::from_effect(b.id, i, &fx));
                    }
                }
                out
            },
        }
    }

    pub fn profiler_clear(&mut self) {
        self.rt.profiler_clear();
    }

    pub fn profiler_ndjson(&self) -> String {
        self.rt.profiler_session().to_ndjson()
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

    pub fn set_bus_volume_db(&mut self, bus_id: u32, db: f32) {
        self.rt.set_bus_volume_db(BusId(bus_id), db.clamp(-60.0, 24.0));
    }

    pub fn capture_snapshot_base(&mut self) {
        self.rt.capture_snapshot_base();
    }

    pub fn capture_named_snapshot(&mut self, name: &str) {
        self.rt.capture_named_snapshot(name);
    }

    pub fn trigger_snapshot(&mut self, name: &str, weight: f32) {
        self.rt.trigger_snapshot(name, weight);
    }

    pub fn clear_snapshot(&mut self) {
        self.rt.clear_snapshot();
    }

    pub fn snapshot_names(&self) -> Vec<String> {
        self.rt.snapshot_names()
    }

    pub fn active_snapshot(&self) -> Option<(String, f32)> {
        self.rt.active_snapshot()
    }

    pub fn set_emitter_xz(&mut self, x: f32, z: f32, vx: f32, vz: f32) {
        let Some(id) = self.playing_id else {
            return;
        };
        let mut attrs = Attributes3d::default();
        attrs.position = Vec3::new(x, 0.0, z);
        attrs.velocity = Vec3::new(vx, 0.0, vz);
        self.rt.set_instance_3d(id, attrs);
    }

    pub fn set_listener_xz(&mut self, x: f32, z: f32, vx: f32, vz: f32) {
        let mut a = self.rt.listener();
        a.position.x = x;
        a.position.z = z;
        a.velocity = Vec3::new(vx, 0.0, vz);
        self.rt.set_listener(a);
    }

    pub fn add_bus_lowpass(&mut self, bus_id: u32) {
        self.rt
            .add_bus_insert(BusId(bus_id), InsertEffect::lowpass(2000.0));
    }
    pub fn add_bus_highpass(&mut self, bus_id: u32) {
        self.rt
            .add_bus_insert(BusId(bus_id), InsertEffect::highpass(200.0));
    }
    pub fn add_bus_compressor(&mut self, bus_id: u32) {
        self.rt.add_bus_insert(
            BusId(bus_id),
            InsertEffect::compressor(-18.0, 4.0, 3.0),
        );
    }
    pub fn add_bus_reverb(&mut self, bus_id: u32) {
        self.rt
            .add_bus_insert(BusId(bus_id), InsertEffect::reverb(45.0, 0.35, 0.25));
    }
    pub fn add_bus_sidechain(&mut self, bus_id: u32) {
        self.rt.add_bus_insert(
            BusId(bus_id),
            InsertEffect::sidechain_compressor(-24.0, 6.0, 0.0, "SFX"),
        );
    }
    pub fn clear_bus_inserts(&mut self, bus_id: u32) {
        self.rt.clear_bus_inserts(BusId(bus_id));
    }

    pub fn set_bus_insert_info(&mut self, info: &BusInsertInfo) {
        let fx = info.to_effect();
        self.rt
            .set_bus_insert(BusId(info.bus_id), info.index, fx);
    }

    pub fn remove_bus_insert(&mut self, bus_id: u32, index: usize) {
        self.rt.remove_bus_insert(BusId(bus_id), index);
    }

    pub fn capture_fx_chain(&self) -> Vec<crate::project::FxInsertSpec> {
        let mut out = Vec::new();
        for b in self.rt.list_bus_meters() {
            for fx in self.rt.list_bus_inserts(BusId(b.id)) {
                let info = BusInsertInfo::from_effect(b.id, 0, &fx);
                out.push(crate::project::FxInsertSpec {
                    bus: b.name.clone(),
                    kind: info.kind,
                    p0: info.p0,
                    p1: info.p1,
                    p2: info.p2,
                    source_bus: info.source_bus,
                });
            }
        }
        out
    }

    pub fn apply_fx_chain(&mut self, inserts: &[crate::project::FxInsertSpec]) {
        for b in self.rt.list_bus_meters() {
            self.rt.clear_bus_inserts(BusId(b.id));
        }
        for spec in inserts {
            let bus_id = self
                .rt
                .list_bus_meters()
                .iter()
                .find(|b| b.name.eq_ignore_ascii_case(&spec.bus))
                .map(|b| b.id);
            let Some(id) = bus_id else {
                continue;
            };
            let info = BusInsertInfo {
                bus_id: id,
                index: 0,
                kind: spec.kind.clone(),
                p0: spec.p0,
                p1: spec.p1,
                p2: spec.p2,
                source_bus: spec.source_bus.clone(),
            };
            self.rt.add_bus_insert(BusId(id), info.to_effect());
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProfilerSnapshot {
    pub cpu_render_us: u32,
    pub active_voices: usize,
    pub active_instances: usize,
    pub master_peak: f32,
    pub master_rms: f32,
    pub playing_events: Vec<String>,
    pub spectrum_bands: Vec<f32>,
    pub session_frames: usize,
    pub avg_cpu_us: f32,
    pub peak_voices: usize,
    pub bus_meters: Vec<BusMeterInfo>,
    pub cpu_history: Vec<u32>,
    pub voice_history: Vec<u32>,
    pub snapshot_names: Vec<String>,
    pub active_snapshot: Option<(String, f32)>,
    pub emitters: Vec<EmitterDebug>,
    pub listener: (f32, f32, f32),
    pub insert_counts: Vec<(u32, usize)>,
    pub bus_inserts: Vec<BusInsertInfo>,
}

#[derive(Debug, Clone)]
pub struct BusInsertInfo {
    pub bus_id: u32,
    pub index: usize,
    pub kind: String,
    pub p0: f32,
    pub p1: f32,
    pub p2: f32,
    /// Side-chain key bus name (e.g. "SFX"); empty for other FX.
    pub source_bus: String,
}

impl BusInsertInfo {
    pub fn from_effect(bus_id: u32, index: usize, fx: &InsertEffect) -> Self {
        match fx {
            InsertEffect::Gain { gain_db } => Self {
                bus_id,
                index,
                kind: "Gain".into(),
                p0: *gain_db,
                p1: 0.0,
                p2: 0.0,
                source_bus: String::new(),
            },
            InsertEffect::LowPass { cutoff_hz } => Self {
                bus_id,
                index,
                kind: "LowPass".into(),
                p0: *cutoff_hz,
                p1: 0.0,
                p2: 0.0,
                source_bus: String::new(),
            },
            InsertEffect::HighPass { cutoff_hz } => Self {
                bus_id,
                index,
                kind: "HighPass".into(),
                p0: *cutoff_hz,
                p1: 0.0,
                p2: 0.0,
                source_bus: String::new(),
            },
            InsertEffect::Compressor {
                threshold_db,
                ratio,
                makeup_db,
            } => Self {
                bus_id,
                index,
                kind: "Compressor".into(),
                p0: *threshold_db,
                p1: *ratio,
                p2: *makeup_db,
                source_bus: String::new(),
            },
            InsertEffect::Reverb {
                delay_ms,
                feedback,
                wet,
            } => Self {
                bus_id,
                index,
                kind: "Reverb".into(),
                p0: *delay_ms,
                p1: *feedback,
                p2: *wet,
                source_bus: String::new(),
            },
            InsertEffect::SidechainCompressor {
                threshold_db,
                ratio,
                makeup_db,
                source_bus,
            } => Self {
                bus_id,
                index,
                kind: "Sidechain".into(),
                p0: *threshold_db,
                p1: *ratio,
                p2: *makeup_db,
                source_bus: source_bus.clone(),
            },
        }
    }

    pub fn to_effect(&self) -> InsertEffect {
        match self.kind.as_str() {
            "Gain" => InsertEffect::gain(self.p0),
            "HighPass" => InsertEffect::highpass(self.p0.max(20.0)),
            "Compressor" => InsertEffect::compressor(self.p0, self.p1.max(1.0), self.p2),
            "Sidechain" => {
                let key = if self.source_bus.is_empty() {
                    "SFX"
                } else {
                    self.source_bus.as_str()
                };
                InsertEffect::sidechain_compressor(self.p0, self.p1.max(1.0), self.p2, key)
            }
            "Reverb" => InsertEffect::reverb(
                self.p0.max(1.0),
                self.p1.clamp(0.0, 0.95),
                self.p2.clamp(0.0, 1.0),
            ),
            _ => InsertEffect::lowpass(self.p0.max(20.0)),
        }
    }
}

struct ResolvedPcm {
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
}

fn resolve_asset_pcm(project: &Project, ev: &EventEntry) -> ResolvedPcm {
    let asset: Option<&AssetEntry> = ev.asset_index.and_then(|i| project.assets.get(i));
    if let Some(AssetEntry {
        source_path: Some(path),
        ..
    }) = asset
    {
        if let Ok(w) = WaveBuffer::from_file(Path::new(path)) {
            return ResolvedPcm {
                samples: w.samples,
                channels: w.channels,
                sample_rate: w.sample_rate,
            };
        }
    }
    let freq = asset.and_then(|a| a.demo_freq_hz).unwrap_or(440.0);
    let rate = 48_000u32;
    let frames = rate as usize / 2;
    let samples: Vec<f32> = (0..frames)
        .map(|i| {
            let t = i as f32 / rate as f32;
            (2.0 * std::f32::consts::PI * freq * t).sin() * 0.3
        })
        .collect();
    ResolvedPcm {
        samples,
        channels: 1,
        sample_rate: rate,
    }
}

fn resolve_instrument_pcm(
    project: &Project,
    asset_index: Option<usize>,
    fallback_ev: &EventEntry,
) -> ResolvedPcm {
    if let Some(ai) = asset_index {
        let mut pseudo = fallback_ev.clone();
        pseudo.asset_index = Some(ai);
        return resolve_asset_pcm(project, &pseudo);
    }
    resolve_asset_pcm(project, fallback_ev)
}
