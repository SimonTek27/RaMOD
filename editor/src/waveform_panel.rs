//! egui Waveform panel — multi-format import, zoom/pan viewport, effects, preview.
//!
//! The frame↔pixel mapping lives in [`ramod_waveform::Viewport`] so the painted
//! envelope, the selection drag and the zoom controls all use the same numbers.

use egui::{self, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use ramod_waveform::{CompressorParams, EqBand, PeakMinMax, Viewport, WaveBuffer};
use std::path::{Path, PathBuf};

pub struct WaveformPanel {
    pub wave: Option<WaveBuffer>,
    pub clipboard: Vec<f32>,
    pub show: bool,
    pub status: String,
    /// Fractional frame where the current selection drag started.
    drag_start: Option<f64>,
    viewport: Viewport,
    export_path: String,
    import_path: String,
    gain_db: f32,
    eq_freq_hz: f32,
    eq_gain_db: f32,
    pitch_semitones: f32,
    stretch_ratio: f32,
    pub last_import_path: Option<PathBuf>,
}

impl Default for WaveformPanel {
    fn default() -> Self {
        Self {
            wave: None,
            clipboard: Vec::new(),
            show: true,
            status: "Load or generate a waveform".into(),
            drag_start: None,
            viewport: Viewport::default(),
            export_path: "export.wav".into(),
            import_path: String::new(),
            gain_db: 0.0,
            eq_freq_hz: 1_000.0,
            eq_gain_db: 0.0,
            pitch_semitones: 0.0,
            stretch_ratio: 1.0,
            last_import_path: None,
        }
    }
}

impl WaveformPanel {
    pub fn load_demo_tone(&mut self) {
        let rate = 48_000u32;
        let frames = rate as usize / 2;
        let samples: Vec<f32> = (0..frames)
            .map(|i| {
                let t = i as f32 / rate as f32;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.4
            })
            .collect();
        match WaveBuffer::new(samples, 1, rate) {
            Ok(mut w) => {
                w.select_all();
                self.viewport = Viewport::new(w.frames());
                self.wave = Some(w);
                self.last_import_path = None;
                self.status = "Demo tone 440 Hz (0.5 s)".into();
            }
            Err(_) => self.status = "Failed to create demo tone".into(),
        }
    }

    pub fn import_wav(&mut self, path: &Path) -> bool {
        match WaveBuffer::from_file(path) {
            Ok(w) => {
                self.status = format!(
                    "Imported {} ({} ch, {} Hz, {:.2}s)",
                    path.display(),
                    w.channels,
                    w.sample_rate,
                    w.duration_secs()
                );
                self.import_path = path.to_string_lossy().into_owned();
                self.last_import_path = Some(path.to_path_buf());
                self.viewport = Viewport::new(w.frames());
                self.wave = Some(w);
                true
            }
            Err(e) => {
                self.status = format!("Import failed: {e}");
                false
            }
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> WaveformAction {
        let mut action = WaveformAction::None;

        ui.horizontal(|ui| {
            ui.heading("Waveform");
            if ui.button("Demo tone").clicked() {
                self.load_demo_tone();
            }
            if ui.button("Select all").clicked() {
                if let Some(w) = self.wave.as_mut() {
                    w.select_all();
                }
            }
            if ui
                .button("▶ Preview")
                .on_hover_text("Play selection (or full)")
                .clicked()
            {
                if let Some(w) = self.wave.as_ref() {
                    let (samples, ch, rate) = if !w.selection.is_empty() {
                        let (a, b) = w.selection.sample_range(w.channels);
                        (w.samples[a..b].to_vec(), w.channels, w.sample_rate)
                    } else {
                        (w.samples.clone(), w.channels, w.sample_rate)
                    };
                    action = WaveformAction::PlayPreview {
                        samples,
                        channels: ch,
                        sample_rate: rate,
                    };
                }
            }
            if ui.button("■").on_hover_text("Stop preview").clicked() {
                action = WaveformAction::StopPreview;
            }
            ui.separator();
            self.toolbar(ui);
        });

        ui.horizontal(|ui| {
            self.effects_row(ui);
        });

        ui.horizontal(|ui| {
            ui.label("Import audio");
            ui.add(
                egui::TextEdit::singleline(&mut self.import_path)
                    .desired_width(280.0)
                    .hint_text("path/to/file.wav|.mp3|.ogg"),
            );
            if ui.button("Load").clicked() {
                let p = PathBuf::from(self.import_path.trim());
                if self.import_wav(&p) {
                    action = WaveformAction::Imported(p);
                }
            }
            if ui.button("Load + add to Assets").clicked() {
                let p = PathBuf::from(self.import_path.trim());
                if self.import_wav(&p) {
                    action = WaveformAction::ImportedAndRegister(p);
                }
            }
        });

        ui.separator();

        self.canvas(ui);

        ui.horizontal(|ui| {
            if let Some(w) = self.wave.as_ref() {
                let (a, b) = self.viewport.visible_range();
                ui.weak(format!(
                    "frames={}  sr={}  ch={}  sel=[{}..{})  view=[{}..{})  {:.3}s",
                    w.frames(),
                    w.sample_rate,
                    w.channels,
                    w.selection.start,
                    w.selection.end,
                    a,
                    b,
                    w.duration_secs()
                ));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(&self.status);
            });
        });

        ui.horizontal(|ui| {
            ui.label("Export");
            ui.add(
                egui::TextEdit::singleline(&mut self.export_path)
                    .desired_width(180.0)
                    .hint_text("export.wav"),
            );
            if ui.button("WAV full").clicked() {
                self.export(false);
            }
            if ui.button("WAV selection").clicked() {
                self.export(true);
            }
        });

        action
    }

    /// Painted waveform with wheel pan, ctrl+wheel zoom and drag selection.
    fn canvas(&mut self, ui: &mut Ui) {
        let height = 160.0;
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), height),
            Sense::click_and_drag(),
        );
        let rect = response.rect;
        let width = rect.width().max(1.0);

        // A time-stretch or a cut changes the frame count under the viewport.
        let frames = self.wave.as_ref().map(|w| w.frames()).unwrap_or(0);
        self.viewport.set_total(frames);

        if response.hovered() {
            let (scroll, modifiers) = ui.input(|i| (i.raw_scroll_delta, i.modifiers));
            if scroll.y.abs() > 0.0 {
                let anchor_px = response
                    .hover_pos()
                    .map(|p| (p.x - rect.left()).clamp(0.0, width))
                    .unwrap_or(width * 0.5);
                if modifiers.ctrl || modifiers.command {
                    let anchor = self.viewport.frame_at_f(anchor_px, width);
                    let factor = if scroll.y > 0.0 { 0.8 } else { 1.25 };
                    self.viewport.zoom_at(anchor, factor);
                } else {
                    let per_px = self.viewport.frames_per_px(width);
                    self.viewport.pan_frames(-scroll.y as f64 * per_px * 3.0);
                }
            }
        }

        let vp = self.viewport;
        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.drag_start =
                    Some(vp.frame_at_f((pos.x - rect.left()).clamp(0.0, width), width));
            }
        }
        if let (Some(start), Some(pos)) = (self.drag_start, response.interact_pointer_pos()) {
            let end = vp.frame_at_f((pos.x - rect.left()).clamp(0.0, width), width);
            if let Some(w) = self.wave.as_mut() {
                let a = start.min(end).floor().max(0.0) as usize;
                let b = start.max(end).ceil().max(a as f64 + 1.0) as usize;
                w.set_selection(a, b);
            }
        }
        if response.drag_stopped() {
            self.drag_start = None;
        }

        painter.rect_filled(rect, 2.0, Color32::from_rgb(22, 22, 26));
        let border = Stroke::new(1.0_f32, Color32::from_rgb(50, 50, 55));
        painter.line_segment([rect.left_top(), rect.right_top()], border);
        painter.line_segment([rect.right_top(), rect.right_bottom()], border);
        painter.line_segment([rect.right_bottom(), rect.left_bottom()], border);
        painter.line_segment([rect.left_bottom(), rect.left_top()], border);

        let mid_y = rect.center().y;
        painter.line_segment(
            [
                Pos2::new(rect.left(), mid_y),
                Pos2::new(rect.right(), mid_y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(40, 40, 48)),
        );

        match self.wave.as_ref() {
            Some(w) => {
                // Second markers, so a zoomed view has a readable time scale.
                let rate = w.sample_rate.max(1) as f64;
                let secs_per_px = vp.frames_per_px(width) / rate;
                if secs_per_px > 0.0 {
                    let step = nice_step(secs_per_px * 90.0);
                    let mut t = (vp.start() / rate / step).floor() * step;
                    let last = vp.end() / rate;
                    let tick = Stroke::new(1.0_f32, Color32::from_rgb(38, 38, 46));
                    while t <= last {
                        let frame = t * rate;
                        if frame >= vp.start() {
                            let x = rect.left() + vp.x_of(frame, width);
                            painter.line_segment(
                                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                                tick,
                            );
                        }
                        t += step;
                    }
                }

                let (a, b) = vp.visible_range();
                let span = b.saturating_sub(a);
                if span > 0 {
                    let buckets = (width as usize).clamp(32, 4096);
                    let overview = w.overview_range(a, span, buckets);
                    Self::paint_overview(&painter, rect, &overview, mid_y);
                }
                Self::paint_selection(&painter, rect, w, &vp, width);
            }
            None => {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No waveform — Import or Demo tone",
                    egui::FontId::proportional(14.0),
                    Color32::from_rgb(120, 120, 130),
                );
            }
        }

        // Zoom controls sit directly under the canvas they affect.
        ui.horizontal(|ui| {
            let enabled = self.wave.is_some();
            if ui.add_enabled(enabled, egui::Button::new("−")).clicked() {
                self.viewport.zoom(1.5);
            }
            if ui.add_enabled(enabled, egui::Button::new("+")).clicked() {
                self.viewport.zoom(0.6667);
            }
            if ui.add_enabled(enabled, egui::Button::new("Fit")).clicked() {
                self.viewport.fit();
            }
            if ui
                .add_enabled(enabled, egui::Button::new("Zoom sel"))
                .clicked()
            {
                if let Some(w) = self.wave.as_ref() {
                    if !w.selection.is_empty() {
                        self.viewport
                            .zoom_to_selection(w.selection.start, w.selection.end);
                    }
                }
            }
            let visible = self.viewport.visible_frames();
            let total = self.viewport.total().max(1) as f64;
            ui.weak(format!(
                "zoom ×{:.1}  ({} frames visible, ctrl+wheel to zoom, wheel to pan)",
                total / visible.max(1.0),
                visible.round() as usize
            ));
        });
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        if self.wave.is_none() {
            return;
        }
        if ui.button("Cut").clicked() {
            if let Some(w) = self.wave.as_mut() {
                self.clipboard = w.cut();
            }
        }
        if ui.button("Copy").clicked() {
            if let Some(w) = self.wave.as_ref() {
                self.clipboard = w.copy();
            }
        }
        if ui.button("Paste").clicked() {
            let clip = self.clipboard.clone();
            if let Some(w) = self.wave.as_mut() {
                w.paste(&clip);
            }
        }
        if ui.button("Silence").clicked() {
            if let Some(w) = self.wave.as_mut() {
                w.apply_silence();
            }
        }
        if ui.button("Reverse").clicked() {
            if let Some(w) = self.wave.as_mut() {
                w.apply_reverse();
            }
        }
        if ui.button("Fade in").clicked() {
            if let Some(w) = self.wave.as_mut() {
                w.apply_fade_in();
            }
        }
        if ui.button("Fade out").clicked() {
            if let Some(w) = self.wave.as_mut() {
                w.apply_fade_out();
            }
        }
        if ui.button("Normalize").clicked() {
            if let Some(w) = self.wave.as_mut() {
                w.apply_normalize(0.95);
            }
        }
        ui.label("Gain");
        ui.add(egui::Slider::new(&mut self.gain_db, -24.0..=12.0).suffix(" dB"));
        if ui.button("Apply gain").clicked() {
            let g = 10f32.powf(self.gain_db / 20.0);
            if let Some(w) = self.wave.as_mut() {
                w.apply_gain(g);
            }
        }
        let can_undo = self.wave.as_ref().is_some_and(|w| w.can_undo());
        let can_redo = self.wave.as_ref().is_some_and(|w| w.can_redo());
        if ui
            .add_enabled(can_undo, egui::Button::new("Undo"))
            .clicked()
        {
            if let Some(w) = self.wave.as_mut() {
                let _ = w.undo();
            }
        }
        if ui
            .add_enabled(can_redo, egui::Button::new("Redo"))
            .clicked()
        {
            if let Some(w) = self.wave.as_mut() {
                let _ = w.redo();
            }
        }
    }

    /// EQ, dynamics, pitch and time-stretch over the selection.
    fn effects_row(&mut self, ui: &mut Ui) {
        if self.wave.is_none() {
            return;
        }
        let empty_selection = self.wave.as_ref().is_some_and(|w| w.selection.is_empty());
        let enabled = !empty_selection;

        ui.label("EQ");
        ui.add(
            egui::Slider::new(&mut self.eq_freq_hz, 40.0..=12_000.0)
                .logarithmic(true)
                .suffix(" Hz"),
        );
        ui.add(egui::Slider::new(&mut self.eq_gain_db, -18.0..=18.0).suffix(" dB"));
        if ui
            .add_enabled(enabled, egui::Button::new("Apply EQ"))
            .on_hover_text("Peaking band, Q 1.0")
            .clicked()
        {
            if let Some(w) = self.wave.as_mut() {
                w.apply_eq(&[EqBand::new(self.eq_freq_hz, self.eq_gain_db, 1.0)]);
            }
        }

        ui.separator();
        if ui
            .add_enabled(enabled, egui::Button::new("Compress"))
            .on_hover_text("4:1 above −18 dB, stereo linked")
            .clicked()
        {
            if let Some(w) = self.wave.as_mut() {
                w.apply_compress(CompressorParams::default());
            }
        }
        if ui
            .add_enabled(enabled, egui::Button::new("Limit −1 dB"))
            .clicked()
        {
            if let Some(w) = self.wave.as_mut() {
                w.apply_limit(-1.0);
            }
        }

        ui.separator();
        ui.add(
            egui::Slider::new(&mut self.pitch_semitones, -12.0..=12.0)
                .suffix(" st")
                .text("pitch"),
        );
        if ui
            .add_enabled(enabled, egui::Button::new("Shift"))
            .clicked()
        {
            if let Some(w) = self.wave.as_mut() {
                w.apply_pitch(self.pitch_semitones);
            }
        }

        ui.separator();
        ui.add(
            egui::Slider::new(&mut self.stretch_ratio, 0.25..=4.0)
                .suffix("×")
                .text("stretch"),
        );
        if ui
            .add_enabled(enabled, egui::Button::new("Stretch"))
            .clicked()
        {
            if let Some(w) = self.wave.as_mut() {
                w.apply_time_stretch(self.stretch_ratio);
            }
        }
    }

    fn paint_overview(painter: &egui::Painter, rect: Rect, overview: &[PeakMinMax], mid_y: f32) {
        if overview.is_empty() {
            return;
        }
        let n = overview.len() as f32;
        let half_h = rect.height() * 0.45;
        let color = Color32::from_rgb(80, 160, 220);
        for (i, p) in overview.iter().enumerate() {
            let x = rect.left() + (i as f32 + 0.5) / n * rect.width();
            let y0 = mid_y - p.max.clamp(-1.0, 1.0) * half_h;
            let y1 = mid_y - p.min.clamp(-1.0, 1.0) * half_h;
            painter.line_segment(
                [Pos2::new(x, y0), Pos2::new(x, y1)],
                Stroke::new(1.0_f32, color),
            );
        }
    }

    fn paint_selection(
        painter: &egui::Painter,
        rect: Rect,
        w: &WaveBuffer,
        vp: &Viewport,
        width: f32,
    ) {
        let x0 = rect.left() + vp.x_of(w.selection.start as f64, width);
        let x1 = rect.left() + vp.x_of(w.selection.end as f64, width);
        let sel = Rect::from_min_max(
            Pos2::new(x0.clamp(rect.left(), rect.right()), rect.top()),
            Pos2::new(x1.clamp(rect.left(), rect.right()), rect.bottom()),
        );
        painter.rect_filled(sel, 0.0, Color32::from_rgba_unmultiplied(70, 120, 200, 50));
        let edge = Stroke::new(1.0_f32, Color32::from_rgb(200, 180, 80));
        for x in [x0, x1] {
            if x >= rect.left() && x <= rect.right() {
                painter.line_segment(
                    [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                    edge,
                );
            }
        }
    }

    fn export(&mut self, selection_only: bool) {
        let Some(w) = self.wave.as_ref() else {
            return;
        };
        let path = PathBuf::from(&self.export_path);
        match w.export_wav(&path, selection_only) {
            Ok(()) => self.status = format!("Exported {}", path.display()),
            Err(e) => self.status = format!("Export failed: {e}"),
        }
    }
}

/// Round a time step to 1, 2 or 5 × a power of ten, so tick labels stay legible.
fn nice_step(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let base = 10f64.powf(exp);
    let n = raw / base;
    let mult = if n <= 1.0 {
        1.0
    } else if n <= 2.0 {
        2.0
    } else if n <= 5.0 {
        5.0
    } else {
        10.0
    };
    mult * base
}

#[derive(Debug, Clone)]
pub enum WaveformAction {
    None,
    Imported(PathBuf),
    ImportedAndRegister(PathBuf),
    PlayPreview {
        samples: Vec<f32>,
        channels: u16,
        sample_rate: u32,
    },
    StopPreview,
}
