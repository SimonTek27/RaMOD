//! Parameter automation curves (S5.3).

/// One keyframe on a curve (time in seconds, value).
#[derive(Debug, Clone, Copy)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
}

/// Piecewise-linear automation curve.
#[derive(Debug, Clone, Default)]
pub struct AutomationCurve {
    pub parameter: String,
    pub keys: Vec<Keyframe>,
}

impl AutomationCurve {
    pub fn new(parameter: impl Into<String>) -> Self {
        Self {
            parameter: parameter.into(),
            keys: Vec::new(),
        }
    }

    pub fn add_key(&mut self, time: f32, value: f32) {
        self.keys.push(Keyframe { time, value });
        self.keys
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
    }

    /// Sample the curve at time `t` (holds endpoints).
    pub fn eval(&self, t: f32) -> Option<f32> {
        if self.keys.is_empty() {
            return None;
        }
        if t <= self.keys[0].time {
            return Some(self.keys[0].value);
        }
        let last = self.keys.last().unwrap();
        if t >= last.time {
            return Some(last.value);
        }
        for w in self.keys.windows(2) {
            let a = w[0];
            let b = w[1];
            if t >= a.time && t <= b.time {
                let span = (b.time - a.time).max(1e-6);
                let x = (t - a.time) / span;
                return Some(a.value + (b.value - a.value) * x);
            }
        }
        Some(last.value)
    }
}
