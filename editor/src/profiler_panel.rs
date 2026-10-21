//! S7 — Editor Profiler panel (CPU, voices, buses, snapshots, interactive 3D map).

use eframe::egui;

use crate::audio_bridge::ProfilerSnapshot;

#[derive(Debug, Clone)]
pub struct ProfilerPanel {
    pub show: bool,
    pub snap_name: String,
}

impl Default for ProfilerPanel {
    fn default() -> Self {
        Self {
            show: true,
            snap_name: "Combat".into(),
        }
    }
}

pub enum ProfilerAction {
    None,
    Clear,
    ExportNdjson,
    ToggleMute(u32),
    ToggleSolo(u32),
    SetVolume(u32, f32),
    CaptureBase,
    SaveSnapshot,
    TriggerSnapshot { name: String, weight: f32 },
    ClearSnapshot,
    SetEmitterPos { x: f32, z: f32 },
    SetListenerPos { x: f32, z: f32 },
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
        ui.label("CPU timeline (µs)");
        draw_sparkline(ui, &snap.cpu_history, egui::Color32::from_rgb(100, 180, 255), 48.0);
        ui.label("Voices timeline");
        draw_sparkline(ui, &snap.voice_history, egui::Color32::from_rgb(120, 200, 120), 36.0);

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
                    ui.label(&b.name);
                    let mute_label = if b.mute { "M●" } else { "M" };
                    let solo_label = if b.solo { "S●" } else { "S" };
                    if ui
                        .add(
                            egui::Button::new(mute_label)
                                .fill(if b.mute {
                                    egui::Color32::from_rgb(120, 40, 40)
                                } else {
                                    egui::Color32::from_rgb(50, 50, 55)
                                })
                                .min_size(egui::vec2(22.0, 18.0)),
                        )
                        .clicked()
                    {
                        action = ProfilerAction::ToggleMute(b.id);
                    }
                    if ui
                        .add(
                            egui::Button::new(solo_label)
                                .fill(if b.solo {
                                    egui::Color32::from_rgb(140, 120, 30)
                                } else {
                                    egui::Color32::from_rgb(50, 50, 55)
                                })
                                .min_size(egui::vec2(22.0, 18.0)),
                        )
                        .clicked()
                    {
                        action = ProfilerAction::ToggleSolo(b.id);
                    }
                    let mut vol = b.volume_db;
                    if ui
                        .add(
                            egui::DragValue::new(&mut vol)
                                .speed(0.25)
                                .range(-48.0..=12.0)
                                .suffix(" dB")
                                .fixed_decimals(1),
                        )
                        .changed()
                    {
                        action = ProfilerAction::SetVolume(b.id, vol);
                    }
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(70.0, 12.0), egui::Sense::hover());
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
        ui.label("Snapshots");
        ui.horizontal(|ui| {
            if ui.button("Capture base").clicked() {
                action = ProfilerAction::CaptureBase;
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.snap_name)
                    .desired_width(90.0)
                    .hint_text("name"),
            );
            if ui.button("Save").clicked() {
                action = ProfilerAction::SaveSnapshot;
            }
            if ui.button("Clear active").clicked() {
                action = ProfilerAction::ClearSnapshot;
            }
        });
        if let Some((ref name, w)) = &snap.active_snapshot {
            ui.weak(format!("active: {name} @ {:.0}%", w * 100.0));
        } else {
            ui.weak("active: (none)");
        }
        if snap.snapshot_names.is_empty() {
            ui.weak("No snaps — tweak bus dB, Capture base, Save");
        } else {
            for name in &snap.snapshot_names {
                ui.horizontal(|ui| {
                    ui.monospace(name);
                    if ui.small_button("50%").clicked() {
                        action = ProfilerAction::TriggerSnapshot {
                            name: name.clone(),
                            weight: 0.5,
                        };
                    }
                    if ui.small_button("100%").clicked() {
                        action = ProfilerAction::TriggerSnapshot {
                            name: name.clone(),
                            weight: 1.0,
                        };
                    }
                });
            }
        }

        ui.separator();
        ui.label("3D emitters (XZ top-down)");
        ui.weak("Drag orange = emitter · green L = listener · scale ±50m");
        if let Some(a) = draw_emitter_map(ui, snap) {
            action = a;
        }
        if snap.emitters.is_empty() {
            ui.weak("No 3D instances — mark event as 3D and Play");
        } else {
            for (i, e) in snap.emitters.iter().enumerate() {
                ui.weak(format!(
                    "  #{i}  ({:.1}, {:.1}, {:.1})  aud {:.2}",
                    e.position.x, e.position.y, e.position.z, e.audibility
                ));
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

fn draw_sparkline(ui: &mut egui::Ui, data: &[u32], color: egui::Color32, height: f32) {
    if data.is_empty() {
        ui.weak("(no samples — press Play)");
        return;
    }
    let w = ui.available_width().max(80.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(25, 25, 28));
    let max_v = data.iter().copied().max().unwrap_or(1).max(1) as f32;
    let n = data.len().max(1) as f32;
    let mut points = Vec::with_capacity(data.len());
    for (i, &v) in data.iter().enumerate() {
        let x = rect.left() + (i as f32 / (n - 1.0).max(1.0)) * rect.width();
        let y = rect.bottom() - (v as f32 / max_v) * (height - 4.0) - 2.0;
        points.push(egui::pos2(x, y));
    }
    if points.len() >= 2 {
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, color)));
    }
    let last = *data.last().unwrap_or(&0);
    painter.text(
        rect.right_top() + egui::vec2(-4.0, 2.0),
        egui::Align2::RIGHT_TOP,
        format!("{last} (max {max_v:.0})"),
        egui::FontId::monospace(10.0),
        egui::Color32::GRAY,
    );
}

