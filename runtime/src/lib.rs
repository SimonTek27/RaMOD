//! # ramod_runtime — open-source real-time audio engine
//!
//! S1–S9 stack with audible cpal output via ring buffer.

mod automation;
mod backend;
mod bank;
mod buffer;
mod bus;
mod designer;
mod dsp;
mod envelope;
mod error;
mod event;
mod export;
mod live_api;
#[cfg(feature = "live-update")]
pub mod live_update;
mod mixer;
mod profiler;
mod spatial;
mod voice;

pub use automation::{AutomationCurve, Keyframe};
pub use backend::{NullBackend, OutputBackend};
pub use bank::{Bank, BankEventSpec, BankName, BankRegistry};
pub use buffer::{BufferHandle, PcmBuffer};
pub use bus::MasterBus;
pub use designer::{
    DesignedEvent, InstrumentDef, InstrumentKind, LoopRegion, TimelineTrack, TransitionMarker,
};
pub use dsp::InsertEffect;
pub use envelope::Ahdsr;
pub use error::RuntimeError;
pub use event::{EventDescription, EventInstance, EventPath};
pub use export::{BankManifest, EventManifestEntry};
pub use mixer::{Bus, BusId, Meter, Mixer, Snapshot, SnapshotPlayer};
pub use profiler::{EmitterDebug, ProfilerFrame, ProfilerSession, SpectrumAnalyzer};
pub use spatial::{doppler_pitch, Attributes3d, Attenuation, Vec3};
pub use voice::{PlayParams, VoiceHandle, VoiceState};

#[cfg(feature = "live-update")]
pub use live_update::{LiveUpdateClient, LiveUpdateServer};

use std::sync::atomic::{AtomicU64, Ordering};

use bank::BankRegistry;
use buffer::BufferStore;
use designer::DesignedEvent;
use event::EventSystem;
use mixer::{Mixer, Snapshot, SnapshotPlayer};
use spatial::{closing_speed, distance, Attributes3d as Attr3d};
use voice::VoicePool;

pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;
pub const DEFAULT_CHANNELS: u16 = 2;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub max_voices: usize,
    pub open_device: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            channels: DEFAULT_CHANNELS,
            max_voices: 64,
            open_device: true,
        }
    }
}

struct ScheduledVoice {
    remaining: f32,
    buffer: BufferHandle,
    params: PlayParams,
    priority: i32,
    envelope: Option<crate::envelope::Ahdsr>,
    instance_id: Option<u64>,
}

struct DesignedPlayback {
    instance_id: u64,
    path: String,
    elapsed: f32,
    voices: Vec<VoiceHandle>,
}

pub struct Runtime {
    sample_rate: u32,
    channels: u16,
    max_voices: usize,
    buffers: BufferStore,
    voices: VoicePool,
    master: MasterBus,
    mixer: Mixer,
    snapshots: SnapshotPlayer,
    pub(crate) events: EventSystem,
    banks: BankRegistry,
    designed: Vec<DesignedEvent>,
    designed_buffers: std::collections::HashMap<String, Vec<BufferHandle>>,
    scheduled: Vec<ScheduledVoice>,
    designed_playbacks: Vec<DesignedPlayback>,
    profiler: crate::profiler::ProfilerSession,
    spectrum: crate::profiler::SpectrumAnalyzer,
    last_render_us: u32,
    next_voice_id: AtomicU64,
    backend: Option<Box<dyn OutputBackend>>,
}

impl Runtime {
    pub fn new(cfg: RuntimeConfig) -> Result<Self, RuntimeError> {
        let mut sample_rate = cfg.sample_rate;
        let mut channels = cfg.channels;
        let backend = match backend::open_default(&cfg)? {
            Some(b) => {
                sample_rate = b.sample_rate();
                channels = b.channels();
                Some(b)
            }
            None => None,
        };

        Ok(Self {
            max_voices: cfg.max_voices,
            voices: VoicePool::new(cfg.max_voices),
            buffers: BufferStore::new(),
            master: MasterBus::new(),
            mixer: Mixer::new_with_defaults(),
            snapshots: SnapshotPlayer::default(),
            events: EventSystem::new(),
            banks: BankRegistry::new(),
            designed: Vec::new(),
            designed_buffers: std::collections::HashMap::new(),
            scheduled: Vec::new(),
            designed_playbacks: Vec::new(),
            profiler: crate::profiler::ProfilerSession::new(600),
            spectrum: crate::profiler::SpectrumAnalyzer::new(16),
            last_render_us: 0,
            next_voice_id: AtomicU64::new(1),
            sample_rate,
            channels,
            backend,
        })
    }

