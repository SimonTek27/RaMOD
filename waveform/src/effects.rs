//! Core effects (S9.2 / S9.3).

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
    let peak = samples[a..b]
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
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
