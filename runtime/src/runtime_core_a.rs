use std::sync::atomic::{AtomicU64, Ordering};

use buffer::BufferStore;
use event::EventSystem;
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
    markers_fired: Vec<String>,
}

struct FadingOut {
    remaining: f32,
    duration: f32,
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
    recent_markers: Vec<String>,
    pending_transition: Option<(u64, String)>,
    fading_out: Vec<FadingOut>,
    transition_fade_secs: f32,
    profiler: crate::profiler::ProfilerSession,
    spectrum: crate::profiler::SpectrumAnalyzer,
    last_render_us: u32,
    next_voice_id: AtomicU64,
    backend: Option<Box<dyn OutputBackend>>,
    current_output_bus: Option<BusId>,
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
            recent_markers: Vec::new(),
            pending_transition: None,
            fading_out: Vec::new(),
            transition_fade_secs: 0.3,
            profiler: crate::profiler::ProfilerSession::new(600),
            spectrum: crate::profiler::SpectrumAnalyzer::new(16),
            last_render_us: 0,
            next_voice_id: AtomicU64::new(1),
            sample_rate,
            channels,
            backend,
            current_output_bus: None,
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

    /// Master bus peak / RMS (linear). Same values the profiler records.
    pub fn master_levels(&self) -> (f32, f32) {
        let master = self.mixer.master_id();
        self.mixer
            .bus(master)
            .map(|b| (b.meter.peak, b.meter.rms))
            .unwrap_or((0.0, 0.0))
    }

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
            .start(
                id,
                buffer,
                buf,
                params,
                self.sample_rate,
                priority,
                audibility,
                envelope,
            )
            .map_err(|_| RuntimeError::NoFreeVoice)?;
        let bus = self
            .current_output_bus
            .unwrap_or_else(|| self.mixer.default_bus());
        self.mixer.route_voice(id, bus);
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
            if let Some(ev) = self.designed.iter().find(|e| e.path == path).cloned() {
                for curve in &ev.automation {
                    if let Some(v) = curve.eval(elapsed) {
                        let _ = self.set_instance_parameter(iid, &curve.parameter, v);
                    }
                }
                if let Some(pb) = self
                    .designed_playbacks
                    .iter_mut()
                    .find(|p| p.instance_id == iid)
                {
                    for m in &ev.markers {
                        if elapsed >= m.time && !pb.markers_fired.iter().any(|n| n == &m.name) {
                            pb.markers_fired.push(m.name.clone());
                            self.recent_markers.push(m.name.clone());
                            if self.recent_markers.len() > 32 {
                                self.recent_markers.remove(0);
                            }
                            if let Some(ref target) = m.target_event {
                                if !target.is_empty() {
                                    self.pending_transition = Some((iid, target.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some((stop_id, next_path)) = self.pending_transition.take() {
            let fade = self.transition_fade_secs.max(0.01);
            let mut fade_voices = Vec::new();
            if let Some(pos) = self
                .designed_playbacks
                .iter()
                .position(|p| p.instance_id == stop_id)
            {
                let pb = self.designed_playbacks.remove(pos);
                fade_voices.extend(pb.voices);
            }
            if let Some(inst) = self.events.instance_mut(stop_id) {
                if let Some(v) = inst.voice.take() {
                    fade_voices.push(v);
                }
                inst.started = false;
            }
            self.scheduled.retain(|s| s.instance_id != Some(stop_id));
            self.events.release_instance(stop_id);
            if !fade_voices.is_empty() {
                self.fading_out.push(FadingOut {
                    remaining: fade,
                    duration: fade,
                    voices: fade_voices,
                });
            }
            let _ = self.play_event(&next_path);
        }

        let mut still_fade = Vec::new();
        for mut f in self.fading_out.drain(..) {
            f.remaining -= dt;
            if f.remaining <= 0.0 {
                for v in f.voices {
                    self.voices.stop(v);
                }
            } else {
                let g = (f.remaining / f.duration).clamp(0.0, 1.0);
                for &v in &f.voices {
                    self.voices.set_gain(v, g);
                }
                still_fade.push(f);
            }
        }
        self.fading_out = still_fade;

        self.voices.advance_envelopes(dt);
        self.voices.reap_finished();
        self.sync_event_spatial_gains();
        self.snapshots.apply(&mut self.mixer);
    }
}