    pub fn sample_rate(&self) -> u32 { self.sample_rate }
    pub fn channels(&self) -> u16 { self.channels }
    pub fn master(&self) -> &MasterBus { &self.master }
    pub fn master_mut(&mut self) -> &mut MasterBus { &mut self.master }
    pub fn mixer(&self) -> &Mixer { &self.mixer }
    pub fn mixer_mut(&mut self) -> &mut Mixer { &mut self.mixer }

    pub fn load_pcm(
        &mut self,
        samples: Vec<f32>,
        channels: u16,
        sample_rate: u32,
    ) -> Result<BufferHandle, RuntimeError> {
        if channels == 0 || sample_rate == 0 {
            return Err(RuntimeError::InvalidBuffer);
        }
        if samples.len() % channels as usize != 0 {
            return Err(RuntimeError::InvalidBuffer);
        }
        Ok(self.buffers.insert(PcmBuffer { samples, channels, sample_rate }))
    }

    pub fn play(
        &mut self,
        buffer: BufferHandle,
        params: PlayParams,
    ) -> Result<VoiceHandle, RuntimeError> {
        self.play_with_priority(buffer, params, 128, 1.0, None)
    }

    fn play_with_priority(
        &mut self,
        buffer: BufferHandle,
        params: PlayParams,
        priority: i32,
        audibility: f32,
        envelope: Option<crate::envelope::Ahdsr>,
    ) -> Result<VoiceHandle, RuntimeError> {
        let buf = self.buffers.get(buffer).ok_or(RuntimeError::InvalidHandle)?;
        if self.voices.active_count() >= self.max_voices {
            if !self.voices.steal_lowest(priority, audibility) {
                return Err(RuntimeError::NoFreeVoice);
            }
        }
        let id = self.next_voice_id.fetch_add(1, Ordering::Relaxed);
        let handle = self
            .voices
            .start(id, buffer, buf, params, self.sample_rate, priority, audibility, envelope)
            .map_err(|_| RuntimeError::NoFreeVoice)?;
        self.mixer.route_voice(id, self.mixer.default_bus());
        Ok(handle)
    }

    pub fn stop(&mut self, voice: VoiceHandle) {
        self.voices.stop(voice);
    }

    pub fn set_voice_gain(&mut self, voice: VoiceHandle, gain: f32) {
        self.voices.set_gain(voice, gain.max(0.0));
    }

    pub fn active_voices(&self) -> usize {
        self.voices.active_count()
    }

    pub fn tick(&mut self, dt: f32) {
        let mut still = Vec::new();
        let due: Vec<_> = self
            .scheduled
            .drain(..)
            .map(|mut s| {
                s.remaining -= dt;
                s
            })
            .collect();
        for s in due {
            if s.remaining <= 0.0 {
                if let Ok(h) =
                    self.play_with_priority(s.buffer, s.params, s.priority, 1.0, s.envelope)
                {
                    if let Some(iid) = s.instance_id {
                        if let Some(pb) = self
                            .designed_playbacks
                            .iter_mut()
                            .find(|p| p.instance_id == iid)
                        {
                            pb.voices.push(h);
                        }
                    }
                }
            } else {
                still.push(s);
            }
        }
        self.scheduled = still;

        let paths: Vec<(u64, String, f32)> = self
            .designed_playbacks
            .iter_mut()
            .map(|p| {
                p.elapsed += dt;
                (p.instance_id, p.path.clone(), p.elapsed)
            })
            .collect();
        for (iid, path, elapsed) in paths {
            if let Some(ev) = self.designed.iter().find(|e| e.path == path) {
                for curve in &ev.automation {
                    if let Some(v) = curve.eval(elapsed) {
                        let _ = self.set_instance_parameter(iid, &curve.parameter, v);
                    }
                }
            }
        }

        self.voices.advance_envelopes(dt);
        self.voices.reap_finished();
        self.sync_event_spatial_gains();
        self.snapshots.apply(&mut self.mixer);
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let t0 = std::time::Instant::now();
        self.sync_event_spatial_gains();
        self.snapshots.apply(&mut self.mixer);
        let ch = self.channels as usize;
        if ch == 0 || out.len() % ch != 0 {
            out.fill(0.0);
            return;
        }
        let frames = out.len() / ch;
        let mut dry = vec![0.0f32; frames * ch];
        self.voices
            .mix_into(&mut dry, frames, ch, self.sample_rate, &self.buffers);
        self.mixer.prepare(frames, ch);
        self.mixer.route_voice(0, self.mixer.default_bus());
        self.mixer.accumulate_voice(0, &dry);
        self.mixer.finish(out, ch, self.sample_rate);
        let g = self.master.gain();
        if (g - 1.0).abs() > f32::EPSILON {
            for s in out.iter_mut() {
                *s *= g;
            }
        }
        self.last_render_us = t0.elapsed().as_micros().min(u128::from(u32::MAX)) as u32;
        self.spectrum.analyze(out, ch);
        self.capture_profiler_frame();
        if let Some(ref backend) = self.backend {
            backend.push_samples(out);
        }
    }

