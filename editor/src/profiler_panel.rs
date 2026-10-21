//! S7 — Profiler panel (CPU, buses, FX params/presets, snapshots, 3D Doppler).

use eframe::egui;

use crate::audio_bridge::ProfilerSnapshot;

#[derive(Debug, Clone)]
pub struct ProfilerPanel {
    pub show: bool,
    pub snap_name: String,
    pub fx_preset_name: String,
}

impl Default for ProfilerPanel {
    fn default() -> Self {
        Self {
            show: true,
            snap_name: "Combat".into(),
            fx_preset_name: "MyChain".into(),
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
    SetEmitterPos { x: f32, z: f32, vx: f32, vz: f32 },
    SetListenerPos { x: f32, z: f32, vx: f32, vz: f32 },
    AddLowPass(u32),
    AddHighPass(u32),
    AddCompressor(u32),
    AddReverb(u32),
    ClearInserts(u32),
    UpdateInsert(crate::audio_bridge::BusInsertInfo),
    RemoveInsert { bus_id: u32, index: usize },
    SaveFxPreset,
    ApplyFxPreset(String),
    DeleteFxPreset(String),
}

impl ProfilerPanel {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        snap: &ProfilerSnapshot,
        fx_preset_names: &[String],
    ) -> ProfilerAction {
        let mut action = ProfilerAction::None;

        ui.horizontal(|ui| {
            ui.heading("Profiler");
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
            kpi(ui, "Avg", format!("{:.0} µs", snap.avg_cpu_us));
            kpi(
                ui,
                "Voices",
                format!("{} / peak {}", snap.active_voices, snap.peak_voices),
            );
        });

        ui.separator();
        ui.label("CPU timeline");
        draw_sparkline(ui, &snap.cpu_history, egui::Color32::from_rgb(100, 180, 255), 40.0);
        ui.label("Voices");
        draw_sparkline(ui, &snap.voice_history, egui::Color32::from_rgb(120, 200, 120), 32.0);

        ui.separator();
        ui.label("Buses");
        for b in &snap.bus_meters {
            ui.horizontal(|ui| {
                ui.label(&b.name);
                if ui
                    .add(egui::Button::new(if b.mute { "M●" } else { "M" }).min_size(egui::vec2(22.0, 18.0)))
                    .clicked()
                {
                    action = ProfilerAction::ToggleMute(b.id);
                }
                if ui
                    .add(egui::Button::new(if b.solo { "S●" } else { "S" }).min_size(egui::vec2(22.0, 18.0)))
                    .clicked()
                {
                    action = ProfilerAction::ToggleSolo(b.id);
                }
                let mut vol = b.volume_db;
                if ui
                    .add(egui::DragValue::new(&mut vol).speed(0.25).range(-48.0..=12.0).suffix(" dB"))
                    .changed()
                {
                    action = ProfilerAction::SetVolume(b.id, vol);
                }
            });
            ui.horizontal(|ui| {
                ui.weak("FX");
                if ui.small_button("LP").clicked() {
                    action = ProfilerAction::AddLowPass(b.id);
                }
                if ui.small_button("HP").clicked() {
                    action = ProfilerAction::AddHighPass(b.id);
                }
                if ui.small_button("Comp").clicked() {
                    action = ProfilerAction::AddCompressor(b.id);
                }
                if ui.small_button("Rev").clicked() {
                    action = ProfilerAction::AddReverb(b.id);
                }
                if ui.small_button("✕").clicked() {
                    action = ProfilerAction::ClearInserts(b.id);
                }
                let n = snap
                    .insert_counts
                    .iter()
                    .find(|(id, _)| *id == b.id)
                    .map(|(_, c)| *c)
                    .unwrap_or(0);
                if n > 0 {
                    ui.weak(format!("{n} ins"));
                }
            });
            for ins in snap.bus_inserts.iter().filter(|i| i.bus_id == b.id) {
                ui.horizontal(|ui| {
                    ui.monospace(format!("  [{}] {}", ins.index, ins.kind));
                    let mut info = ins.clone();
                    match ins.kind.as_str() {
                        "LowPass" | "HighPass" => {
                            if ui
                                .add(
                                    egui::DragValue::new(&mut info.p0)
                                        .speed(10.0)
                                        .range(20.0..=20000.0)
                                        .suffix(" Hz"),
                                )
                                .changed()
                            {
                                action = ProfilerAction::UpdateInsert(info);
                            }
                        }
                        "Compressor" => {
                            if ui
                                .add(egui::DragValue::new(&mut info.p0).speed(0.5).range(-60.0..=0.0).suffix(" thr"))
                                .changed()
                            {
                                action = ProfilerAction::UpdateInsert(info.clone());
                            }
                            if ui
                                .add(egui::DragValue::new(&mut info.p1).speed(0.1).range(1.0..=20.0).suffix(":1"))
                                .changed()
                            {
                                action = ProfilerAction::UpdateInsert(info.clone());
                            }
                            if ui
                                .add(egui::DragValue::new(&mut info.p2).speed(0.25).range(0.0..=24.0).suffix(" mk"))
                                .changed()
                            {
                                action = ProfilerAction::UpdateInsert(info);
                            }
                        }
                        "Reverb" => {
                            if ui
                                .add(egui::DragValue::new(&mut info.p0).speed(1.0).range(1.0..=500.0).suffix(" ms"))
                                .changed()
                            {
                                action = ProfilerAction::UpdateInsert(info.clone());
                            }
                            if ui
                                .add(egui::DragValue::new(&mut info.p1).speed(0.01).range(0.0..=0.95).suffix(" fb"))
                                .changed()
                            {
                                action = ProfilerAction::UpdateInsert(info.clone());
                            }
                            if ui
                                .add(egui::DragValue::new(&mut info.p2).speed(0.01).range(0.0..=1.0).suffix(" wet"))
                                .changed()
                            {
                                action = ProfilerAction::UpdateInsert(info);
                            }
                        }
                        _ => {
                            if ui
                                .add(egui::DragValue::new(&mut info.p0).speed(0.25).range(-48.0..=12.0).suffix(" dB"))
                                .changed()
                            {
                                action = ProfilerAction::UpdateInsert(info);
                            }
                        }
                    }
                    if ui.small_button("×").clicked() {
                        action = ProfilerAction::RemoveInsert {
                            bus_id: ins.bus_id,
                            index: ins.index,
                        };
                    }
                });
            }
        }

        ui.separator();
        ui.label("Snapshots");
        ui.horizontal(|ui| {
            if ui.button("Capture base").clicked() {
                action = ProfilerAction::CaptureBase;
            }
            ui.add(egui::TextEdit::singleline(&mut self.snap_name).desired_width(80.0));
            if ui.button("Save").clicked() {
                action = ProfilerAction::SaveSnapshot;
            }
            if ui.button("Clear").clicked() {
                action = ProfilerAction::ClearSnapshot;
            }
        });
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

        ui.separator();
        ui.label("FX presets");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.fx_preset_name)
                    .desired_width(90.0)
                    .hint_text("name"),
            );
            if ui
                .button("Save FX")
                .on_hover_text("Save current bus inserts as preset")
                .clicked()
            {
                action = ProfilerAction::SaveFxPreset;
            }
        });
        if fx_preset_names.is_empty() {
            ui.weak("No FX presets — add LP/Rev then Save FX");
        } else {
            for name in fx_preset_names {
                ui.horizontal(|ui| {
                    ui.monospace(name);
                    if ui.small_button("Apply").clicked() {
                        action = ProfilerAction::ApplyFxPreset(name.clone());
                    }
                    if ui.small_button("Del").clicked() {
                        action = ProfilerAction::DeleteFxPreset(name.clone());
                    }
                });
            }
        }

        ui.separator();
        ui.label("3D emitters");
        ui.weak("Drag = move + Doppler");
        if let Some(a) = draw_emitter_map(ui, snap) {
            action = a;
        }

        action
    }
}

