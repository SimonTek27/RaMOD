//! AHDSR envelope (S5.5).

/// Attack-Hold-Decay-Sustain-Release in seconds / level.
#[derive(Debug, Clone, Copy)]
pub struct Ahdsr {
    pub attack: f32,
    pub hold: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Default for Ahdsr {
    fn default() -> Self {
        Self {
            attack: 0.0,
            hold: 0.0,
            decay: 0.0,
            sustain: 1.0,
            release: 0.05,
        }
    }
}

impl Ahdsr {
    /// Linear envelope gain at `t` seconds from note-on. Not yet in release.
    pub fn gain_at(&self, t: f32) -> f32 {
        let t = t.max(0.0);
        if t < self.attack {
            if self.attack <= 1e-6 {
                return 1.0;
            }
            return t / self.attack;
        }
        let t = t - self.attack;
        if t < self.hold {
            return 1.0;
        }
        let t = t - self.hold;
        if t < self.decay {
            if self.decay <= 1e-6 {
                return self.sustain;
            }
            let x = t / self.decay;
            return 1.0 + (self.sustain - 1.0) * x;
        }
        self.sustain.clamp(0.0, 1.0)
    }

    /// Release segment: from current level down to 0 over `release` seconds.
    pub fn release_gain(&self, level_at_release: f32, t_since_release: f32) -> f32 {
        if self.release <= 1e-6 {
            return 0.0;
        }
        let x = (t_since_release / self.release).clamp(0.0, 1.0);
        level_at_release * (1.0 - x)
    }
}
