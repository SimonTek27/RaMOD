//! Transport Play → ramod_runtime (audible via cpal when available).

use std::path::Path;

use ramod_runtime::{
    Attenuation, EventDescription, PlayParams, Runtime, RuntimeConfig,
};
use ramod_waveform::WaveBuffer;

use crate::project::{AssetEntry, EventEntry, Project};

pub struct AudioBridge {
    rt: Runtime,
    registered: Vec<String>,
    playing_id: Option<u64>,
    /// Voice-style play from waveform (buffer handle path).
    preview_voice: Option<ramod_runtime::VoiceHandle>,
}

impl AudioBridge {
    pub fn new() -> Result<Self, String> {
        // Prefer real device; fall back to headless.
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

    /// Preview arbitrary PCM (waveform panel).
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
        // With open_device, cpal callback may also pull; render keeps state advancing.
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

    // Prefer file on disk (wav/mp3/ogg/flac).
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

    // Demo tone fallback
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