fn draw_emitter_map(ui: &mut egui::Ui, snap: &ProfilerSnapshot) -> Option<ProfilerAction> {
    let size = 140.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(22, 24, 28));
    let mid = rect.center();
    painter.line_segment(
        [egui::pos2(rect.left(), mid.y), egui::pos2(rect.right(), mid.y)],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 48)),
    );
    painter.line_segment(
        [egui::pos2(mid.x, rect.top()), egui::pos2(mid.x, rect.bottom())],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 48)),
    );
    let scale = size / 100.0;
    let to_screen = |x: f32, z: f32| egui::pos2(mid.x + x * scale, mid.y - z * scale);
    let to_world = |pos: egui::Pos2| {
        let x = ((pos.x - mid.x) / scale).clamp(-50.0, 50.0);
        let z = ((mid.y - pos.y) / scale).clamp(-50.0, 50.0);
        (x, z)
    };

    let (lx, _ly, lz) = snap.listener;
    let lp = to_screen(lx, lz);
    painter.circle_filled(lp, 5.0, egui::Color32::from_rgb(80, 200, 120));
    painter.text(
        lp + egui::vec2(6.0, -8.0),
        egui::Align2::LEFT_TOP,
        "L",
        egui::FontId::monospace(10.0),
        egui::Color32::from_rgb(80, 200, 120),
    );
    for e in &snap.emitters {
        let p = to_screen(e.position.x, e.position.z);
        let r = 3.0 + e.audibility.clamp(0.0, 1.0) * 6.0;
        let alpha = (80.0 + e.audibility.clamp(0.0, 1.0) * 175.0) as u8;
        painter.circle_filled(
            p,
            r,
            egui::Color32::from_rgba_unmultiplied(220, 120, 60, alpha),
        );
        painter.line_segment(
            [lp, p],
            egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_unmultiplied(180, 180, 100, 80),
            ),
        );
    }

    let mut action = None;
    if response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let (wx, wz) = to_world(pointer);
            let dist_l = pointer.distance(lp);
            let mut nearest_emitter_d = f32::MAX;
            for e in &snap.emitters {
                let p = to_screen(e.position.x, e.position.z);
                nearest_emitter_d = nearest_emitter_d.min(pointer.distance(p));
            }
            const HIT: f32 = 16.0;
            if nearest_emitter_d < HIT && nearest_emitter_d <= dist_l {
                action = Some(ProfilerAction::SetEmitterPos { x: wx, z: wz });
            } else if dist_l < HIT {
                action = Some(ProfilerAction::SetListenerPos { x: wx, z: wz });
            } else if !snap.emitters.is_empty() {
                action = Some(ProfilerAction::SetEmitterPos { x: wx, z: wz });
            } else {
                action = Some(ProfilerAction::SetListenerPos { x: wx, z: wz });
            }
        }
    }
    if response.double_clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let (wx, wz) = to_world(pointer);
            action = Some(ProfilerAction::SetListenerPos { x: wx, z: wz });
        }
    }
    action
}