    pub fn profiler_enable(&mut self, on: bool) { self.profiler.enable(on); }
    pub fn profiler_clear(&mut self) { self.profiler.clear(); }
    pub fn profiler_session(&self) -> &crate::profiler::ProfilerSession { &self.profiler }
    pub fn profiler_latest(&self) -> Option<&crate::profiler::ProfilerFrame> { self.profiler.latest() }
    pub fn spectrum(&self) -> &crate::profiler::SpectrumAnalyzer { &self.spectrum }
    pub fn last_render_cpu_us(&self) -> u32 { self.last_render_us }

    fn capture_profiler_frame(&mut self) {
        if !self.profiler.is_enabled() { return; }
        let master = self.mixer.master_id();
        let (peak, rms) = self.mixer.bus(master).map(|b| (b.meter.peak, b.meter.rms)).unwrap_or((0.0, 0.0));
        let mut playing_events = Vec::new();
        let mut params = Vec::new();
        let mut emitters = Vec::new();
        for id in 1..1024u64 {
            let Some(inst) = self.events.instance(id) else { continue };
            if !inst.is_playing() { continue; }
            let path = inst.path().as_str().to_string();
            playing_events.push(path.clone());
            if let Some(v) = inst.get_parameter("Intensity") {
                params.push((path.clone(), "Intensity".into(), v));
            }
            if inst.is_3d {
                let a = inst.attributes();
                emitters.push(crate::profiler::EmitterDebug {
                    position: a.position,
                    velocity: a.velocity,
                    audibility: 1.0,
                });
            }
        }
        if playing_events.len() > 16 { playing_events.truncate(16); }
        self.profiler.push(crate::profiler::ProfilerFrame {
            index: 0,
            time: 0.0,
            cpu_render_us: self.last_render_us,
            active_voices: self.voices.active_count(),
            active_instances: self.events.instance_count(),
            master_peak: peak,
            master_rms: rms,
            playing_events,
            params,
            emitters,
        });
    }

    pub fn render_seconds(&mut self, seconds: f32) -> Vec<f32> {
        let frames = (seconds * self.sample_rate as f32).max(0.0) as usize;
        let mut out = vec![0.0f32; frames * self.channels as usize];
        self.render(&mut out);
        out
    }

    pub fn register_event(&mut self, desc: EventDescription) {
        self.events.register(desc);
    }

    pub fn register_designed_event(
        &mut self,
        event: DesignedEvent,
        buffers: &[BufferHandle],
    ) -> Result<(), RuntimeError> {
        let primary = event
            .tracks
            .iter()
            .flat_map(|t| t.instruments.iter())
            .find(|i| !i.buffer_slots.is_empty())
            .and_then(|i| i.buffer_slots.first().copied())
            .and_then(|slot| buffers.get(slot).copied())
            .ok_or(RuntimeError::InvalidBuffer)?;
        let mut desc = EventDescription::new(&event.path, primary)
            .with_priority(event.priority)
            .with_loop(
                event.tracks.iter().flat_map(|t| t.instruments.iter()).any(|i| i.looping),
            );
        if event.is_3d {
            desc = desc.with_3d(event.attenuation);
        }
        for (n, v) in &event.parameters {
            desc = desc.with_parameter(n.clone(), *v);
        }
        self.events.register(desc);
        self.designed_buffers.insert(event.path.clone(), buffers.to_vec());
        self.designed.push(event);
        Ok(())
    }

