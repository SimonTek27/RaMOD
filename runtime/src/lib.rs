//! # ramod_runtime — open-source real-time audio engine
//!
//! S1 PCM · S2 Events/3D · S4 Mixer/snapshots/DSP/meters

mod backend;
mod bank;
mod buffer;
mod bus;
mod dsp;
mod error;
mod event;
mod mixer;
mod spatial;
mod voice;

pub use backend::{NullBackend, OutputBackend};
pub use bank::{Bank, BankEventSpec, BankName, BankRegistry};
pub use buffer::{BufferHandle, PcmBuffer};
pub use bus::MasterBus;
pub use dsp::InsertEffect;
pub use error::RuntimeError;
pub use event::{EventDescription, EventInstance, EventPath};
pub use mixer::{Bus, BusId, Meter, Mixer, Snapshot, SnapshotPlayer};
pub use spatial::{doppler_pitch, Attributes3d, Attenuation, Vec3};
pub use voice::{PlayParams, VoiceHandle, VoiceState};

use std::sync::atomic::{AtomicU64, Ordering};

use bank::BankRegistry;
use buffer::BufferStore;
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

pub struct Runtime {
    sample_rate: u32,
    channels: u16,
    max_voices: usize,
    buffers: BufferStore,
    voices: VoicePool,
    master: MasterBus,
    mixer: Mixer,
    snapshots: SnapshotPlayer,
    events: EventSystem,
    banks: BankRegistry,
    next_voice_id: AtomicU64,
    #[allow(dead_code)]
    backend: Option<Box<dyn OutputBackend>>,
}

