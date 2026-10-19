//! Voice pool and mixing.

use crate::buffer::{BufferHandle, BufferStore, PcmBuffer};

/// Handle to a playing voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceHandle(pub(crate) u64);

/// Playback parameters for [`crate::Runtime::play`].
#[derive(Debug, Clone, Copy)]
pub struct PlayParams {
    pub gain: f32,
    pub looped: bool,
}

impl Default for PlayParams {
    fn default() -> Self {
        Self {
            gain: 1.0,
            looped: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceState {
    Playing,
    Stopped,
}

struct Voice {
    id: u64,
    buffer: BufferHandle,
    cursor: f64,
    gain: f32,
    pitch: f32,
    looped: bool,
    src_rate: u32,
    src_channels: u16,
    src_frames: usize,
    priority: i32,
    audibility: f32,
    alive: bool,
}

pub(crate) struct VoicePool {
    max: usize,
    voices: Vec<Voice>,
}

impl VoicePool {
    pub fn new(max: usize) -> Self {
        Self {
            max,
            voices: Vec::with_capacity(max),
        }
    }

    pub fn active_count(&self) -> usize {
        self.voices.iter().filter(|v| v.alive).count()
    }

    pub fn start(
        &mut self,
        id: u64,
        buffer: BufferHandle,
        buf: &PcmBuffer,
        params: PlayParams,
        _out_rate: u32,
        priority: i32,
        audibility: f32,
    ) -> Result<VoiceHandle, ()> {
        self.reap_finished();
        if self.active_count() >= self.max {
            return Err(());
        }
        self.voices.push(Voice {
            id,
            buffer,
            cursor: 0.0,
            gain: params.gain.max(0.0),
            pitch: 1.0,
            looped: params.looped,
            src_rate: buf.sample_rate,
            src_channels: buf.channels,
            src_frames: buf.frames(),
            priority,
            audibility,
            alive: true,
        });
        Ok(VoiceHandle(id))
    }

    /// Steal the least important voice if `priority`/`audibility` wins.
    pub fn steal_lowest(&mut self, priority: i32, audibility: f32) -> bool {
        let score = |v: &Voice| v.priority as f32 * 1000.0 + v.audibility;
        let new_score = priority as f32 * 1000.0 + audibility;
        let victim = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, v)| v.alive)
            .min_by(|a, b| {
                score(a.1)
                    .partial_cmp(&score(b.1))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        if let Some((idx, v)) = victim {
            if score(v) < new_score {
                self.voices[idx].alive = false;
                self.reap_finished();
                return true;
            }
        }
        false
    }

    pub fn stop(&mut self, h: VoiceHandle) {
        if let Some(v) = self.voices.iter_mut().find(|v| v.id == h.0) {
            v.alive = false;
        }
    }

    pub fn set_gain(&mut self, h: VoiceHandle, gain: f32) {
        if let Some(v) = self.voices.iter_mut().find(|v| v.id == h.0) {
            v.gain = gain;
        }
    }

    pub fn set_pitch(&mut self, h: VoiceHandle, pitch: f32) {
        if let Some(v) = self.voices.iter_mut().find(|v| v.id == h.0) {
            v.pitch = pitch.clamp(0.25, 4.0);
        }
    }

    pub fn reap_finished(&mut self) {
        self.voices.retain(|v| v.alive);
    }

    pub fn mix_into(
        &mut self,
        out: &mut [f32],
        frames: usize,
        out_ch: usize,
        out_rate: u32,
        buffers: &BufferStore,
    ) {
        for voice in self.voices.iter_mut() {
            if !voice.alive {
                continue;
            }
            let Some(buf) = buffers.get(voice.buffer) else {
                voice.alive = false;
                continue;
            };
            let src_ch = voice.src_channels as usize;
            if src_ch == 0 || voice.src_frames == 0 {
                voice.alive = false;
                continue;
            }
            let ratio = (out_rate as f64 / voice.src_rate.max(1) as f64) / voice.pitch as f64;

            for f in 0..frames {
                let src_pos = voice.cursor / ratio;
                let mut src_frame = src_pos.floor() as usize;
                if src_frame >= voice.src_frames {
                    if voice.looped {
                        src_frame %= voice.src_frames;
                        voice.cursor = (src_frame as f64) * ratio;
                    } else {
                        voice.alive = false;
                        break;
                    }
                }
                let frac = src_pos.fract() as f32;
                let next = (src_frame + 1).min(voice.src_frames.saturating_sub(1));
                for c in 0..out_ch {
                    let sc = c.min(src_ch - 1);
                    let i0 = src_frame * src_ch + sc;
                    let i1 = next * src_ch + sc;
                    let s0 = buf.samples.get(i0).copied().unwrap_or(0.0);
                    let s1 = buf.samples.get(i1).copied().unwrap_or(s0);
                    let s = s0 + (s1 - s0) * frac;
                    out[f * out_ch + c] += s * voice.gain;
                }
                voice.cursor += 1.0;
            }
        }
    }
}
