//! # ramod_runtime — open-source real-time audio engine (S1 MVP)
//!
//! Headless-friendly mixer that plays interleaved `f32` PCM buffers.
//! Optional `backend-cpal` opens a device stream.
//!
//! ## Thread model
//!
//! - **Control thread**: [`Runtime::play`], [`stop`], [`set_voice_gain`], [`tick`], [`render`].
//! - **Audio callback** (future): pull via shared `Arc<Mutex<Runtime>>`.
//!
//! ## Example
//!
//! ```ignore
//! let mut rt = Runtime::new(RuntimeConfig { open_device: false, ..Default::default() })?;
//! let buf = rt.load_pcm(pcm, 1, 48_000)?;
//! let v = rt.play(buf, PlayParams::default())?;
//! let mut out = vec![0.0f32; 4800];
//! rt.render(&mut out);
//! rt.stop(v);
//! ```

mod backend;
mod buffer;
mod bus;
mod error;
mod voice;

pub use backend::{NullBackend, OutputBackend};
pub use buffer::{BufferHandle, PcmBuffer};
pub use bus::MasterBus;
pub use error::RuntimeError;
pub use voice::{PlayParams, VoiceHandle, VoiceState};

use std::sync::atomic::{AtomicU64, Ordering};

use buffer::BufferStore;
use voice::VoicePool;

pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;
pub const DEFAULT_CHANNELS: u16 = 2;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub max_voices: usize,
    /// Open a real device when `backend-cpal` is enabled.
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
    buffers: BufferStore,
    voices: VoicePool,
    master: MasterBus,
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
            voices: VoicePool::new(cfg.max_voices),
            buffers: BufferStore::new(),
            master: MasterBus::new(),
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
        let buf = self
            .buffers
            .get(buffer)
            .ok_or(RuntimeError::InvalidHandle)?;
        let id = self.next_voice_id.fetch_add(1, Ordering::Relaxed);
        self.voices
            .start(id, buffer, buf, params, self.sample_rate)
            .map_err(|_| RuntimeError::NoFreeVoice)
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

    /// Advance transport (S1: reaps finished voices).
    pub fn tick(&mut self, _dt: f32) {
        self.voices.reap_finished();
    }

    /// Mix `out.len() / channels` frames into `out` (interleaved f32).
    pub fn render(&mut self, out: &mut [f32]) {
        let ch = self.channels as usize;
        if ch == 0 || out.len() % ch != 0 {
            out.fill(0.0);
            return;
        }
        let frames = out.len() / ch;
        out.fill(0.0);
        self.voices
            .mix_into(out, frames, ch, self.sample_rate, &self.buffers);
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

    #[test]
    fn plays_and_mixes_pcm() {
        let mut rt = Runtime::new(RuntimeConfig {
            open_device: false,
            sample_rate: 48_000,
            channels: 1,
            max_voices: 8,
        })
        .unwrap();
        let pcm = sine(4800, 48_000, 440.0);
        let buf = rt.load_pcm(pcm, 1, 48_000).unwrap();
        let v = rt.play(buf, PlayParams::default()).unwrap();
        assert_eq!(rt.active_voices(), 1);
        let out = rt.render_seconds(0.05);
        assert!(out.iter().any(|s| s.abs() > 0.01));
        rt.stop(v);
        rt.tick(0.0);
        assert_eq!(rt.active_voices(), 0);
    }

    #[test]
    fn master_gain_scales_output() {
        let mut rt = Runtime::new(RuntimeConfig {
            open_device: false,
            sample_rate: 48_000,
            channels: 1,
            max_voices: 4,
        })
        .unwrap();
        let buf = rt.load_pcm(sine(4800, 48_000, 220.0), 1, 48_000).unwrap();
        rt.play(buf, PlayParams::default()).unwrap();
        let peak_full = rt
            .render_seconds(0.02)
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);

        let mut rt = Runtime::new(RuntimeConfig {
            open_device: false,
            sample_rate: 48_000,
            channels: 1,
            max_voices: 4,
        })
        .unwrap();
        let buf = rt.load_pcm(sine(4800, 48_000, 220.0), 1, 48_000).unwrap();
        rt.play(buf, PlayParams::default()).unwrap();
        rt.master_mut().set_gain(0.5);
        let peak_half = rt
            .render_seconds(0.02)
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);
        assert!(peak_half < peak_full * 0.75);
    }
}