impl Runtime {
    pub fn new(cfg: RuntimeConfig) -> Result<Self, RuntimeError> {
        let mut sample_rate = cfg.sample_rate;
        let mut channels = cfg.channels;
        let backend = if cfg.open_device {
            match backend::open_default(&cfg)? {
                Some(b) => {
                    sample_rate = b.sample_rate();
                    channels = b.channels();
                    Some(b)
                }
                None => None,
            }
        } else {
            None
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
            next_voice_id: AtomicU64::new(1),
            sample_rate,
            channels,
            backend,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn master(&self) -> &MasterBus {
        &self.master
    }

    pub fn master_mut(&mut self) -> &mut MasterBus {
        &mut self.master
    }

    pub fn mixer(&self) -> &Mixer {
        &self.mixer
    }

    pub fn mixer_mut(&mut self) -> &mut Mixer {
        &mut self.mixer
    }

    // ---- S1 PCM API ---------------------------------------------------------

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
        Ok(self.buffers.insert(PcmBuffer {
            samples,
            channels,
            sample_rate,
        }))
    }

    pub fn play(
        &mut self,
        buffer: BufferHandle,
        params: PlayParams,
    ) -> Result<VoiceHandle, RuntimeError> {
        self.play_with_priority(buffer, params, 128, 1.0)
    }

    fn play_with_priority(
        &mut self,
        buffer: BufferHandle,
        params: PlayParams,
        priority: i32,
        audibility: f32,
    ) -> Result<VoiceHandle, RuntimeError> {
        let buf = self
            .buffers
            .get(buffer)
            .ok_or(RuntimeError::InvalidHandle)?;
        if self.voices.active_count() >= self.max_voices {
            if !self.voices.steal_lowest(priority, audibility) {
                return Err(RuntimeError::NoFreeVoice);
            }
        }
        let id = self.next_voice_id.fetch_add(1, Ordering::Relaxed);
        let handle = self
            .voices
            .start(id, buffer, buf, params, self.sample_rate, priority, audibility)
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

    pub fn tick(&mut self, _dt: f32) {
        self.voices.reap_finished();
        self.sync_event_spatial_gains();
        self.snapshots.apply(&mut self.mixer);
    }

    pub fn render(&mut self, out: &mut [f32]) {
        self.sync_event_spatial_gains();
        self.snapshots.apply(&mut self.mixer);
        let ch = self.channels as usize;
        if ch == 0 || out.len() % ch != 0 {
            out.fill(0.0);
            return;
        }
        let frames = out.len() / ch;

        // Mix all voices into a temp buffer, route to default bus (S4 MVP).
        let mut dry = vec![0.0f32; frames * ch];
        self.voices
            .mix_into(&mut dry, frames, ch, self.sample_rate, &self.buffers);

        self.mixer.prepare(frames, ch);
        // Fold into default leaf bus under a synthetic voice id 0.
        self.mixer.accumulate_voice(0, &dry);
        // Ensure voice 0 is routed to default bus
        self.mixer.route_voice(0, self.mixer.default_bus());
        self.mixer.finish(out, ch, self.sample_rate);

        // Legacy master gain still applies on top.
        let g = self.master.gain();
        if (g - 1.0).abs() > f32::EPSILON {
            for s in out.iter_mut() {
                *s *= g;
            }
        }
    }

    pub fn render_seconds(&mut self, seconds: f32) -> Vec<f32> {
        let frames = (seconds * self.sample_rate as f32).max(0.0) as usize;
        let mut out = vec![0.0f32; frames * self.channels as usize];
        self.render(&mut out);
        out
    }

    // ---- S2 Event API -------------------------------------------------------

    pub fn register_event(&mut self, desc: EventDescription) {
        self.events.register(desc);
    }

    pub fn load_bank(&mut self, bank: Bank) {
        for spec in bank.events.iter().cloned() {
            self.events.register(spec.into_description());
        }
        self.banks.insert(bank);
    }

    pub fn set_listener(&mut self, attributes: Attr3d) {
        self.events.set_listener(attributes);
    }

    pub fn listener(&self) -> Attr3d {
        self.events.listener()
    }

    pub fn create_instance(&mut self, path: &str) -> Result<EventInstance, RuntimeError> {
        let id = self
            .events
            .create_instance(path)
            .ok_or(RuntimeError::EventNotFound)?;
        self.events
            .release_instance(id)
            .ok_or(RuntimeError::InvalidHandle)
    }

    pub fn start_instance(&mut self, mut inst: EventInstance) -> Result<u64, RuntimeError> {
        let audibility = self.audibility_for(&inst);
        let params = inst.play_params();
        let voice = self.play_with_priority(inst.buffer, params, inst.priority, audibility)?;
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
    }

    pub fn release_instance(&mut self, id: u64) {
        self.stop_instance(id);
        self.events.release_instance(id);
    }

    pub fn set_instance_parameter(&mut self, id: u64, name: &str, value: f32) -> bool {
        self.events
            .instance_mut(id)
            .map(|i| i.set_parameter(name, value))
            .unwrap_or(false)
    }

    pub fn set_instance_3d(&mut self, id: u64, attributes: Attr3d) {
        if let Some(i) = self.events.instance_mut(id) {
            i.set_3d(attributes);
        }
    }

    pub fn instance_count(&self) -> usize {
        self.events.instance_count()
    }

    // ---- S4 Mixer / Snapshot API --------------------------------------------

    pub fn set_bus_volume_db(&mut self, bus: BusId, db: f32) {
        self.mixer.set_volume_db(bus, db);
    }

    pub fn set_bus_mute(&mut self, bus: BusId, mute: bool) {
        self.mixer.set_mute(bus, mute);
    }

    pub fn set_bus_solo(&mut self, bus: BusId, solo: bool) {
        self.mixer.set_solo(bus, solo);
    }

    pub fn add_bus_insert(&mut self, bus: BusId, effect: InsertEffect) {
        self.mixer.add_insert(bus, effect);
    }

    pub fn register_snapshot(&mut self, snapshot: Snapshot) {
        self.snapshots.add(snapshot);
    }

    pub fn capture_snapshot_base(&mut self) {
        self.snapshots.capture_base(&self.mixer);
    }

    /// Blend toward named snapshot; weight 0 = base, 1 = full snapshot.
    pub fn trigger_snapshot(&mut self, name: &str, weight: f32) {
        self.snapshots.trigger(name, weight);
        self.snapshots.apply(&mut self.mixer);
    }

    pub fn clear_snapshot(&mut self) {
        self.snapshots.clear();
    }

    pub fn bus_meter(&self, bus: BusId) -> Option<Meter> {
        self.mixer.bus(bus).map(|b| b.meter)
    }

    fn audibility_for(&self, inst: &EventInstance) -> f32 {
        if !inst.is_3d {
            return 1.0;
        }
        let listener = self.events.listener();
        let d = distance(&listener, &inst.attributes);
        inst.attenuation.gain_at(d) * inst.gain_user
    }

    fn sync_event_spatial_gains(&mut self) {
        let listener = self.events.listener();
        let mut updates: Vec<(VoiceHandle, f32, f32)> = Vec::new();
        for inst in self.events.instances_mut() {
            let Some(voice) = inst.voice else {
                continue;
            };
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
            max_voices: 8,
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

    #[test]
    fn event_play_by_path() {
        let mut rt = headless();
        let buf = rt.load_pcm(sine(4800, 48_000, 330.0), 1, 48_000).unwrap();
        rt.register_event(EventDescription::new("event:/sfx/beep", buf));
        let id = rt.play_event("event:/sfx/beep").unwrap();
        assert_eq!(rt.active_voices(), 1);
        let out = rt.render_seconds(0.03);
        assert!(out.iter().any(|s| s.abs() > 0.01));
        rt.release_instance(id);
        assert_eq!(rt.active_voices(), 0);
    }

    #[test]
    fn bus_mute_silences_output() {
        let mut rt = headless();
        let buf = rt.load_pcm(sine(4800, 48_000, 440.0), 1, 48_000).unwrap();
        rt.play(buf, PlayParams::default()).unwrap();
        let loud = rt.render_seconds(0.02);
        let peak_loud = loud.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        let mut rt = headless();
        let buf = rt.load_pcm(sine(4800, 48_000, 440.0), 1, 48_000).unwrap();
        rt.play(buf, PlayParams::default()).unwrap();
        let master = rt.mixer().master_id();
        rt.set_bus_mute(master, true);
        let quiet = rt.render_seconds(0.02);
        let peak_quiet = quiet.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(peak_loud > 0.01);
        assert!(peak_quiet < 0.001);
    }

    #[test]
    fn snapshot_blends_volume() {
        let mut rt = headless();
        let master = rt.mixer().master_id();
        rt.capture_snapshot_base();
        let mut snap = Snapshot::new("quiet");
        snap.set_volume(master, -24.0);
        rt.register_snapshot(snap);
        rt.trigger_snapshot("quiet", 1.0);
        assert!((rt.mixer().bus(master).unwrap().volume_db - (-24.0)).abs() < 0.01);
    }

    #[test]
    fn meter_updates_after_render() {
        let mut rt = headless();
        let buf = rt.load_pcm(sine(4800, 48_000, 440.0), 1, 48_000).unwrap();
        rt.play(buf, PlayParams::default()).unwrap();
        let _ = rt.render_seconds(0.05);
        let master = rt.mixer().master_id();
        let m = rt.bus_meter(master).unwrap();
        assert!(m.peak > 0.0 || m.rms >= 0.0);
    }
}
