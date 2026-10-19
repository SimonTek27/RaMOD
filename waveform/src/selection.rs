//! Frame-based selection (S9.1).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// Inclusive start frame.
    pub start: usize,
    /// Exclusive end frame.
    pub end: usize,
}

impl Selection {
    pub fn region(start: usize, end: usize) -> Self {
        let end = end.max(start);
        Self { start, end }
    }

    pub fn all(frames: usize) -> Self {
        Self {
            start: 0,
            end: frames,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Sample index range in interleaved buffer.
    pub fn sample_range(&self, channels: u16) -> (usize, usize) {
        let ch = channels as usize;
        (self.start * ch, self.end * ch)
    }
}
