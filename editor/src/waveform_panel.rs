//! egui Waveform / GoldWave-style panel (S9 UI).

use egui::{self, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use ramod_waveform::{PeakMinMax, WaveBuffer};
use std::path::PathBuf;

pub struct WaveformPanel {
    pub wave: Option<WaveBuffer>,
    pub clipboard: Vec<f32>,
    pub show: bool,
    pub status: String,
    drag_start: Option<f32>,
    export_path: String,
    gain_db: f32,
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
            gain_db: 0.0,
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
                self.status = "Demo tone 440 Hz (0.5 s)".into();
            }
            Err(_) => self.status = "Failed to create demo tone".into(),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
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
            ui.separator();
            self.toolbar(ui);
        });

        ui.separator();

        let height = 120.0;
        let (response, painter) =
            ui.allocate_painter(Vec2::new(ui.available_width(), height), Sense::click_and_drag());
        let rect = response.rect;

        // Update selection from drag before paint
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
                "No waveform — click Demo tone",
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
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        if self.wave.is_none() {
            return;
        }

        if ui.button("Cut").clicked() {
            if let Some(w) = self.wave.as_mut() {
                self.clipboard = w.cut();
                self.status = format!("Cut {} samples", self.clipboard.len());
            }
        }
        if ui.button("Copy").clicked() {
            if let Some(w) = self.wave.as_ref() {
                self.clipboard = w.copy();
                self.status = format!("Copied {} samples", self.clipboard.len());
            }
        }
        if ui.button("Paste").clicked() {
            let clip = self.clipboard.clone();
            if let Some(w) = self.wave.as_mut() {
                w.paste(&clip);
                self.status = "Pasted".into();
            }
        }
        if ui.button("Silence").clicked() {
            if let Some(w) = self.wave.as_mut() {
                w.apply_silence();
                self.status = "Silence".into();
            }
        }
        if ui.button("Reverse").clicked() {
            if let Some(w) = self.wave.as_mut() {
                w.apply_reverse();
                self.status = "Reverse".into();
            }
        }
        if ui.button("Fade in").clicked() {
            if let Some(w) = self.wave.as_mut() {
                w.apply_fade_in();
                self.status = "Fade in".into();
            }
        }
        if ui.button("Fade out").clicked() {
            if let Some(w) = self.wave.as_mut() {
                w.apply_fade_out();
                self.status = "Fade out".into();
            }
        }
        if ui.button("Normalize").clicked() {
            if let Some(w) = self.wave.as_mut() {
                w.apply_normalize(0.95);
                self.status = "Normalize".into();
            }
        }
        ui.label("Gain");
        ui.add(egui::Slider::new(&mut self.gain_db, -24.0..=12.0).suffix(" dB"));
        if ui.button("Apply gain").clicked() {
            let g = 10f32.powf(self.gain_db / 20.0);
            if let Some(w) = self.wave.as_mut() {
                w.apply_gain(g);
                self.status = format!("Gain {:+.1} dB", self.gain_db);
            }
        }
        if ui.button("Undo").clicked() {
            if let Some(w) = self.wave.as_mut() {
                if w.undo() {
                    self.status = "Undo".into();
                } else {
                    self.status = "Nothing to undo".into();
                }
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
            self.status = "No waveform".into();
            return;
        };
        let path = PathBuf::from(&self.export_path);
        match w.export_wav(&path, selection_only) {
            Ok(()) => {
                self.status = format!(
                    "Exported {} ({})",
                    path.display(),
                    if selection_only { "selection" } else { "full" }
                );
            }
            Err(e) => self.status = format!("Export failed: {e}"),
        }
    }
}
