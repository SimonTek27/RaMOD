//! # ramod_waveform — GoldWave-style PCM editing core (S9)
//!
//! Pure-Rust friendly buffer with selection, effects, and undo/redo.
//! Import: WAV / MP3 / OGG Vorbis / FLAC (Symphonia).

mod effects;
mod export;
mod history;
mod import;
mod selection;
mod view;

pub use effects::{
    compress, db_to_linear, eq, fade, gain, limiter, linear_to_db, normalize, pitch_shift,
    resample, reverse, silence, time_stretch, CompressorParams, EqBand,
};
pub use export::{write_wav_i16, WavError};
pub use history::{EditCommand, UndoStack};
pub use import::{is_supported_extension, read_audio, read_wav, ImportError};
pub use selection::Selection;
pub use view::{peak_overview, PeakMinMax, Viewport};

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

    /// Load WAV / MP3 / OGG / FLAC.
    pub fn from_file(path: &std::path::Path) -> Result<Self, ImportError> {
        let mut w = read_audio(path)?;
        w.select_all();
        Ok(w)
    }

    /// Alias for [`from_file`].
    pub fn from_wav(path: &std::path::Path) -> Result<Self, ImportError> {
        Self::from_file(path)
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

    /// Record the whole buffer before a structural edit (cut / paste / delete).
    fn push_undo(&mut self) {
        self.undo.push(EditCommand::ReplaceAll {
            samples: self.samples.clone(),
            selection: self.selection,
        });
    }

    /// Record only the affected range before an in-place effect.
    fn push_region_undo(&mut self) {
        let (a, b) = self.selection.sample_range(self.channels);
        let b = b.min(self.samples.len());
        let a = a.min(b);
        self.undo.push(EditCommand::ReplaceRegion {
            start: a,
            end: b,
            samples: self.samples[a..b].to_vec(),
            selection: self.selection,
        });
    }

    fn clamp_selection(&mut self) {
        let f = self.frames();
        self.selection.end = self.selection.end.min(f);
        if self.selection.start > self.selection.end {
            self.selection.start = self.selection.end;
        }
    }

    /// Apply `cmd` to the buffer and return the command that undoes it.
    fn apply_command(&mut self, cmd: EditCommand) -> EditCommand {
        match cmd {
            EditCommand::ReplaceAll { samples, selection } => {
                let prev_samples = std::mem::replace(&mut self.samples, samples);
                let prev_selection = std::mem::replace(&mut self.selection, selection);
                EditCommand::ReplaceAll {
                    samples: prev_samples,
                    selection: prev_selection,
                }
            }
            EditCommand::ReplaceRegion {
                start,
                end,
                samples,
                selection,
            } => {
                let end = end.min(self.samples.len());
                let start = start.min(end);
                let prev_samples: Vec<f32> = self.samples[start..end].to_vec();
                for (i, s) in samples.iter().enumerate() {
                    if let Some(slot) = self.samples.get_mut(start + i) {
                        *slot = *s;
                    }
                }
                let prev_selection = std::mem::replace(&mut self.selection, selection);
                EditCommand::ReplaceRegion {
                    start,
                    end: start + samples.len(),
                    samples: prev_samples,
                    selection: prev_selection,
                }
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.undo.can_redo()
    }

    pub fn undo(&mut self) -> bool {
        let Some(cmd) = self.undo.pop() else {
            return false;
        };
        let inverse = self.apply_command(cmd);
        self.undo.record_redo(inverse);
        self.clamp_selection();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(cmd) = self.undo.pop_redo() else {
            return false;
        };
        let inverse = self.apply_command(cmd);
        self.undo.record_undo(inverse);
        self.clamp_selection();
        true
    }

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

    pub fn paste(&mut self, clipboard: &[f32]) {
        if clipboard.is_empty() {
            return;
        }
        let ch = self.channels as usize;
        let len = clipboard.len() - (clipboard.len() % ch);
        if len == 0 {
            return;
        }
        self.push_undo();
        let (a, b) = self.selection.sample_range(self.channels);
        self.samples.splice(a..b, clipboard[..len].iter().copied());
        let inserted_frames = len / ch;
        self.selection =
            Selection::region(self.selection.start, self.selection.start + inserted_frames);
    }

    pub fn apply_silence(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.push_region_undo();
        silence(&mut self.samples, self.channels, self.selection);
    }

    pub fn apply_reverse(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.push_region_undo();
        reverse(&mut self.samples, self.channels, self.selection);
    }

    pub fn apply_gain(&mut self, linear: f32) {
        if self.selection.is_empty() {
            return;
        }
        self.push_region_undo();
        gain(&mut self.samples, self.channels, self.selection, linear);
    }

    pub fn apply_normalize(&mut self, target_peak: f32) {
        if self.selection.is_empty() {
            return;
        }
        self.push_region_undo();
        normalize(
            &mut self.samples,
            self.channels,
            self.selection,
            target_peak,
        );
    }

    pub fn apply_fade_in(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.push_region_undo();
        fade(&mut self.samples, self.channels, self.selection, true);
    }

    pub fn apply_fade_out(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.push_region_undo();
        fade(&mut self.samples, self.channels, self.selection, false);
    }

    /// Apply peaking EQ bands to the selection; undo restores only that region.
    pub fn apply_eq(&mut self, bands: &[EqBand]) -> bool {
        if self.selection.is_empty() || bands.is_empty() {
            return false;
        }
        let (a, b) = self.region_range();
        if a >= b {
            return false;
        }
        self.push_region_undo();
        eq(
            &mut self.samples[a..b],
            self.channels,
            bands,
            self.sample_rate,
        );
        true
    }

    /// Compress the selection (stereo-linked, so the image does not shift).
    pub fn apply_compress(&mut self, params: CompressorParams) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let (a, b) = self.region_range();
        if a >= b {
            return false;
        }
        self.push_region_undo();
        compress(
            &mut self.samples[a..b],
            self.channels,
            params,
            self.sample_rate,
        );
        true
    }

    /// Limit the selection to a ceiling in dB.
    pub fn apply_limit(&mut self, ceiling_db: f32) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let (a, b) = self.region_range();
        if a >= b {
            return false;
        }
        self.push_region_undo();
        limiter(
            &mut self.samples[a..b],
            self.channels,
            ceiling_db,
            self.sample_rate,
        );
        true
    }

    /// Stretch the selection in time. `ratio > 1` is longer; pitch is unchanged.
    pub fn apply_time_stretch(&mut self, ratio: f32) -> bool {
        if self.selection.is_empty() || !ratio.is_finite() || ratio <= 0.0 {
            return false;
        }
        let (a, b) = self.region_range();
        if a >= b {
            return false;
        }
        let out = time_stretch(&self.samples[a..b], self.channels, ratio, self.sample_rate);
        self.replace_region(a, b, out)
    }

    /// Shift the selection's pitch by `semitones`, keeping its length.
    pub fn apply_pitch(&mut self, semitones: f32) -> bool {
        if self.selection.is_empty() || !semitones.is_finite() || semitones.abs() < 1e-3 {
            return false;
        }
        let (a, b) = self.region_range();
        if a >= b {
            return false;
        }
        let out = pitch_shift(
            &self.samples[a..b],
            self.channels,
            semitones,
            self.sample_rate,
        );
        self.replace_region(a, b, out)
    }

    /// Clamped sample range of the current selection.
    fn region_range(&self) -> (usize, usize) {
        let (a, b) = self.selection.sample_range(self.channels);
        let b = b.min(self.samples.len());
        (a.min(b), b)
    }

    /// Replace `[a, b)` with `out`, choosing region or full-buffer undo by length.
    fn replace_region(&mut self, a: usize, b: usize, out: Vec<f32>) -> bool {
        if out.is_empty() {
            return false;
        }
        let ch = (self.channels as usize).max(1);
        if out.len() == b - a {
            self.push_region_undo();
            self.samples[a..b].copy_from_slice(&out);
            return true;
        }
        self.push_undo();
        let inserted_frames = out.len() / ch;
        let start = a / ch;
        self.samples.splice(a..b, out.iter().copied());
        self.selection = Selection::region(start, start + inserted_frames);
        true
    }

    pub fn overview(&self, buckets: usize) -> Vec<PeakMinMax> {
        peak_overview(&self.samples, self.channels, buckets)
    }

    /// Peak envelope of `frames` frames starting at `start_frame`.
    ///
    /// A zoomed view must paint the visible span, not the whole file squeezed
    /// into the same pixels, or zooming in and out shows an identical picture.
    pub fn overview_range(
        &self,
        start_frame: usize,
        frames: usize,
        buckets: usize,
    ) -> Vec<PeakMinMax> {
        let ch = (self.channels as usize).max(1);
        let total = self.frames();
        let start = start_frame.min(total);
        let end = start.saturating_add(frames).min(total);
        peak_overview(&self.samples[start * ch..end * ch], self.channels, buckets)
    }

    /// Visible span as `(start_frame, frame_count)`, for painting.
    pub fn visible_span(&self, viewport: &Viewport) -> (usize, usize) {
        let (a, b) = viewport.visible_range();
        (a, b.saturating_sub(a))
    }

    pub fn export_wav(&self, path: &std::path::Path, selection_only: bool) -> Result<(), WavError> {
        let samples = if selection_only && !self.selection.is_empty() {
            let (a, b) = self.selection.sample_range(self.channels);
            &self.samples[a..b]
        } else {
            self.samples.as_slice()
        };
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
    fn wav_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("ramod_wav_test.wav");
        let mut w = mono_ramp();
        w.select_all();
        w.export_wav(&path, false).unwrap();
        let loaded = WaveBuffer::from_file(&path).unwrap();
        assert_eq!(loaded.frames(), 100);
        assert_eq!(loaded.channels, 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn undo_then_redo_restores_the_edit() {
        let mut w = mono_ramp();
        w.set_selection(10, 20);
        w.apply_silence();
        assert!(w.samples[10..20].iter().all(|s| *s == 0.0));
        assert!(w.can_undo());
        assert!(!w.can_redo());
        assert!(w.undo());
        assert!(w.samples[10..20].iter().any(|s| *s != 0.0));
        assert!(w.can_redo());
        assert!(w.redo());
        assert!(w.samples[10..20].iter().all(|s| *s == 0.0));
    }

    #[test]
    fn new_edit_discards_redo_history() {
        let mut w = mono_ramp();
        w.select_all();
        w.apply_gain(0.5);
        assert!(w.undo());
        assert!(w.can_redo());
        w.set_selection(0, 5);
        w.apply_silence();
        assert!(!w.can_redo(), "a new edit must invalidate redo");
    }

    #[test]
    fn undo_after_structural_edit_restores_length() {
        let mut w = mono_ramp();
        w.set_selection(0, 10);
        let clip = w.cut();
        assert_eq!(clip.len(), 10);
        assert_eq!(w.frames(), 90);
        assert!(w.undo());
        assert_eq!(w.frames(), 100);
        assert_eq!(w.samples[0], 0.0);
        assert!(w.redo());
        assert_eq!(w.frames(), 90);
    }

    #[test]
    fn region_undo_preserves_interleaved_channels() {
        let mut interleaved = Vec::with_capacity(200);
        for i in 0..100 {
            interleaved.push(i as f32);
            interleaved.push(-(i as f32));
        }
        let mut w = WaveBuffer::new(interleaved, 2, 48_000).unwrap();
        w.set_selection(10, 20);
        w.apply_reverse();
        assert_eq!(w.samples[20], 19.0);
        assert_eq!(w.samples[21], -19.0);
        assert!(w.undo());
        assert_eq!(w.samples[20], 10.0);
        assert_eq!(w.samples[21], -10.0);
    }

    #[test]
    fn undo_with_empty_history_is_false() {
        let mut w = mono_ramp();
        assert!(!w.undo());
        assert!(!w.redo());
        assert!(!w.can_redo());
    }

    fn sine(frames: usize, freq: f32, rate: u32, amp: f32) -> Vec<f32> {
        (0..frames)
            .map(|i| (std::f32::consts::TAU * freq * i as f32 / rate as f32).sin() * amp)
            .collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|s| s * s).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    fn zero_crossings(x: &[f32]) -> usize {
        x.windows(2)
            .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
            .count()
    }

    #[test]
    fn eq_boost_raises_the_band_and_undo_restores_it() {
        let mut w = WaveBuffer::new(sine(24_000, 1_000.0, 48_000, 0.5), 1, 48_000).unwrap();
        let before = rms(&w.samples);
        w.select_all();
        assert!(w.apply_eq(&[EqBand::new(1_000.0, 12.0, 1.0)]));
        let ratio = rms(&w.samples) / before;
        assert!(
            ratio > 2.5 && ratio < 5.0,
            "+12 dB should be about 4x, got {ratio}"
        );
        assert!(w.undo());
        assert!((rms(&w.samples) - before).abs() < 1e-4);
    }

    #[test]
    fn eq_cut_leaves_far_bands_alone() {
        let mut w = WaveBuffer::new(sine(24_000, 5_000.0, 48_000, 0.5), 1, 48_000).unwrap();
        let before = rms(&w.samples);
        w.select_all();
        assert!(w.apply_eq(&[EqBand::new(200.0, -18.0, 1.0)]));
        let after = rms(&w.samples);
        assert!(
            (after - before).abs() / before < 0.1,
            "5 kHz should be largely unaffected"
        );
    }

    #[test]
    fn compressor_reduces_loud_material() {
        let mut w = WaveBuffer::new(sine(24_000, 200.0, 48_000, 0.5), 1, 48_000).unwrap();
        w.select_all();
        assert!(w.apply_compress(CompressorParams {
            threshold_db: -20.0,
            ratio: 8.0,
            attack_ms: 1.0,
            release_ms: 40.0,
            makeup_db: 0.0,
        }));
        let tail = &w.samples[12_000..];
        let peak = tail.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak < 0.2, "expected heavy reduction below 0.2, got {peak}");
    }

    #[test]
    fn limiter_never_exceeds_the_ceiling() {
        let mut loud = sine(24_000, 300.0, 48_000, 3.0);
        loud[0] = 0.01;
        let mut w = WaveBuffer::new(loud, 1, 48_000).unwrap();
        w.select_all();
        assert!(w.apply_limit(-1.0));
        let peak = w.samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak <= 0.892, "peak {peak} exceeds the ceiling");
    }

    #[test]
    fn time_stretch_scales_duration_and_is_undoable() {
        let mut w = WaveBuffer::new(sine(12_000, 440.0, 48_000, 0.5), 1, 48_000).unwrap();
        w.select_all();
        assert!(w.apply_time_stretch(1.5));
        let frames = w.frames();
        assert!((frames as i64 - 18_000).abs() < 400, "frames {frames}");
        assert!(rms(&w.samples) > 0.1, "stretch must keep the signal");
        assert!(w.undo());
        assert_eq!(w.frames(), 12_000);
    }

    #[test]
    fn pitch_shift_keeps_length_and_raises_frequency() {
        let n = 24_000;
        let mut w = WaveBuffer::new(sine(n, 220.0, 48_000, 0.5), 1, 48_000).unwrap();
        let before = zero_crossings(&w.samples);
        w.select_all();
        assert!(w.apply_pitch(12.0));
        assert_eq!(w.frames(), n, "pitch must not change duration");
        let after = zero_crossings(&w.samples);
        assert!(
            after as f32 > before as f32 * 1.3,
            "octave up should cross zero more often ({before} -> {after})"
        );
        assert!(w.undo());
        assert_eq!(zero_crossings(&w.samples), before);
    }

    #[test]
    fn structural_effects_keep_stereo_interleaving() {
        let mut interleaved = Vec::with_capacity(24_000);
        for i in 0..12_000 {
            let v = (i as f32 * 0.01).sin() * 0.5;
            interleaved.push(v);
            interleaved.push(-v);
        }
        let mut w = WaveBuffer::new(interleaved, 2, 48_000).unwrap();
        w.select_all();
        assert!(w.apply_time_stretch(2.0));
        let mut checked = 0;
        for f in w.samples.chunks_exact(2) {
            assert!((f[0] + f[1]).abs() < 1e-3, "channels drifted: {f:?}");
            checked += 1;
        }
        assert!(checked > 20_000, "only {checked} frames checked");
    }

    #[test]
    fn zoomed_overview_shows_only_the_visible_span() {
        // First half silent, second half full-scale: the peaks must follow the
        // viewport, which is what makes zooming in the UI show anything at all.
        let mut samples = vec![0.0f32; 12_000];
        samples.extend(std::iter::repeat(0.75).take(12_000));
        let w = WaveBuffer::new(samples, 1, 48_000).unwrap();

        let quiet = w.overview_range(0, 12_000, 64);
        assert!(quiet.iter().all(|p| p.max == 0.0 && p.min == 0.0));

        let loud = w.overview_range(12_000, 12_000, 64);
        assert!(loud.iter().all(|p| (p.max - 0.75).abs() < 1e-6));

        // A span past the end is clipped rather than panicking.
        let clipped = w.overview_range(20_000, 50_000, 8);
        assert_eq!(clipped.len(), 8);
        assert!(clipped.iter().all(|p| (p.max - 0.75).abs() < 1e-6));
        assert!(w
            .overview_range(w.frames(), 1_000, 4)
            .iter()
            .all(|p| p.max == 0.0));
    }

    #[test]
    fn viewport_span_tracks_zoom_to_selection() {
        let mut w = mono_ramp();
        w.set_selection(40, 60);
        let mut vp = Viewport::new(w.frames());
        vp.zoom_to_selection(w.selection.start, w.selection.end);
        assert_eq!(w.visible_span(&vp), (40, 20));
        // Zooming back out covers the whole buffer again.
        vp.fit();
        assert_eq!(w.visible_span(&vp), (0, 100));
    }
}
