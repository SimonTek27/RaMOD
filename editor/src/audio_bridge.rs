//! Transport Play → ramod_runtime (audible via cpal when available).

use std::path::Path;

use ramod_runtime::{
    Attenuation, DesignedEvent, EventDescription, InstrumentDef, PlayParams, Runtime,
    RuntimeConfig, TimelineTrack,
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
        Ok(Self {
            rt,
            registered: Vec::new(),
            playing_id: None,
            preview_voice: None,
        })
    }

    pub fn sync_project(&mut self, project: &Project) {
        self.stop();
        self.registered.clear();
        let open = true;
        if let Ok(rt) = Runtime::new(RuntimeConfig {
            open_device: open,
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
        }

        for ev in &project.events {
            if !ev.tracks.is_empty() {
                let mut buffers = Vec::new();
                let mut designed = DesignedEvent::new(&ev.path);
                designed.priority = ev.priority;
                designed.is_3d = ev.is_3d;
                designed.length = ev.length;
                designed.parameters = ev.parameters.clone();
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
            self.rt.register_event(desc);
            self.registered.push(ev.path.clone());
        }
    }

    pub fn play_event(&mut self, path: &str) -> Result<(), String> {
        self.stop();
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
    }

    pub fn set_parameter(&mut self, name: &str, value: f32) {
        if let Some(id) = self.playing_id {
            self.rt.set_instance_parameter(id, name, value);
        }
    }

    pub fn pump(&mut self, dt: f32) {
        if self.playing_id.is_none() && self.preview_voice.is_none() {
            return;
        }
        self.rt.tick(dt);
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
