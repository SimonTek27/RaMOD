//! Master bus.

/// Simple master bus with linear gain.
#[derive(Debug, Clone)]
pub struct MasterBus {
    gain: f32,
}

impl MasterBus {
    pub fn new() -> Self {
        Self { gain: 1.0 }
    }

    pub fn gain(&self) -> f32 {
        self.gain
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.max(0.0);
    }
}

impl Default for MasterBus {
    fn default() -> Self {
        Self::new()
    }
}
