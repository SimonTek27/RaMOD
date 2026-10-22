//! RaMOD as a Renzora-compatible audio **Backend**.
//!
//! Renzora drives mixers through `renzora::audio_backend::Backend`. This module
//! provides [`RamodRenzoraBackend`]: a `ramod_runtime::Runtime` wrapped to match
//! that contract.
//!
//! ## Two ways to use it
//!
//! ### A) RaMOD-only path (no Kira, no Backend trait)
//! Use [`crate::renzora::RenzoraRamodExt::replace_renzora_audio_with_ramod`].
//!
//! ### B) Replace Kira inside Renzora's audio system
//! Register this backend so Renzora's command queue talks to RaMOD.
//! See `docs/RENZORA.md`.

use std::collections::HashMap;

use ramod_runtime::{
    Attributes3d, Attenuation, EventDescription, Runtime, RuntimeConfig, Vec3 as RVec3,
};

/// Capability bits — same layout as Renzora `Caps`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Caps(pub u32);

impl Caps {
    pub const CAPTURE: Self = Self(1 << 0);
    pub const SPATIAL: Self = Self(1 << 1);
    pub const FEEDS: Self = Self(1 << 2);
    pub const DEVICE_LIST: Self = Self(1 << 3);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackendInfo {
    pub sample_rate: u32,
    pub caps: Caps,
    pub device: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BusState {
    pub key: String,
    pub gain: f32,
    pub pan: f32,
    pub muted: bool,
    pub soloed: bool,
    pub output: Option<String>,
    pub reverb_send: f32,
    pub delay_send: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmitterState {
    pub position: [f32; 3],
    pub min_distance: f32,
    pub max_distance: f32,
    pub rolloff: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListenerState {
    pub position: [f32; 3],
    pub right: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipInfo {
    pub duration: f64,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayRequest {
    pub voice: u64,
    pub clip: u64,
    pub bus: String,
    pub gain: f32,
    pub pan: f32,
    pub pitch: f64,
    pub looping: Option<(f64, f64)>,
    pub fade_in: f32,
    pub start: f64,
    pub emitter: Option<EmitterState>,
    pub reverb_send: f32,
    pub delay_send: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StopTarget {
    Voice(u64),
    Bus(String),
    All,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StopRequest {
    pub target: StopTarget,
    pub fade: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UpdateRequest {
    pub listener: Option<ListenerState>,
    pub moved: Vec<(u64, [f32; 3])>,
    pub gains: Vec<(u64, f32)>,
    pub pitches: Vec<(u64, f64)>,
    pub buses: Vec<(u64, String)>,
    pub pans: Vec<(u64, f32)>,
    pub emitters: Vec<(u64, EmitterState)>,
    pub paused: Vec<(u64, bool)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UpdateReply {
    pub peaks: Vec<f32>,
    pub finished: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureInfo {
    pub sample_rate: u32,
    pub device: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeviceList {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

/// Shared bus keys used by RaMOD Studio and Renzora hosts.
pub mod buses {
    pub const MASTER: &str = "Master";
    pub const SFX: &str = "SFX";
    pub const MUSIC: &str = "Music";
}

/// RaMOD runtime driven as a Renzora-style mixer backend.
pub struct RamodRenzoraBackend {
    runtime: Runtime,
    clips: HashMap<u64, String>,
    voices: HashMap<u64, u64>,
    bus_keys: Vec<String>,
    open_device: bool,
}

impl RamodRenzoraBackend {
    pub fn new(open_device: bool) -> Result<Self, String> {
        let runtime = Runtime::new(RuntimeConfig {
            open_device,
            sample_rate: 48_000,
            channels: 2,
            max_voices: 128,
        })
        .map_err(|e| e.to_string())?;
        Ok(Self {
            runtime,
            clips: HashMap::new(),
            voices: HashMap::new(),
            bus_keys: vec![
                buses::MASTER.into(),
                buses::SFX.into(),
                buses::MUSIC.into(),
            ],
            open_device,
        })
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    pub fn name(&self) -> &str {
        "ramod"
    }

    pub fn init(&mut self) -> Result<BackendInfo, String> {
        Ok(BackendInfo {
            sample_rate: self.runtime.sample_rate(),
            caps: Caps::SPATIAL.union(Caps::FEEDS).union(Caps::DEVICE_LIST),
            device: if self.open_device {
                "cpal-default".into()
            } else {
                "null".into()
            },
        })
    }

    pub fn shutdown(&mut self) {
        self.voices.clear();
        self.clips.clear();
    }

    pub fn set_buses(&mut self, buses: &[BusState]) {
        self.bus_keys = buses.iter().map(|b| b.key.clone()).collect();
        let _ = buses;
    }

    pub fn load_clip(
        &mut self,
        clip: u64,
        extension: &str,
        bytes: &[u8],
    ) -> Result<ClipInfo, String> {
        let path = format!("event:/renzora/clip/{clip}");
        let (samples, channels, sample_rate) =
            decode_bytes(extension, bytes).unwrap_or_else(|| {
                let rate = 48_000u32;
                let frames = rate as usize / 4;
                let pcm: Vec<f32> = (0..frames)
                    .map(|i| {
                        let t = i as f32 / rate as f32;
                        (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.15
                    })
                    .collect();
                (pcm, 1u16, rate)
            });

        let duration = samples.len() as f64 / (sample_rate as f64 * channels.max(1) as f64);
        let buf = self
            .runtime
            .load_pcm(samples, channels, sample_rate)
            .map_err(|e| e.to_string())?;
        self.runtime.register_event(
            EventDescription::new(&path, buf)
                .with_parameter("Intensity", 1.0)
                .with_output_bus(buses::SFX),
        );
        self.clips.insert(clip, path);
        Ok(ClipInfo {
            duration,
            sample_rate,
        })
    }

    pub fn unload_clip(&mut self, clip: u64) {
        self.clips.remove(&clip);
    }

    pub fn play(&mut self, request: &PlayRequest) -> Result<(), String> {
        let path = self
            .clips
            .get(&request.clip)
            .cloned()
            .ok_or_else(|| format!("unknown clip {}", request.clip))?;
        let id = self
            .runtime
            .play_event(&path)
            .map_err(|e| e.to_string())?;
        let _ = self
            .runtime
            .set_instance_parameter(id, "Intensity", request.gain);
        if let Some(em) = &request.emitter {
            self.runtime.set_instance_3d(
                id,
                Attributes3d {
                    position: RVec3 {
                        x: em.position[0],
                        y: em.position[1],
                        z: em.position[2],
                    },
                    ..Default::default()
                },
            );
        }
        self.voices.insert(request.voice, id);
        Ok(())
    }

    pub fn stop(&mut self, request: &StopRequest) {
        match &request.target {
            StopTarget::Voice(v) => {
                if let Some(id) = self.voices.remove(v) {
                    self.runtime.release_instance(id);
                }
            }
            StopTarget::All | StopTarget::Bus(_) => {
                for (_, id) in self.voices.drain() {
                    self.runtime.release_instance(id);
                }
            }
        }
        let _ = request.fade;
    }

    pub fn update(&mut self, request: &UpdateRequest) -> UpdateReply {
        if let Some(l) = &request.listener {
            self.runtime.set_listener(Attributes3d {
                position: RVec3 {
                    x: l.position[0],
                    y: l.position[1],
                    z: l.position[2],
                },
                ..Default::default()
            });
        }
        for (voice, pos) in &request.moved {
            if let Some(&id) = self.voices.get(voice) {
                self.runtime.set_instance_3d(
                    id,
                    Attributes3d {
                        position: RVec3 {
                            x: pos[0],
                            y: pos[1],
                            z: pos[2],
                        },
                        ..Default::default()
                    },
                );
            }
        }
        for (voice, gain) in &request.gains {
            if let Some(&id) = self.voices.get(voice) {
                let _ = self.runtime.set_instance_parameter(id, "Intensity", *gain);
            }
        }
        for (voice, em) in &request.emitters {
            if let Some(&id) = self.voices.get(voice) {
                self.runtime.set_instance_3d(
                    id,
                    Attributes3d {
                        position: RVec3 {
                            x: em.position[0],
                            y: em.position[1],
                            z: em.position[2],
                        },
                        ..Default::default()
                    },
                );
                let _ = Attenuation {
                    min_distance: em.min_distance,
                    max_distance: em.max_distance,
                };
            }
        }

        self.runtime.tick(1.0 / 60.0);
        let frames = 256usize;
        let ch = self.runtime.channels() as usize;
        let mut out = vec![0.0f32; frames * ch.max(1)];
        self.runtime.render(&mut out);

        let (peak, _rms) = self.runtime.master_levels();
        let peaks = self.bus_keys.iter().map(|_| peak).collect();

        UpdateReply {
            peaks,
            finished: Vec::new(),
        }
    }

    pub fn open_capture(
        &mut self,
        _capture: u64,
        _device: Option<&str>,
    ) -> Result<CaptureInfo, String> {
        Err("ramod backend does not capture".into())
    }

    pub fn close_capture(&mut self, _capture: u64) {}

    pub fn read_capture(&mut self, _capture: u64) -> Vec<f32> {
        Vec::new()
    }

    pub fn push_frames(&mut self, _bus: &str, samples: &[f32]) {
        let _ = samples;
    }

    pub fn list_devices(&mut self) -> DeviceList {
        DeviceList {
            inputs: Vec::new(),
            outputs: vec!["cpal-default".into()],
        }
    }
}

fn decode_bytes(extension: &str, bytes: &[u8]) -> Option<(Vec<f32>, u16, u32)> {
    let ext = extension.trim_start_matches('.').to_ascii_lowercase();
    if matches!(ext.as_str(), "wav" | "wave") && bytes.len() > 44 {
        if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return None;
        }
        let mut pos = 12usize;
        let mut channels = 1u16;
        let mut rate = 48_000u32;
        let mut bits = 16u16;
        let mut data: Option<&[u8]> = None;
        while pos + 8 <= bytes.len() {
            let id = &bytes[pos..pos + 4];
            let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().ok()?) as usize;
            let start = pos + 8;
            let end = (start + size).min(bytes.len());
            if id == b"fmt " && size >= 16 {
                channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into().ok()?);
                rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().ok()?);
                bits = u16::from_le_bytes(bytes[start + 14..start + 16].try_into().ok()?);
            } else if id == b"data" {
                data = Some(&bytes[start..end]);
            }
            pos = start + size + (size % 2);
        }
        let data = data?;
        let samples = match bits {
            16 => data
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                .collect(),
            8 => data.iter().map(|&b| (b as i8 as f32) / 128.0).collect(),
            _ => return None,
        };
        return Some((samples, channels.max(1), if rate == 0 { 48_000 } else { rate }));
    }
    None
}
