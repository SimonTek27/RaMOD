//! Core effects (S9.2 / S9.3) plus the effect chain: EQ, compressor, limiter,
//! time-stretch and pitch-shift. Pure Rust, no dependencies.

use crate::selection::Selection;

pub fn silence(samples: &mut [f32], channels: u16, sel: Selection) {
    let (a, b) = sel.sample_range(channels);
    let b = b.min(samples.len());
    let a = a.min(b);
    for s in &mut samples[a..b] {
        *s = 0.0;
    }
}

pub fn gain(samples: &mut [f32], channels: u16, sel: Selection, linear: f32) {
    let (a, b) = sel.sample_range(channels);
    let b = b.min(samples.len());
    let a = a.min(b);
    for s in &mut samples[a..b] {
        *s *= linear;
    }
}

pub fn normalize(samples: &mut [f32], channels: u16, sel: Selection, target_peak: f32) {
    let (a, b) = sel.sample_range(channels);
    let b = b.min(samples.len());
    let a = a.min(b);
    if a >= b {
        return;
    }
    let peak = samples[a..b].iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if peak < 1e-8 {
        return;
    }
    let g = target_peak.clamp(0.0, 1.0) / peak;
    for s in &mut samples[a..b] {
        *s *= g;
    }
}

/// Reverse selected region in time (preserves interleaving).
pub fn reverse(samples: &mut [f32], channels: u16, sel: Selection) {
    let ch = channels as usize;
    if ch == 0 || sel.is_empty() {
        return;
    }
    let start = sel.start;
    let end = sel.end;
    let frames = end - start;
    if frames < 2 {
        return;
    }
    let mut i = 0;
    let mut j = frames - 1;
    while i < j {
        for c in 0..ch {
            let ai = (start + i) * ch + c;
            let bi = (start + j) * ch + c;
            if ai < samples.len() && bi < samples.len() {
                samples.swap(ai, bi);
            }
        }
        i += 1;
        j -= 1;
    }
}

/// Linear fade in (`fade_in = true`) or out across selection.
pub fn fade(samples: &mut [f32], channels: u16, sel: Selection, fade_in: bool) {
    let ch = channels as usize;
    if ch == 0 || sel.is_empty() {
        return;
    }
    let frames = sel.len();
    if frames == 0 {
        return;
    }
    for f in 0..frames {
        let t = f as f32 / frames as f32;
        let g = if fade_in { t } else { 1.0 - t };
        for c in 0..ch {
            let i = (sel.start + f) * ch + c;
            if i < samples.len() {
                samples[i] *= g;
            }
        }
    }
}

/// dB to linear amplitude.
pub fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Linear amplitude to dB (clamped away from zero).
pub fn linear_to_db(x: f32) -> f32 {
    20.0 * x.abs().max(1e-12).log10()
}

/// One peaking EQ band (RBJ audio-EQ cookbook).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqBand {
    /// Centre frequency in Hz.
    pub freq: f32,
    /// Boost / cut in dB, applied at the centre frequency.
    pub gain_db: f32,
    /// Bandwidth; higher is narrower.
    pub q: f32,
}

impl EqBand {
    pub fn new(freq: f32, gain_db: f32, q: f32) -> Self {
        Self {
            freq,
            gain_db,
            q: q.max(0.05),
        }
    }
}