    pub fn play_designed(
        &mut self,
        path: &str,
        buffers: &[BufferHandle],
    ) -> Result<Vec<VoiceHandle>, RuntimeError> {
        let event = self
            .designed
            .iter()
            .find(|e| e.path == path)
            .cloned()
            .ok_or(RuntimeError::EventNotFound)?;
        let any_solo = event.tracks.iter().any(|t| t.solo);
        let mut handles = Vec::new();
        for track in &event.tracks {
            if track.mute { continue; }
            if any_solo && !track.solo { continue; }
            for inst in &track.instruments {
                let roll = simple_hash(&format!("{}:{}", path, inst.name)) as f32 / u64::MAX as f32;
                if roll > inst.probability.clamp(0.0, 1.0) { continue; }
                let count = match inst.kind {
                    designer::InstrumentKind::Scatter => inst.scatter_count.max(1),
                    _ => 1,
                };
                for n in 0..count {
                    if inst.buffer_slots.is_empty() { continue; }
                    let slot = if inst.kind == designer::InstrumentKind::Multi {
                        let idx = (simple_hash(&(inst.name.clone() + &n.to_string())) as usize)
                            % inst.buffer_slots.len();
                        inst.buffer_slots[idx]
                    } else {
                        inst.buffer_slots[0]
                    };
                    let Some(&buf) = buffers.get(slot) else { continue };
                    let params = PlayParams { gain: 1.0, looped: inst.looping };
                    if inst.start_time <= 0.001 {
                        match self.play_with_priority(buf, params, event.priority, 1.0, Some(inst.envelope)) {
                            Ok(h) => handles.push(h),
                            Err(_) => break,
                        }
                    } else {
                        self.scheduled.push(ScheduledVoice {
                            remaining: inst.start_time,
                            buffer: buf,
                            params,
                            priority: event.priority,
                            envelope: Some(inst.envelope),
                            instance_id: None,
                        });
                    }
                }
            }
        }
        Ok(handles)
    }

    pub fn play_designed_registered(
        &mut self,
        path: &str,
    ) -> Result<Vec<VoiceHandle>, RuntimeError> {
        let buffers = self
            .designed_buffers
            .get(path)
            .cloned()
            .ok_or(RuntimeError::EventNotFound)?;
        self.play_designed(path, &buffers)
    }

    pub fn designed_events(&self) -> &[DesignedEvent] { &self.designed }

    pub fn export_bank_manifest(&self, name: &str) -> BankManifest {
        BankManifest::from_events(name, &self.designed)
    }

    pub fn load_bank(&mut self, bank: Bank) {
        for spec in bank.events.iter().cloned() {
            self.events.register(spec.into_description());
        }
        self.banks.insert(bank);
    }

    pub fn set_listener(&mut self, attributes: Attr3d) { self.events.set_listener(attributes); }
    pub fn listener(&self) -> Attr3d { self.events.listener() }

    pub fn create_instance(&mut self, path: &str) -> Result<EventInstance, RuntimeError> {
        let id = self.events.create_instance(path).ok_or(RuntimeError::EventNotFound)?;
        self.events.release_instance(id).ok_or(RuntimeError::InvalidHandle)
    }

    pub fn start_instance(&mut self, mut inst: EventInstance) -> Result<u64, RuntimeError> {
        let audibility = self.audibility_for(&inst);
        let params = inst.play_params();
        let voice = self.play_with_priority(inst.buffer, params, inst.priority, audibility, None)?;
        inst.voice = Some(voice);
        inst.started = true;
        let id = inst.id;
        if self.events.instance(id).is_none() {
            self.events.instances_reinsert(inst);
        } else if let Some(slot) = self.events.instance_mut(id) {
            *slot = inst;
        }
        Ok(id)
    }

    pub fn play_event(&mut self, path: &str) -> Result<u64, RuntimeError> {
        if self.designed.iter().any(|e| e.path == path) {
            let mut handles = self.play_designed_registered(path)?;
            let mut inst = self.create_instance(path)?;
            inst.started = true;
            let id = inst.id;
            for s in self.scheduled.iter_mut() {
                if s.instance_id.is_none() {
                    s.instance_id = Some(id);
                }
            }
            self.designed_playbacks.push(DesignedPlayback {
                instance_id: id,
                path: path.to_string(),
                elapsed: 0.0,
                voices: std::mem::take(&mut handles),
            });
            self.events.instances_reinsert(inst);
            return Ok(id);
        }
        let inst = self.create_instance(path)?;
        self.start_instance(inst)
    }

