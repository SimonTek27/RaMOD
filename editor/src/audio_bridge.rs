//! S3.3 — Transport Play → ramod_runtime.

use ramod_runtime::{
    Attributes3d, Attenuation, EventDescription, PlayParams, Runtime, RuntimeConfig, Vec3,
};

use crate::project::{AssetEntry, EventEntry, Project};

pub struct AudioBridge {
    rt: Runtime,
    /// path → registered
    registered: Vec<String>,
    playing_id: Option<u64>,
}

impl AudioBridge {
    pub fn new() -> Result<Self, String> {
        let rt = Runtime::new(RuntimeConfig {
            open_device: false, // headless mix; wire cpal later
            sample_rate: 48_000,
            channels: 2,
            max_voices: 32,
        })
        .map_err(|e| e.to_string())?;
        Ok(Self {
            rt,
            registered: Vec::new(),
            playing_id: None,
        })
    }

    /// Rebuild event registry from the project (call after project edits).
    pub fn sync_project(&mut self, project: &Project) {
        self.stop();
        self.registered.clear();
        // Re-create runtime to clear buffers/events cleanly (S3 simplicity).
        if let Ok(rt) = Runtime::new(RuntimeConfig {
            open_device: false,
            sample_rate: 48_000,
            channels: 2,
            max_voices: 32,
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

    pub fn stop(&mut self) {
        if let Some(id) = self.playing_id.take() {
            self.rt.release_instance(id);
        }
    }

    pub fn set_parameter(&mut self, name: &str, value: f32) {
        if let Some(id) = self.playing_id {
            self.rt.set_instance_parameter(id, name, value);
        }
    }

    /// Drain a short buffer (keeps voices advancing while UI plays).
    pub fn pump(&mut self, dt: f32) {
        if self.playing_id.is_none() {
            return;
        }
        self.rt.tick(dt);
        let frames = ((dt * self.rt.sample_rate() as f32).max(1.0) as usize).min(4096);
        let mut out = vec![0.0f32; frames * self.rt.channels() as usize];
        self.rt.render(&mut out);
        // S3: mixed to null (open_device: false). Peak used for UI meter later.
        let _peak = out.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    }

    pub fn is_playing(&self) -> bool {
        self.playing_id.is_some()
    }
}

struct ResolvedPcm {
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
}

fn resolve_asset_pcm(project: &Project, ev: &EventEntry) -> ResolvedPcm {
    let asset: Option<&AssetEntry> = ev
        .asset_index
        .and_then(|i| project.assets.get(i));
    let freq = asset.and_then(|a| a.demo_freq_hz).unwrap_or(440.0);
    let rate = 48_000u32;
    let frames = rate as usize / 2; // 0.5 s
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

#[allow(dead_code)]
fn _play_params() -> PlayParams {
    PlayParams::default()
}

#[allow(dead_code)]
fn _listener() -> Attributes3d {
    Attributes3d {
        position: Vec3::ZERO,
        ..Default::default()
    }
}
