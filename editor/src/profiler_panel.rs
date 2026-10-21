//! S7 — Editor Profiler panel (CPU, voices, spectrum, buses).

use eframe::egui;

use crate::audio_bridge::ProfilerSnapshot;

#[derive(Debug, Clone)]
pub struct ProfilerPanel {
    pub show: bool,
}

impl Default for ProfilerPanel {
    fn default() -> Self {
        Self { show: true }
    }
}

pub enum ProfilerAction {
    None,
    Clear,
    ExportNdjson,
}

impl ProfilerPanel {
    pub fn ui(&mut self, ui: &mut egui::Ui, snap: &ProfilerSnapshot) -> ProfilerAction {
        let mut action = ProfilerAction::None;

        ui.horizontal(|ui| {
            ui.heading("Profiler");
            ui.separator();
            if ui.button("Clear").clicked() {
                action = ProfilerAction::Clear;
            }
            if ui.button("Export NDJSON").clicked() {
                action = ProfilerAction::ExportNdjson;
            }
            ui.weak(format!("{} frames", snap.session_frames));
        });
        ui.separator();

        ui.horizontal(|ui| {
            kpi(ui, "CPU", format!("{} µs", snap.cpu_render_us));
            kpi(ui, "Avg CPU", format!("{:.0} µs", snap.avg_cpu_us));
            kpi(
                ui,
                "Voices",
                format!("{} (peak {})", snap.active_voices, snap.peak_voices),
            );
            kpi(ui, "Instances", format!("{}", snap.active_instances));
        });

        ui.horizontal(|ui| {
            kpi(ui, "Peak", format!("{:.3}", snap.master_peak));
            kpi(ui, "RMS", format!("{:.3}", snap.master_rms));
            let (rect, _) = ui.allocate_exact_size(egui::vec2(120.0, 16.0), egui::Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(28, 28, 30));
            let fill = snap.master_peak.clamp(0.0, 1.0) * rect.width();
            let color = if snap.master_peak > 0.9 {
                egui::Color32::from_rgb(220, 60, 60)
            } else if snap.master_peak > 0.6 {
                egui::Color32::from_rgb(220, 180, 40)
            } else {
                egui::Color32::from_rgb(60, 180, 90)
            };
            if fill > 0.5 {
                painter.rect_filled(
                    egui::Rect::from_min_size(rect.min, egui::vec2(fill, rect.height())),
                    2.0,
                    color,
                );
            }
        });

        ui.separator();
        ui.label("Playing events");
        if snap.playing_events.is_empty() {
            ui.weak("(none)");
        } else {
            for p in &snap.playing_events {
                ui.monospace(p);
            }
        }

        ui.separator();
        ui.label("Buses");
        if snap.bus_meters.is_empty() {
            ui.weak("(no buses)");
        } else {
            for b in &snap.bus_meters {
                ui.horizontal(|ui| {
                    let flags = match (b.mute, b.solo) {
                        (true, _) => " [M]",
                        (_, true) => " [S]",
                        _ => "",
                    };
                    ui.label(format!("{}{}  {:+.1} dB", b.name, flags, b.volume_db));
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(100.0, 12.0), egui::Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(28, 28, 30));
                    let fill = b.peak.clamp(0.0, 1.0) * rect.width();
                    let color = if b.peak > 0.9 {
                        egui::Color32::from_rgb(220, 60, 60)
                    } else if b.peak > 0.6 {
                        egui::Color32::from_rgb(220, 180, 40)
                    } else {
                        egui::Color32::from_rgb(60, 180, 90)
                    };
                    if fill > 0.5 {
                        painter.rect_filled(
                            egui::Rect::from_min_size(rect.min, egui::vec2(fill, rect.height())),
                            2.0,
                            color,
                        );
                    }
                    ui.weak(format!("p {:.2}", b.peak));
                });
            }
        }

        ui.separator();
        ui.label("Spectrum");
        let bands = &snap.spectrum_bands;
        if bands.is_empty() {
            ui.weak("(no data)");
        } else {
            let h = 64.0;
            let w = ui.available_width().max(80.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(25, 25, 28));
            let n = bands.len().max(1) as f32;
            let bar_w = (rect.width() / n).max(1.0) - 1.0;
            for (i, &lv) in bands.iter().enumerate() {
                let x = rect.left() + (i as f32 / n) * rect.width();
                let bh = lv.clamp(0.0, 1.0) * (h - 2.0);
                let bar = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - bh),
                    egui::vec2(bar_w, bh),
                );
                painter.rect_filled(bar, 1.0, egui::Color32::from_rgb(80, 160, 220));
            }
        }

        action
    }
}

fn kpi(ui: &mut egui::Ui, label: &str, value: String) {
    ui.vertical(|ui| {
        ui.weak(label);
        ui.strong(value);
    });
    ui.add_space(12.0);
}