    pub fn stop_instance(&mut self, id: u64) {
        if let Some(inst) = self.events.instance_mut(id) {
            if let Some(v) = inst.voice.take() {
                self.voices.stop(v);
            }
            inst.started = false;
        }
        if let Some(pos) = self.designed_playbacks.iter().position(|p| p.instance_id == id) {
            let pb = self.designed_playbacks.remove(pos);
            for v in pb.voices {
                self.voices.stop(v);
            }
        }
        self.scheduled.retain(|s| s.instance_id != Some(id));
    }

    pub fn clear_scheduled(&mut self) {
        self.scheduled.clear();
    }

    pub fn release_instance(&mut self, id: u64) {
        self.stop_instance(id);
        self.events.release_instance(id);
    }

    pub fn set_instance_parameter(&mut self, id: u64, name: &str, value: f32) -> bool {
        self.events.instance_mut(id).map(|i| i.set_parameter(name, value)).unwrap_or(false)
    }

    pub fn set_instance_3d(&mut self, id: u64, attributes: Attr3d) {
        if let Some(i) = self.events.instance_mut(id) {
            i.set_3d(attributes);
        }
    }

    pub fn instance_count(&self) -> usize { self.events.instance_count() }

    pub fn set_bus_volume_db(&mut self, bus: BusId, db: f32) { self.mixer.set_volume_db(bus, db); }
    pub fn set_bus_mute(&mut self, bus: BusId, mute: bool) { self.mixer.set_mute(bus, mute); }
    pub fn set_bus_solo(&mut self, bus: BusId, solo: bool) { self.mixer.set_solo(bus, solo); }
    pub fn add_bus_insert(&mut self, bus: BusId, effect: InsertEffect) { self.mixer.add_insert(bus, effect); }

    pub fn register_snapshot(&mut self, snapshot: Snapshot) { self.snapshots.add(snapshot); }
    pub fn capture_snapshot_base(&mut self) { self.snapshots.capture_base(&self.mixer); }
    pub fn trigger_snapshot(&mut self, name: &str, weight: f32) {
        self.snapshots.trigger(name, weight);
        self.snapshots.apply(&mut self.mixer);
    }
    pub fn clear_snapshot(&mut self) { self.snapshots.clear(); }
    pub fn bus_meter(&self, bus: BusId) -> Option<Meter> { self.mixer.bus(bus).map(|b| b.meter) }

    fn audibility_for(&self, inst: &EventInstance) -> f32 {
        if !inst.is_3d { return 1.0; }
        let listener = self.events.listener();
        let d = distance(&listener, &inst.attributes);
        inst.attenuation.gain_at(d) * inst.gain_user
    }

    fn sync_event_spatial_gains(&mut self) {
        let listener = self.events.listener();
        let mut updates: Vec<(VoiceHandle, f32, f32)> = Vec::new();
        for inst in self.events.instances_mut() {
            let Some(voice) = inst.voice else { continue };
            let mut gain = inst.gain_user;
            let mut pitch = 1.0;
            if inst.is_3d {
                let d = distance(&listener, &inst.attributes);
                gain *= inst.attenuation.gain_at(d);
                let closing = closing_speed(&listener, &inst.attributes);
                pitch = spatial::doppler_pitch(closing);
            }
            if let Some(intensity) = inst.get_parameter("Intensity") {
                gain *= intensity.clamp(0.0, 4.0);
            }
            updates.push((voice, gain, pitch));
        }
        for (voice, gain, pitch) in updates {
            self.voices.set_gain(voice, gain);
            self.voices.set_pitch(voice, pitch);
        }
    }
}

fn simple_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frames: usize, rate: u32, freq: f32) -> Vec<f32> {
        (0..frames)
            .map(|i| {
                let t = i as f32 / rate as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin() * 0.25
            })
            .collect()
    }

    fn headless() -> Runtime {
        Runtime::new(RuntimeConfig {
            open_device: false,
            sample_rate: 48_000,
            channels: 1,
            max_voices: 16,
        })
        .unwrap()
    }

    #[test]
    fn plays_and_mixes_pcm() {
        let mut rt = headless();
        let buf = rt.load_pcm(sine(4800, 48_000, 440.0), 1, 48_000).unwrap();
        let v = rt.play(buf, PlayParams::default()).unwrap();
        assert_eq!(rt.active_voices(), 1);
        let out = rt.render_seconds(0.05);
        assert!(out.iter().any(|s| s.abs() > 0.01));
        rt.stop(v);
        rt.tick(0.0);
        assert_eq!(rt.active_voices(), 0);
    }
}
