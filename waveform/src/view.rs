//! Waveform overview and viewport (S9.1).
//!
//! [`peak_overview`] downsamples PCM into a min/max envelope; [`Viewport`] maps
//! between frames and pixels for a zoomed, scrollable view. The mapping lives
//! here rather than in the UI so selection, zoom and painting all agree on the
//! same numbers.

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

/// Most frames the viewport will show at once is the whole buffer; the fewest is
/// this, so the view can never collapse to a single sample and lose its place.
pub const MIN_VISIBLE_FRAMES: f64 = 16.0;

/// A zoomed, scrollable window onto the buffer measured in frames.
///
/// `start` and `visible` are floating point because a zoomed view rarely aligns
/// to whole frames, and rounding both on every scroll would make the view drift.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    total: usize,
    start: f64,
    visible: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Viewport {
    /// A viewport fitted to `total` frames.
    pub fn new(total: usize) -> Self {
        let mut viewport = Self {
            total,
            start: 0.0,
            visible: total.max(1) as f64,
        };
        viewport.fit();
        viewport
    }

    /// Total number of frames in the buffer.
    pub fn total(&self) -> usize {
        self.total
    }

    /// First visible frame.
    pub fn start(&self) -> f64 {
        self.start
    }

    /// One past the last visible frame.
    pub fn end(&self) -> f64 {
        self.start + self.visible
    }

    /// Number of visible frames.
    pub fn visible_frames(&self) -> f64 {
        self.visible
    }

    /// Whether the whole buffer is visible.
    pub fn is_fitted(&self) -> bool {
        self.total == 0 || self.visible >= self.total as f64 - 1e-6
    }

    /// Frames per pixel across `width` pixels.
    pub fn frames_per_px(&self, width: f32) -> f64 {
        self.visible / width.max(1.0) as f64
    }

    /// Track a new buffer length, keeping the current zoom where it still fits.
    ///
    /// A time-stretch changes the frame count under the viewport; the view must
    /// stay valid afterwards instead of pointing past the end.
    pub fn set_total(&mut self, total: usize) {
        if total == self.total {
            return;
        }
        let was_fitted = self.is_fitted();
        self.total = total;
        if was_fitted || total == 0 {
            self.fit();
        } else {
            self.visible = self.visible.min(total.max(1) as f64);
            self.clamp();
        }
    }

    /// Show the whole buffer.
    pub fn fit(&mut self) {
        self.start = 0.0;
        self.visible = self.total.max(1) as f64;
    }

    /// Zoom so the selection fills the view.
    pub fn zoom_to_selection(&mut self, start_frame: usize, end_frame: usize) {
        if end_frame <= start_frame {
            return;
        }
        self.start = start_frame as f64;
        self.visible = (end_frame - start_frame) as f64;
        self.clamp();
    }

    /// Centre the view on `frame`, keeping the zoom.
    pub fn center_on(&mut self, frame: f64) {
        self.start = frame - self.visible / 2.0;
        self.clamp();
    }

    /// Zoom by `factor` (`< 1` zooms in) around the centre of the view.
    pub fn zoom(&mut self, factor: f32) {
        let anchor = self.start + self.visible / 2.0;
        self.zoom_at(anchor, factor);
    }

    /// Zoom by `factor` (`< 1` zooms in) keeping `anchor_frame` under the cursor.
    ///
    /// This is the behaviour that makes wheel zoom feel correct: the frame the
    /// pointer is over does not move, so zooming never loses the edit point.
    pub fn zoom_at(&mut self, anchor_frame: f64, factor: f32) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let max_visible = self.total.max(1) as f64;
        let lower = MIN_VISIBLE_FRAMES.min(max_visible);
        let new_visible = (self.visible * factor as f64).clamp(lower, max_visible);
        // Keep the anchor at the same fraction through the view.
        let frac = if self.visible > 0.0 {
            (anchor_frame - self.start) / self.visible
        } else {
            0.5
        };
        self.start = anchor_frame - frac * new_visible;
        self.visible = new_visible;
        self.clamp();
    }

    /// Scroll by a number of frames (positive moves the view later).
    pub fn pan_frames(&mut self, delta: f64) {
        self.start += delta;
        self.clamp();
    }

    /// Scroll by a number of pixels.
    pub fn pan_px(&mut self, delta_px: f32, width: f32) {
        self.pan_frames(delta_px as f64 * self.frames_per_px(width));
    }

    /// Clamp the window inside the buffer.
    pub fn clamp(&mut self) {
        let total = self.total as f64;
        if self.total == 0 {
            self.start = 0.0;
            self.visible = 1.0;
            return;
        }
        self.visible = self
            .visible
            .clamp(MIN_VISIBLE_FRAMES.min(total), total.max(1.0));
        let max_start = (total - self.visible).max(0.0);
        self.start = self.start.clamp(0.0, max_start);
    }

    /// Fractional frame under `px` across a `width`-pixel view.
    pub fn frame_at_f(&self, px: f32, width: f32) -> f64 {
        let width = width.max(1.0) as f64;
        self.start + (px as f64 / width).clamp(0.0, 1.0) * self.visible
    }

    /// Frame under `px`, clamped into the buffer.
    pub fn frame_at(&self, px: f32, width: f32) -> usize {
        let frame = self.frame_at_f(px, width).round();
        frame.clamp(0.0, self.total as f64) as usize
    }

    /// Pixel offset of `frame` across a `width`-pixel view.
    pub fn x_of(&self, frame: f64, width: f32) -> f32 {
        if self.visible <= 0.0 {
            return 0.0;
        }
        (((frame - self.start) / self.visible) * width.max(1.0) as f64) as f32
    }

    /// Visible frame range as whole frames, clipped to the buffer.
    pub fn visible_range(&self) -> (usize, usize) {
        let start = self.start.floor().max(0.0) as usize;
        let end = self.end().ceil().max(0.0) as usize;
        (start.min(self.total), end.min(self.total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: f32 = 800.0;

    #[test]
    fn new_viewport_is_fitted() {
        let vp = Viewport::new(48_000);
        assert!(vp.is_fitted());
        assert_eq!(vp.start(), 0.0);
        assert_eq!(vp.visible_frames(), 48_000.0);
        assert_eq!(vp.visible_range(), (0, 48_000));
    }

    #[test]
    fn frame_and_pixel_round_trip() {
        let mut vp = Viewport::new(48_000);
        vp.zoom_to_selection(10_000, 14_000);
        for frame in [10_000.0, 11_234.0, 12_000.0, 13_999.0] {
            let x = vp.x_of(frame, WIDTH);
            let back = vp.frame_at_f(x, WIDTH);
            assert!(
                (back - frame).abs() < 1.0,
                "frame {frame} -> x {x} -> {back}"
            );
        }
    }

    #[test]
    fn zoom_at_keeps_the_frame_under_the_cursor() {
        let mut vp = Viewport::new(100_000);
        let px = WIDTH * 0.7;
        let before = vp.frame_at_f(px, WIDTH);
        vp.zoom_at(before, 0.25); // zoom in 4x
        let after = vp.frame_at_f(px, WIDTH);
        assert!(
            (after - before).abs() < 2.0,
            "anchor moved from {before} to {after}"
        );
        // And the view really did zoom in.
        assert!(vp.visible_frames() < 100_000.0 * 0.3);
    }

    #[test]
    fn zoom_is_bounded_by_the_buffer_and_a_floor() {
        let mut vp = Viewport::new(1_000);
        for _ in 0..80 {
            vp.zoom(0.5);
        }
        assert!(vp.visible_frames() >= MIN_VISIBLE_FRAMES - 1e-9);
        for _ in 0..80 {
            vp.zoom(2.0);
        }
        assert_eq!(vp.visible_frames(), 1_000.0);
        assert!(vp.is_fitted());
        assert_eq!(vp.start(), 0.0);
    }

    #[test]
    fn panning_clamps_to_the_buffer_edges() {
        let mut vp = Viewport::new(10_000);
        vp.zoom_to_selection(4_000, 6_000); // 2 000 frames visible
        vp.pan_frames(-50_000.0);
        assert_eq!(vp.start(), 0.0);
        vp.pan_frames(50_000.0);
        assert_eq!(vp.start(), 8_000.0);
        assert_eq!(vp.end(), 10_000.0);
    }

    #[test]
    fn zoom_to_selection_frames_the_edit() {
        let mut vp = Viewport::new(96_000);
        vp.zoom_to_selection(24_000, 30_000);
        assert_eq!(vp.start(), 24_000.0);
        assert_eq!(vp.visible_frames(), 6_000.0);
        assert!(!vp.is_fitted());
    }

    #[test]
    fn tracking_a_new_total_keeps_the_zoom_valid() {
        // Fitted view follows the buffer (a stretch from 12k to 18k frames).
        let mut vp = Viewport::new(12_000);
        vp.set_total(18_000);
        assert!(vp.is_fitted());
        assert_eq!(vp.visible_frames(), 18_000.0);

        // A zoomed view is clamped inside the new length instead of overflowing.
        let mut vp = Viewport::new(12_000);
        vp.zoom_to_selection(10_000, 12_000);
        vp.set_total(6_000);
        assert!(vp.end() <= 6_000.0 + 1e-6);
        assert_eq!(vp.start(), 4_000.0);
    }

    #[test]
    fn empty_buffer_is_safe() {
        let mut vp = Viewport::new(0);
        assert_eq!(vp.frame_at(10.0, WIDTH), 0);
        assert_eq!(vp.visible_range(), (0, 0));
        vp.zoom(0.5);
        vp.pan_frames(100.0);
        assert_eq!(vp.start(), 0.0);
        assert_eq!(vp.x_of(0.0, WIDTH), 0.0);
    }
}