fn kpi(ui: &mut egui::Ui, label: &str, value: String) {
    ui.vertical(|ui| {
        ui.weak(label);
        ui.strong(value);
    });
    ui.add_space(8.0);
}

fn draw_sparkline(ui: &mut egui::Ui, data: &[u32], color: egui::Color32, height: f32) {
    if data.is_empty() {
        ui.weak("(play)");
        return;
    }
    let w = ui.available_width().max(60.0);
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
}

fn draw_emitter_map(ui: &mut egui::Ui, snap: &ProfilerSnapshot) -> Option<ProfilerAction> {
    let size = 120.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(22, 24, 28));
    let mid = rect.center();
    let scale = size / 100.0;
    let to_screen = |x: f32, z: f32| egui::pos2(mid.x + x * scale, mid.y - z * scale);
    let to_world = |pos: egui::Pos2| {
        (
            ((pos.x - mid.x) / scale).clamp(-50.0, 50.0),
            ((mid.y - pos.y) / scale).clamp(-50.0, 50.0),
        )
    };
    let (lx, _, lz) = snap.listener;
    let lp = to_screen(lx, lz);
    painter.circle_filled(lp, 5.0, egui::Color32::from_rgb(80, 200, 120));
    for e in &snap.emitters {
        let p = to_screen(e.position.x, e.position.z);
        painter.circle_filled(p, 4.0 + e.audibility * 4.0, egui::Color32::from_rgb(220, 120, 60));
        painter.line_segment([lp, p], egui::Stroke::new(1.0, egui::Color32::from_gray(80)));
    }
    let d = response.drag_delta();
    let vx = (d.x / scale) * 12.0;
    let vz = (-d.y / scale) * 12.0;
    let mut action = None;
    if response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let (wx, wz) = to_world(pointer);
            if !snap.emitters.is_empty() {
                action = Some(ProfilerAction::SetEmitterPos { x: wx, z: wz, vx, vz });
            } else {
                action = Some(ProfilerAction::SetListenerPos { x: wx, z: wz, vx, vz });
            }
        }
    } else if response.drag_stopped() {
        if !snap.emitters.is_empty() {
            let e = &snap.emitters[0];
            action = Some(ProfilerAction::SetEmitterPos {
                x: e.position.x,
                z: e.position.z,
                vx: 0.0,
                vz: 0.0,
            });
        }
    }
    action
}
