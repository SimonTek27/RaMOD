//! Minimal insert DSP effects (S4.3).

/// One insert slot on a bus.
#[derive(Debug, Clone)]
pub enum InsertEffect {
    Gain { gain_db: f32 },
    /// One-pole low-pass. `cutoff_hz` relative to engine sample rate.
    LowPass { cutoff_hz: f32 },
    /// Soft knee-ish compressor (simplified).
    Compressor {
        threshold_db: f32,
        ratio: f32,
        makeup_db: f32,
    },
}

impl InsertEffect {
    pub fn gain(gain_db: f32) -> Self {
        Self::Gain { gain_db }
    }

    pub fn lowpass(cutoff_hz: f32) -> Self {
        Self::LowPass { cutoff_hz }
    }

    pub fn compressor(threshold_db: f32, ratio: f32, makeup_db: f32) -> Self {
        Self::Compressor {
            threshold_db,
            ratio,
            makeup_db,
        }
    }
}

fn db_to_lin(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Per-channel filter memory for LowPass.
#[derive(Debug, Clone, Default)]
pub struct EffectState {
    lp_z: Vec<f32>,
}

impl EffectState {
    pub fn ensure_channels(&mut self, n: usize) {
        if self.lp_z.len() != n {
            self.lp_z = vec![0.0; n];
        }
    }
}

/// Process one interleaved buffer in-place through an effect.
pub fn process_effect(
    effect: &InsertEffect,
    state: &mut EffectState,
    samples: &mut [f32],
    channels: usize,
    sample_rate: u32,
) {
    if channels == 0 {
        return;
    }
    state.ensure_channels(channels);
    match effect {
        InsertEffect::Gain { gain_db } => {
            let g = db_to_lin(*gain_db);
            for s in samples.iter_mut() {
                *s *= g;
            }
        }
        InsertEffect::LowPass { cutoff_hz } => {
            let sr = sample_rate.max(1) as f32;
            let fc = cutoff_hz.clamp(20.0, sr * 0.45);
            // Simple one-pole: y += a * (x - y)
            let a = 1.0 - (-2.0 * std::f32::consts::PI * fc / sr).exp();
            for frame in samples.chunks_mut(channels) {
                for (c, s) in frame.iter_mut().enumerate() {
                    let y = state.lp_z[c] + a * (*s - state.lp_z[c]);
                    state.lp_z[c] = y;
                    *s = y;
                }
            }
        }
        InsertEffect::Compressor {
            threshold_db,
            ratio,
            makeup_db,
        } => {
            let thresh = db_to_lin(*threshold_db);
            let ratio = ratio.max(1.0);
            let makeup = db_to_lin(*makeup_db);
            for s in samples.iter_mut() {
                let abs = s.abs();
                if abs > thresh {
                    let over = abs / thresh;
                    // Compress excess toward threshold.
                    let compressed = thresh * over.powf(1.0 / ratio);
                    let g = if abs > 1e-8 { compressed / abs } else { 1.0 };
                    *s *= g;
                }
                *s *= makeup;
            }
        }
    }
}
