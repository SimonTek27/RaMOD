//! Waveform overview for UI (S9.1).

#[derive(Debug, Clone, Copy, Default)]
pub struct PeakMinMax {
    pub min: f32,
    pub max: f32,
}

/// Downsample interleaved PCM into `buckets` min/max pairs (mono peak envelope).
pub fn peak_overview(samples: &[f32], channels: u16, buckets: usize) -> Vec<PeakMinMax> {
    let buckets = buckets.max(1);
    let ch = channels.max(1) as usize;
    let frames = samples.len() / ch;
    if frames == 0 {
        return vec![PeakMinMax::default(); buckets];
    }
    let mut out = Vec::with_capacity(buckets);
    for b in 0..buckets {
        let start = b * frames / buckets;
        let end = ((b + 1) * frames / buckets).max(start + 1).min(frames);
        let mut mn = 0.0f32;
        let mut mx = 0.0f32;
        for f in start..end {
            let mut s = 0.0f32;
            for c in 0..ch {
                s += samples[f * ch + c];
            }
            s /= ch as f32;
            mn = mn.min(s);
            mx = mx.max(s);
        }
        out.push(PeakMinMax { min: mn, max: mx });
    }
    out
}
