//! Voice pool with pitch, gain, AHDSR, steal, and per-voice mix.

use crate::buffer::{BufferHandle, BufferStore, PcmBuffer};
use crate::envelope::Ahdsr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceHandle(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceState {
    Playing,
    Stopped,
}

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
    envelope: Option<Ahdsr>,
    env_t: f32,
    env_gain: f32,
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
        envelope: Option<Ahdsr>,
    ) -> Result<VoiceHandle, ()> {
        if self.active_count() >= self.max {
            return Err(());
        }
        let src_frames = buf.samples.len() / buf.channels.max(1) as usize;
        self.voices.push(Voice {
            id,
            buffer,
            cursor: 0.0,
            gain: params.gain.max(0.0),
            pitch: 1.0,
            looped: params.looped,
            src_rate: buf.sample_rate,
            src_channels: buf.channels,
            src_frames,
            priority,
            audibility,
            alive: true,
            envelope,
            env_t: 0.0,
            env_gain: 1.0,
        });
        Ok(VoiceHandle(id))
    }

    pub fn advance_envelopes(&mut self, dt: f32) {
        for v in self.voices.iter_mut() {
            if !v.alive {
                continue;
            }
            if let Some(env) = v.envelope {
                v.env_t += dt;
                v.env_gain = env.gain_at(v.env_t);
            }
        }
    }

    pub fn steal_lowest(&mut self, priority: i32, audibility: f32) -> bool {
        let mut best: Option<usize> = None;
        let mut best_score = f32::MAX;
        for (i, v) in self.voices.iter().enumerate() {
            if !v.alive {
                continue;
            }
            let score = v.priority as f32 * 1000.0 + v.audibility;
            if v.priority < priority || (v.priority == priority && v.audibility < audibility) {
                if score < best_score {
                    best_score = score;
                    best = Some(i);
                }
            }
        }
        if let Some(i) = best {
            self.voices[i].alive = false;
            true
        } else {
            false
        }
    }

    pub fn stop(&mut self, h: VoiceHandle) {
        for v in self.voices.iter_mut() {
            if v.id == h.0 {
                v.alive = false;
            }
        }
    }

    pub fn set_gain(&mut self, h: VoiceHandle, gain: f32) {
        for v in self.voices.iter_mut() {
            if v.id == h.0 {
                v.gain = gain.max(0.0);
            }
        }
    }

    pub fn set_pitch(&mut self, h: VoiceHandle, pitch: f32) {
        for v in self.voices.iter_mut() {
            if v.id == h.0 {
                v.pitch = pitch.max(0.01);
            }
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
        self.mix_each(frames, out_ch, out_rate, buffers, |_id, slice| {
            for (d, s) in out.iter_mut().zip(slice.iter()) {
                *d += *s;
            }
        });
    }

    /// Mix each live voice into a temporary buffer and invoke `on_voice(id, samples)`.
    pub fn mix_each(
        &mut self,
        frames: usize,
        out_ch: usize,
        out_rate: u32,
        buffers: &BufferStore,
        mut on_voice: impl FnMut(u64, &[f32]),
    ) {
        let mut temp = vec![0.0f32; frames * out_ch];
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
            temp.fill(0.0);
            let ratio = (out_rate as f64 / voice.src_rate.max(1) as f64) / voice.pitch as f64;
            let env_g = voice.env_gain;
            let id = voice.id;

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
                    temp[f * out_ch + c] = s * voice.gain * env_g;
                }
                voice.cursor += 1.0;
            }
            on_voice(id, &temp);
        }
    }
}
