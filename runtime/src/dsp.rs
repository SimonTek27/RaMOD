//! Minimal insert DSP effects (S4.3 / S7.12).

/// One insert slot on a bus.
#[derive(Debug, Clone)]
pub enum InsertEffect {
    Gain { gain_db: f32 },
    /// One-pole low-pass. `cutoff_hz` relative to engine sample rate.
    LowPass { cutoff_hz: f32 },
    /// One-pole high-pass.
    HighPass { cutoff_hz: f32 },
    /// Soft knee-ish compressor (simplified).
    Compressor {
        threshold_db: f32,
        ratio: f32,
        makeup_db: f32,
    },
    /// Simple mono feedback delay (wet/dry blend).
    Reverb {
        delay_ms: f32,
        feedback: f32,
        wet: f32,
    },
}

impl InsertEffect {
    pub fn gain(gain_db: f32) -> Self {
        Self::Gain { gain_db }
    }

    pub fn lowpass(cutoff_hz: f32) -> Self {
        Self::LowPass { cutoff_hz }
    }

    pub fn highpass(cutoff_hz: f32) -> Self {
        Self::HighPass { cutoff_hz }
    }

    pub fn compressor(threshold_db: f32, ratio: f32, makeup_db: f32) -> Self {
        Self::Compressor {
            threshold_db,
            ratio,
            makeup_db,
        }
    }

    pub fn reverb(delay_ms: f32, feedback: f32, wet: f32) -> Self {
        Self::Reverb {
            delay_ms,
            feedback,
            wet,
        }
    }

    pub fn process(
        &self,
        samples: &mut [f32],
        channels: usize,
        sample_rate: u32,
        state: &mut EffectState,
    ) {
        process_effect(self, state, samples, channels, sample_rate);
    }
}

fn db_to_lin(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Per-channel filter memory for LowPass / HighPass / delay.
#[derive(Debug, Clone, Default)]
pub struct EffectState {
    lp_z: Vec<f32>,
    hp_z: Vec<f32>,
    delay: Vec<f32>,
    delay_pos: usize,
}

impl EffectState {
    pub fn ensure_channels(&mut self, n: usize) {
        if self.lp_z.len() != n {
            self.lp_z = vec![0.0; n];
        }
        if self.hp_z.len() != n {
            self.hp_z = vec![0.0; n];
        }
    }

    pub fn ensure_delay(&mut self, len: usize) {
        if self.delay.len() != len {
            self.delay = vec![0.0; len.max(1)];
            self.delay_pos = 0;
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
            let a = 1.0 - (-2.0 * std::f32::consts::PI * fc / sr).exp();
            for frame in samples.chunks_mut(channels) {
                for (c, s) in frame.iter_mut().enumerate() {
                    let y = state.lp_z[c] + a * (*s - state.lp_z[c]);
                    state.lp_z[c] = y;
                    *s = y;
                }
            }
        }
        InsertEffect::HighPass { cutoff_hz } => {
            let sr = sample_rate.max(1) as f32;
            let fc = cutoff_hz.clamp(20.0, sr * 0.45);
            let a = 1.0 - (-2.0 * std::f32::consts::PI * fc / sr).exp();
            for frame in samples.chunks_mut(channels) {
                for (c, s) in frame.iter_mut().enumerate() {
                    let x = *s;
                    let lp = state.hp_z[c] + a * (x - state.hp_z[c]);
                    state.hp_z[c] = lp;
                    *s = x - lp;
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
                    let compressed = thresh * over.powf(1.0 / ratio);
                    let g = if abs > 1e-8 { compressed / abs } else { 1.0 };
                    *s *= g;
                }
                *s *= makeup;
            }
        }
        InsertEffect::Reverb {
            delay_ms,
            feedback,
            wet,
        } => {
            let sr = sample_rate.max(1) as f32;
            let delay_len = ((*delay_ms).max(1.0) * 0.001 * sr) as usize;
            let delay_len = delay_len.max(1).min(sample_rate as usize);
            state.ensure_delay(delay_len);
            let fb = feedback.clamp(0.0, 0.95);
            let wet = wet.clamp(0.0, 1.0);
            let dry = 1.0 - wet;
            for s in samples.iter_mut() {
                let delayed = state.delay[state.delay_pos];
                let input = *s + delayed * fb;
                state.delay[state.delay_pos] = input;
                state.delay_pos = (state.delay_pos + 1) % state.delay.len();
                *s = *s * dry + delayed * wet;
            }
        }
    }
}
