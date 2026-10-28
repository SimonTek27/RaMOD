//! S7 — Runtime profiler: capture, session timeline, 3D debug, spectrum.

use std::time::Instant;

use crate::mixer::BusId;
use crate::spatial::Vec3;

/// One captured frame of runtime state.
#[derive(Debug, Clone)]
pub struct ProfilerFrame {
    /// Monotonic frame index.
    pub index: u64,
    /// Seconds since session start.
    pub time: f32,
    /// Wall-clock microseconds spent in last `render` (approx).
    pub cpu_render_us: u32,
    pub active_voices: usize,
    pub active_instances: usize,
    /// Peak linear on Master bus.
    pub master_peak: f32,
    pub master_rms: f32,
    /// Sample of instance paths currently playing.
    pub playing_events: Vec<String>,
    /// Key parameters observed (path, name, value).
    pub params: Vec<(String, String, f32)>,
    /// 3D emitter positions for debug view.
    pub emitters: Vec<EmitterDebug>,
}

/// Emitter position for optional 3D debug overlay (S7.3).
#[derive(Debug, Clone, Copy)]
pub struct EmitterDebug {
    pub position: Vec3,
    pub velocity: Vec3,
    pub audibility: f32,
}

/// Recorded session of profiler frames.
#[derive(Debug, Default)]
pub struct ProfilerSession {
    pub frames: Vec<ProfilerFrame>,
    pub max_frames: usize,
    start: Option<Instant>,
    next_index: u64,
    enabled: bool,
}

impl ProfilerSession {
    pub fn new(max_frames: usize) -> Self {
        Self {
            frames: Vec::with_capacity(max_frames.min(4096)),
            max_frames: max_frames.max(1),
            start: None,
            next_index: 0,
            enabled: false,
        }
    }

    pub fn enable(&mut self, on: bool) {
        self.enabled = on;
        if on && self.start.is_none() {
            self.start = Some(Instant::now());
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.next_index = 0;
        self.start = if self.enabled {
            Some(Instant::now())
        } else {
            None
        };
    }

    pub fn push(&mut self, mut frame: ProfilerFrame) {
        if !self.enabled {
            return;
        }
        let t = self
            .start
            .map(|s| s.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        frame.time = t;
        frame.index = self.next_index;
        self.next_index += 1;
        if self.frames.len() >= self.max_frames {
            self.frames.remove(0);
        }
        self.frames.push(frame);
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn latest(&self) -> Option<&ProfilerFrame> {
        self.frames.last()
    }

    /// NDJSON export for timeline tools / editor (S7.2).
    pub fn to_ndjson(&self) -> String {
        let mut out = String::new();
        for f in &self.frames {
            out.push_str(&frame_to_json(f));
            out.push('\n');
        }
        out
    }

    /// Peak voice count across the session.
    pub fn peak_voices(&self) -> usize {
        self.frames.iter().map(|f| f.active_voices).max().unwrap_or(0)
    }

    /// Average render CPU µs.
    pub fn avg_cpu_us(&self) -> f32 {
        if self.frames.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.frames.iter().map(|f| f.cpu_render_us as u64).sum();
        sum as f32 / self.frames.len() as f32
    }
}

fn frame_to_json(f: &ProfilerFrame) -> String {
    let events: String = f
        .playing_events
        .iter()
        .map(|e| format!("\"{}\"", e.replace('"', "\"")))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"index":{},"time":{:.4},"cpu_us":{},"voices":{},"instances":{},"master_peak":{:.4},"master_rms":{:.4},"events":[{}]}}"#,
        f.index,
        f.time,
        f.cpu_render_us,
        f.active_voices,
        f.active_instances,
        f.master_peak,
        f.master_rms,
        events
    )
}

/// Simple magnitude-band spectrum from interleaved PCM (S7.4, UI-side).
///
/// Not a full FFT — uses block energy in `bands` equal partitions of the buffer
/// (good enough for a meter strip without external crates).
#[derive(Debug, Clone)]
pub struct SpectrumAnalyzer {
    pub bands: usize,
    pub levels: Vec<f32>,
}

impl SpectrumAnalyzer {
    pub fn new(bands: usize) -> Self {
        let bands = bands.max(1);
        Self {
            bands,
            levels: vec![0.0; bands],
        }
    }

    /// Analyze mono or interleaved buffer; updates smoothed band levels 0..1.
    pub fn analyze(&mut self, samples: &[f32], channels: usize) {
        if samples.is_empty() || channels == 0 {
            for l in &mut self.levels {
                *l *= 0.9;
            }
            return;
        }
        // Downmix to mono magnitudes.
        let frames = samples.len() / channels;
        let mut mono = Vec::with_capacity(frames);
        for i in 0..frames {
            let mut s = 0.0f32;
            for c in 0..channels {
                s += samples[i * channels + c].abs();
            }
            mono.push(s / channels as f32);
        }
        let chunk = (mono.len() / self.bands).max(1);
        for b in 0..self.bands {
            let start = b * chunk;
            let end = ((b + 1) * chunk).min(mono.len());
            if start >= end {
                self.levels[b] *= 0.9;
                continue;
            }
            let mut peak = 0.0f32;
            for v in &mono[start..end] {
                if *v > peak {
                    peak = *v;
                }
            }
            // Smooth
            self.levels[b] = self.levels[b] * 0.7 + peak.clamp(0.0, 1.0) * 0.3;
        }
    }
}

/// Helper to time a closure in microseconds.
pub fn measure_us<R>(f: impl FnOnce() -> R) -> (R, u32) {
    let t0 = Instant::now();
    let r = f();
    let us = t0.elapsed().as_micros().min(u128::from(u32::MAX)) as u32;
    (r, us)
}

/// Bus meter snapshot for profiler UI.
#[derive(Debug, Clone, Copy)]
pub struct BusMeterSnap {
    pub id: BusId,
    pub peak: f32,
    pub rms: f32,
}
