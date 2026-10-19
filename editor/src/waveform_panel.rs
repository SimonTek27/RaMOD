//! egui Waveform panel — multi-format import + preview.

use egui::{self, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use ramod_waveform::{PeakMinMax, WaveBuffer};
use std::path::{Path, PathBuf};

pub struct WaveformPanel {
    pub wave: Option<WaveBuffer>,
    pub clipboard: Vec<f32>,
    pub show: bool,
    pub status: String,
    drag_start: Option<f32>,
    export_path: String,
    import_path: String,
    gain_db: f32,
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
            export_path: "export.wav".into(),
            import_path: String::new(),
            gain_db: 0.0,
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

        let height = 120.0;
        let (response, painter) =
            ui.allocate_painter(Vec2::new(ui.available_width(), height), Sense::click_and_drag());
        let rect = response.rect;

        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.drag_start = Some(((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0));
            }
        }
        if let Some(start_n) = self.drag_start {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(w) = self.wave.as_mut() {
                    let end_n = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                    let a = (start_n.min(end_n) * w.frames() as f32) as usize;
                    let b = (start_n.max(end_n) * w.frames() as f32) as usize;
                    w.set_selection(a, b.max(a + 1).min(w.frames()));
                }
            }
        }
        if response.drag_stopped() {
            self.drag_start = None;
        }

        painter.rect_filled(rect, 2.0, Color32::from_rgb(22, 22, 26));
        painter.rect_stroke(rect, 2.0, Stroke::new(1.0, Color32::from_rgb(50, 50, 55)));

        let mid_y = rect.center().y;
        painter.line_segment(
            [Pos2::new(rect.left(), mid_y), Pos2::new(rect.right(), mid_y)],
            Stroke::new(1.0, Color32::from_rgb(40, 40, 48)),
        );

        if let Some(w) = self.wave.as_ref() {
            let buckets = (rect.width() as usize).clamp(32, 2048);
            let overview = w.overview(buckets);
            Self::paint_overview(&painter, rect, &overview, mid_y);
            Self::paint_selection(&painter, rect, w);
        } else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No waveform — Import or Demo tone",
                egui::FontId::proportional(14.0),
                Color32::from_rgb(120, 120, 130),
            );
        }

        ui.horizontal(|ui| {
            if let Some(w) = self.wave.as_ref() {
                ui.weak(format!(
                    "frames={}  sr={}  ch={}  sel=[{}..{})  {:.3}s",
                    w.frames(),
                    w.sample_rate,
                    w.channels,
                    w.selection.start,
                    w.selection.end,
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
        if ui.button("Undo").clicked() {
            if let Some(w) = self.wave.as_mut() {
                let _ = w.undo();
            }
        }
    }

    fn paint_overview(
        painter: &egui::Painter,
        rect: Rect,
        overview: &[PeakMinMax],
        mid_y: f32,
    ) {
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
                Stroke::new(1.0, color),
            );
        }
    }

    fn paint_selection(painter: &egui::Painter, rect: Rect, w: &WaveBuffer) {
        let frames = w.frames().max(1) as f32;
        let x0 = rect.left() + (w.selection.start as f32 / frames) * rect.width();
        let x1 = rect.left() + (w.selection.end as f32 / frames) * rect.width();
        let sel = Rect::from_min_max(
            Pos2::new(x0, rect.top()),
            Pos2::new(x1, rect.bottom()),
        );
        painter.rect_filled(sel, 0.0, Color32::from_rgba_unmultiplied(70, 120, 200, 50));
        painter.line_segment(
            [Pos2::new(x0, rect.top()), Pos2::new(x0, rect.bottom())],
            Stroke::new(1.0, Color32::from_rgb(200, 180, 80)),
        );
        painter.line_segment(
            [Pos2::new(x1, rect.top()), Pos2::new(x1, rect.bottom())],
            Stroke::new(1.0, Color32::from_rgb(200, 180, 80)),
        );
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