/// Direct-form biquad section.
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Biquad {
    fn peaking(freq: f32, gain_db: f32, q: f32, sample_rate: u32) -> Self {
        let fs = sample_rate.max(1) as f32;
        let f = freq.clamp(1.0, fs * 0.49);
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = std::f32::consts::TAU * f / fs;
        let cos = w0.cos();
        let alpha = w0.sin() / (2.0 * q.max(0.05));
        let a0 = 1.0 + alpha / a;
        Self {
            b0: (1.0 + alpha * a) / a0,
            b1: (-2.0 * cos) / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: (-2.0 * cos) / a0,
            a2: (1.0 - alpha / a) / a0,
        }
    }

    fn process(&self, x: f32, s: &mut BiquadState) -> f32 {
        let y = self.b0 * x + self.b1 * s.x1 + self.b2 * s.x2 - self.a1 * s.y1 - self.a2 * s.y2;
        s.x2 = s.x1;
        s.x1 = x;
        s.y2 = s.y1;
        s.y1 = y;
        y
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

/// Apply peaking EQ bands to an interleaved region, in place.
pub fn eq(region: &mut [f32], channels: u16, bands: &[EqBand], sample_rate: u32) {
    let ch = channels.max(1) as usize;
    if region.is_empty() || ch == 0 {
        return;
    }
    for band in bands {
        let bq = Biquad::peaking(band.freq, band.gain_db, band.q, sample_rate);
        let mut states = vec![BiquadState::default(); ch];
        for frame in region.chunks_exact_mut(ch) {
            for (c, s) in frame.iter_mut().enumerate() {
                *s = bq.process(*s, &mut states[c]);
            }
        }
    }
}

/// Compressor settings; thresholds and makeup are in dB.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressorParams {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub makeup_db: f32,
}

impl Default for CompressorParams {
    fn default() -> Self {
        Self {
            threshold_db: -18.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 80.0,
            makeup_db: 0.0,
        }
    }
}

/// Stereo-linked peak compressor: one gain per frame, so the image stays put.
pub fn compress(region: &mut [f32], channels: u16, params: CompressorParams, sample_rate: u32) {
    let ch = channels.max(1) as usize;
    if region.is_empty() || ch == 0 {
        return;
    }
    let fs = sample_rate.max(1) as f32;
    let thr = params.threshold_db;
    let ratio = params.ratio.max(1.0);
    let makeup = db_to_linear(params.makeup_db);
    let attack_tau = (params.attack_ms.max(0.01) / 1000.0) * fs;
    let release_tau = (params.release_ms.max(0.01) / 1000.0) * fs;
    let a_att = (-1.0 / attack_tau).exp();
    let a_rel = (-1.0 / release_tau).exp();
    let mut env = 0.0f32;
    for frame in region.chunks_exact_mut(ch) {
        let peak = frame.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let coeff = if peak > env { a_att } else { a_rel };
        env = coeff * env + (1.0 - coeff) * peak;
        let over = linear_to_db(env) - thr;
        let gain_db = if over > 0.0 {
            -over * (1.0 - 1.0 / ratio)
        } else {
            0.0
        };
        let g = db_to_linear(gain_db) * makeup;
        for s in frame.iter_mut() {
            *s *= g;
        }
    }
}

/// Peak limiter with instant attack and 50 ms release: output never exceeds the ceiling.
pub fn limiter(region: &mut [f32], channels: u16, ceiling_db: f32, sample_rate: u32) {
    let ch = channels.max(1) as usize;
    if region.is_empty() || ch == 0 {
        return;
    }
    let ceiling = db_to_linear(ceiling_db).clamp(1e-4, 1.0);
    let a_rel = (-1.0f32 / (sample_rate.max(1) as f32 * 0.05)).exp();
    let mut gain = 1.0f32;
    for frame in region.chunks_exact_mut(ch) {
        let peak = frame.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let target = if peak > ceiling { ceiling / peak } else { 1.0 };
        if target < gain {
            gain = target;
        } else {
            gain = a_rel * gain + (1.0 - a_rel) * target.min(1.0);
        }
        for s in frame.iter_mut() {
            *s *= gain;
        }
    }
}

/// Linear-interpolation resampler. `ratio > 1` plays faster: shorter and higher.
pub fn resample(region: &[f32], channels: u16, ratio: f32) -> Vec<f32> {
    let ch = channels.max(1) as usize;
    let frames = region.len() / ch;
    if frames == 0 || !ratio.is_finite() || ratio <= 0.0 {
        return region.to_vec();
    }
    let out_frames = ((frames as f32 / ratio).round() as usize).max(1);
    let mut out = vec![0.0f32; out_frames * ch];
    for f in 0..out_frames {
        let src = f as f32 * ratio;
        let idx = src.floor() as usize;
        let frac = src - idx as f32;
        let i0 = idx.min(frames - 1);
        let i1 = (i0 + 1).min(frames - 1);
        for c in 0..ch {
            let a = region[i0 * ch + c];
            let b = region[i1 * ch + c];
            out[f * ch + c] = a + (b - a) * frac;
        }
    }
    out
}

/// Hann-windowed overlap-add time stretch. Duration scales by `ratio`, pitch does not.
pub fn time_stretch(region: &[f32], channels: u16, ratio: f32, sample_rate: u32) -> Vec<f32> {
    let ch = channels.max(1) as usize;
    let frames = region.len() / ch;
    if frames == 0 || !ratio.is_finite() || ratio <= 0.0 {
        return region.to_vec();
    }
    let fs = sample_rate.max(8_000) as f32;
    let mut win = (fs * 0.046) as usize;
    win = win.clamp(256, 4096);
    win -= win % 2;
    let hop_out = (win / 2).max(1);
    let hop_in = ((hop_out as f32 / ratio).round() as usize).max(1);
    let out_frames = ((frames as f32 * ratio).round() as usize).max(1);
    let mut out = vec![0.0f32; out_frames * ch];
    let mut norm = vec![0.0f32; out_frames];
    let window: Vec<f32> = (0..win)
        .map(|i| {
            let t = i as f32 / win as f32;
            0.5 - 0.5 * (std::f32::consts::TAU * t).cos()
        })
        .collect();
    let mut pos_out = 0usize;
    let mut pos_in = 0usize;
    while pos_out < out_frames {
        for i in 0..win {
            let o = pos_out + i;
            if o >= out_frames {
                break;
            }
            let src = (pos_in + i).min(frames - 1);
            let g = window[i];
            for c in 0..ch {
                out[o * ch + c] += region[src * ch + c] * g;
            }
            norm[o] += g;
        }
        pos_out += hop_out;
        pos_in += hop_in;
        if pos_in >= frames {
            break;
        }
    }
    for (o, n) in norm.iter().enumerate() {
        if *n > 1e-6 {
            for c in 0..ch {
                out[o * ch + c] /= *n;
            }
        }
    }
    out
}

/// Shift pitch by `semitones` while keeping the region length unchanged.
pub fn pitch_shift(region: &[f32], channels: u16, semitones: f32, sample_rate: u32) -> Vec<f32> {
    let ch = channels.max(1) as usize;
    let target_frames = region.len() / ch;
    if target_frames == 0 || semitones.abs() < 1e-3 {
        return region.to_vec();
    }
    let r = 2f32.powf(semitones / 12.0);
    let stretched = time_stretch(region, channels, r, sample_rate);
    let mut out = resample(&stretched, channels, r);
    out.resize(target_frames * ch, 0.0);
    out.truncate(target_frames * ch);
    out
}
