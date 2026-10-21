//! Transport Play → ramod_runtime (audible via cpal when available).

use std::path::Path;

use ramod_runtime::{
    Attenuation, AutomationCurve, BusId, BusMeterInfo, DesignedEvent, EventDescription,
    InstrumentDef, PlayParams, Runtime, RuntimeConfig, TimelineTrack, TransitionMarker,
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
