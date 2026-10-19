//! # ramod_waveform — GoldWave-style PCM editing core (S9)
//!
//! Pure-Rust, non-destructive-friendly buffer with selection, effects, undo.

mod effects;
mod export;
mod history;
mod selection;
mod view;

pub use effects::{fade, gain, normalize, reverse, silence};
pub use export::{write_wav_i16, WavError};
pub use history::{EditCommand, UndoStack};
pub use selection::Selection;
pub use view::{peak_overview, PeakMinMax};

/// Interleaved float PCM buffer (−1.0 … 1.0).
#[derive(Debug, Clone)]
pub struct WaveBuffer {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
    pub selection: Selection,
    undo: UndoStack,
}

impl WaveBuffer {
    pub fn new(samples: Vec<f32>, channels: u16, sample_rate: u32) -> Result<Self, WaveError> {
        if channels == 0 || sample_rate == 0 {
            return Err(WaveError::InvalidFormat);
        }
        if samples.len() % channels as usize != 0 {
            return Err(WaveError::InvalidFormat);
        }
        Ok(Self {
            samples,
            channels,
            sample_rate,
            selection: Selection::all(0),
            undo: UndoStack::new(64),
        })
    }

    pub fn silent(frames: usize, channels: u16, sample_rate: u32) -> Result<Self, WaveError> {
        let n = frames.saturating_mul(channels as usize);
        Self::new(vec![0.0; n], channels, sample_rate)
    }

    pub fn frames(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }

    pub fn duration_secs(&self) -> f32 {
        self.frames() as f32 / self.sample_rate.max(1) as f32
    }

    pub fn select_all(&mut self) {
        self.selection = Selection::region(0, self.frames());
    }

    pub fn set_selection(&mut self, start_frame: usize, end_frame: usize) {
        let end = end_frame.min(self.frames());
        let start = start_frame.min(end);
        self.selection = Selection::region(start, end);
    }

    fn push_undo(&mut self) {
        self.undo.push(EditCommand::ReplaceAll {
            samples: self.samples.clone(),
        });
    }

    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    pub fn undo(&mut self) -> bool {
        if let Some(EditCommand::ReplaceAll { samples }) = self.undo.pop() {
            self.samples = samples;
            let f = self.frames();
            if self.selection.end > f {
                self.selection.end = f;
            }
            if self.selection.start > self.selection.end {
                self.selection.start = self.selection.end;
            }
            true
        } else {
            false
        }
    }

    /// Cut selection to clipboard; removes from buffer.
    pub fn cut(&mut self) -> Vec<f32> {
        let clip = self.copy();
        self.delete_selection();
        clip
    }

    pub fn copy(&self) -> Vec<f32> {
        let (a, b) = self.selection.sample_range(self.channels);
        self.samples[a..b].to_vec()
    }

    pub fn delete_selection(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.push_undo();
        let (a, b) = self.selection.sample_range(self.channels);
        self.samples.drain(a..b);
        self.selection = Selection::region(self.selection.start, self.selection.start);
    }

    /// Insert clipboard samples at selection start (replaces selection if non-empty).
    pub fn paste(&mut self, clipboard: &[f32]) {
        if clipboard.is_empty() {
            return;
        }
        // Align to channel multiple
        let ch = self.channels as usize;
        let len = clipboard.len() - (clipboard.len() % ch);
        if len == 0 {
            return;
        }
        self.push_undo();
        let (a, b) = self.selection.sample_range(self.channels);
        self.samples.splice(a..b, clipboard[..len].iter().copied());
        let inserted_frames = len / ch;
        self.selection = Selection::region(self.selection.start, self.selection.start + inserted_frames);
    }

    pub fn apply_silence(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.push_undo();
        silence(&mut self.samples, self.channels, self.selection);
    }

    pub fn apply_reverse(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.push_undo();
        reverse(&mut self.samples, self.channels, self.selection);
    }

    pub fn apply_gain(&mut self, linear: f32) {
        if self.selection.is_empty() {
            return;
        }
        self.push_undo();
        gain(&mut self.samples, self.channels, self.selection, linear);
    }

    pub fn apply_normalize(&mut self, target_peak: f32) {
        if self.selection.is_empty() {
            return;
        }
        self.push_undo();
        normalize(&mut self.samples, self.channels, self.selection, target_peak);
    }

    pub fn apply_fade_in(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.push_undo();
        fade(&mut self.samples, self.channels, self.selection, true);
    }

    pub fn apply_fade_out(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.push_undo();
        fade(&mut self.samples, self.channels, self.selection, false);
    }

    /// Peak overview for waveform UI (S9.1).
    pub fn overview(&self, buckets: usize) -> Vec<PeakMinMax> {
        peak_overview(&self.samples, self.channels, buckets)
    }

    /// Export selection or entire buffer as WAV i16 (S9.4).
    pub fn export_wav(&self, path: &std::path::Path, selection_only: bool) -> Result<(), WavError> {
        let (samples, frames) = if selection_only && !self.selection.is_empty() {
            let (a, b) = self.selection.sample_range(self.channels);
            (&self.samples[a..b], self.selection.len())
        } else {
            (self.samples.as_slice(), self.frames())
        };
        let _ = frames;
        write_wav_i16(path, samples, self.channels, self.sample_rate)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaveError {
    InvalidFormat,
}

impl std::fmt::Display for WaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaveError::InvalidFormat => write!(f, "invalid wave format"),
        }
    }
}

impl std::error::Error for WaveError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn mono_ramp() -> WaveBuffer {
        let s: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        WaveBuffer::new(s, 1, 48_000).unwrap()
    }

    #[test]
    fn cut_paste_roundtrip() {
        let mut w = mono_ramp();
        w.set_selection(10, 20);
        let clip = w.cut();
        assert_eq!(clip.len(), 10);
        assert_eq!(w.frames(), 90);
        w.set_selection(0, 0);
        w.paste(&clip);
        assert_eq!(w.frames(), 100);
    }

    #[test]
    fn reverse_selection() {
        let mut w = mono_ramp();
        w.set_selection(0, 4);
        w.apply_reverse();
        assert!((w.samples[0] - 0.03).abs() < 0.001);
        assert!((w.samples[3] - 0.0).abs() < 0.001);
    }

    #[test]
    fn undo_restores() {
        let mut w = mono_ramp();
        let before = w.samples.clone();
        w.select_all();
        w.apply_gain(0.5);
        assert!((w.samples[50] - before[50] * 0.5).abs() < 1e-5);
        assert!(w.undo());
        assert!((w.samples[50] - before[50]).abs() < 1e-5);
    }

    #[test]
    fn overview_buckets() {
        let w = mono_ramp();
        let o = w.overview(10);
        assert_eq!(o.len(), 10);
    }
}
