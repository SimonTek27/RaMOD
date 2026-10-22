//! Multi-track Event Editor panel (timeline, tracks, instruments, AHDSR, markers).

use eframe::egui;

use crate::project::{EventEntry, Project};

#[derive(Debug, Clone, Default)]
pub struct EventEditorState {
    pub selected_track: Option<usize>,
    pub selected_instrument: Option<(usize, usize)>,
    pub marker_name: String,
    pub marker_time: f32,
    pub marker_target: String,
    /// True while dragging a clip on the timeline.
    pub dragging_clip: bool,
}

pub enum EventEditorAction {
    None,
    Changed,
}

pub fn ensure_default_track(ev: &mut EventEntry) {
    if ev.tracks.is_empty() {
        ev.tracks.push(crate::project::TrackEntry::new("Audio 1"));
    }
}

pub fn show_event_editor(
    ui: &mut egui::Ui,
    project: &mut Project,
    event_index: usize,
    state: &mut EventEditorState,
) -> EventEditorAction {
    let mut dirty = false;

    ui.heading("Event Editor");
    if let Some(e) = project.events.get(event_index) {
        ui.monospace(&e.path);
    }

    ui.horizontal(|ui| {
        if ui.button("+ Track").clicked() {
            let n = project.events[event_index].tracks.len();
            let _ = project.add_track(event_index, format!("Audio {}", n + 1));
            dirty = true;
        }
        if ui.button("+ Instrument").clicked() {
            let ti = state.selected_track.unwrap_or(0);
            let asset = project.events[event_index].asset_index;
            let _ = project.add_instrument(event_index, ti, format!("clip_{}", ti), asset);
            dirty = true;
        }
    });

    ui.separator();

    ui.horizontal(|ui| {
        if let Some(e) = project.events.get_mut(event_index) {
            if ui.checkbox(&mut e.looping, "Loop").changed() {
                dirty = true;
            }
            if ui.checkbox(&mut e.is_3d, "3D").changed() {
                dirty = true;
            }
            ui.label("Priority");
            if ui
                .add(egui::DragValue::new(&mut e.priority).range(0..=255))
                .changed()
            {
                dirty = true;
            }
            ui.label("Bus");
            egui::ComboBox::from_id_salt("event_output_bus")
                .selected_text(&e.output_bus)
                .show_ui(ui, |ui| {
                    for name in ["SFX", "Music", "Master"] {
                        if ui.selectable_label(e.output_bus == name, name).clicked() {
                            e.output_bus = name.to_string();
                            dirty = true;
                        }
                    }
                });
        }
    });

    ui.separator();

    let length = project
        .events
        .get(event_index)
        .map(|e| e.length.max(0.5))
        .unwrap_or(4.0);
    let track_count = project
        .events
        .get(event_index)
        .map(|e| e.tracks.len())
        .unwrap_or(0);

    if track_count == 0 {
        ui.weak("No tracks — click + Track to start the timeline.");
        return if dirty {
            EventEditorAction::Changed
        } else {
            EventEditorAction::None
        };
    }

    let timeline_h = (track_count as f32 * 36.0).clamp(80.0, 280.0);
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), timeline_h),
        egui::Sense::click_and_drag(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(32, 32, 36));

    if let Some(ev) = project.events.get(event_index) {
        let row_h = rect.height() / track_count.max(1) as f32;
        for (ti, track) in ev.tracks.iter().enumerate() {
            let y0 = rect.top() + ti as f32 * row_h;
            let row = egui::Rect::from_min_size(
                egui::pos2(rect.left(), y0),
                egui::vec2(rect.width(), row_h - 2.0),
            );
            painter.rect_filled(row, 1.0, egui::Color32::from_rgb(40, 40, 46));
            painter.text(
                row.left_top() + egui::vec2(4.0, 4.0),
                egui::Align2::LEFT_TOP,
                &track.name,
                egui::FontId::proportional(12.0),
                egui::Color32::LIGHT_GRAY,
            );
            for (ii, inst) in track.instruments.iter().enumerate() {
                let x0 = row.left() + (inst.start_time / length) * row.width();
                let w = ((inst.length.max(0.05) / length) * row.width()).max(8.0);
                let ir = egui::Rect::from_min_size(
                    egui::pos2(x0, y0 + 14.0),
                    egui::vec2(w, row_h - 18.0),
                );
                let selected = state.selected_instrument == Some((ti, ii));
                let color = if selected {
                    egui::Color32::from_rgb(100, 180, 255)
                } else {
                    egui::Color32::from_rgb(70, 120, 180)
                };
                painter.rect_filled(ir, 2.0, color);
            }
        }
        for m in &ev.markers {
            let x = rect.left() + (m.time / length) * rect.width();
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.5, egui::Color32::from_rgb(220, 200, 60)),
            );
        }
    }

    let hit_instrument = |pointer: egui::Pos2| -> Option<(usize, usize)> {
        let Some(ev) = project.events.get(event_index) else {
            return None;
        };
        let row_h = rect.height() / track_count.max(1) as f32;
        for (ti, track) in ev.tracks.iter().enumerate() {
            let y0 = rect.top() + ti as f32 * row_h;
            for (ii, inst) in track.instruments.iter().enumerate() {
                let x0 = rect.left() + (inst.start_time / length) * rect.width();
                let w = ((inst.length.max(0.05) / length) * rect.width()).max(8.0);
                let ir = egui::Rect::from_min_size(
                    egui::pos2(x0, y0 + 14.0),
                    egui::vec2(w, row_h - 18.0),
                );
                if ir.contains(pointer) {
                    return Some((ti, ii));
                }
            }
        }
        None
    };

    if resp.drag_started() || resp.clicked() {
        if let Some(pointer) = resp.interact_pointer_pos() {
            if let Some((ti, ii)) = hit_instrument(pointer) {
                state.selected_track = Some(ti);
                state.selected_instrument = Some((ti, ii));
                state.dragging_clip = resp.drag_started();
            }
        }
    }

    if resp.dragged() && state.dragging_clip {
        if let Some((ti, ii)) = state.selected_instrument {
            let dx = resp.drag_delta().x;
            if rect.width() > 1.0 {
                let dt = (dx / rect.width()) * length;
                if let Some(ev) = project.events.get_mut(event_index) {
                    if let Some(track) = ev.tracks.get_mut(ti) {
                        if let Some(inst) = track.instruments.get_mut(ii) {
                            inst.start_time = (inst.start_time + dt).max(0.0);
                            let max_start = (length - inst.length * 0.1).max(0.0);
                            inst.start_time = inst.start_time.min(max_start);
                            dirty = true;
                        }
                    }
                }
            }
        }
    }

    if resp.drag_stopped() {
        state.dragging_clip = false;
    }

    if let Some((ti, ii)) = state.selected_instrument {
        ui.separator();
        ui.label(format!("Instrument [{ti}:{ii}]"));
        if let Some(ev) = project.events.get_mut(event_index) {
            if let Some(track) = ev.tracks.get_mut(ti) {
                if let Some(inst) = track.instruments.get_mut(ii) {
                    ui.horizontal(|ui| {
                        ui.label("Start");
                        if ui
                            .add(
                                egui::DragValue::new(&mut inst.start_time)
                                    .speed(0.01)
                                    .range(0.0..=60.0)
                                    .suffix(" s"),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        ui.label("Len");
                        if ui
                            .add(
                                egui::DragValue::new(&mut inst.length)
                                    .speed(0.01)
                                    .range(0.05..=60.0)
                                    .suffix(" s"),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        if ui.checkbox(&mut inst.looping, "Loop").changed() {
                            dirty = true;
                        }
                        ui.label("Prob");
                        if ui
                            .add(
                                egui::DragValue::new(&mut inst.probability)
                                    .speed(0.01)
                                    .range(0.0..=1.0),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.strong("AHDSR");
                        ui.label("A");
                        if ui
                            .add(
                                egui::DragValue::new(&mut inst.attack)
                                    .speed(0.005)
                                    .range(0.0..=5.0)
                                    .suffix("s"),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        ui.label("H");
                        if ui
                            .add(
                                egui::DragValue::new(&mut inst.hold)
                                    .speed(0.005)
                                    .range(0.0..=5.0)
                                    .suffix("s"),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        ui.label("D");
                        if ui
                            .add(
                                egui::DragValue::new(&mut inst.decay)
                                    .speed(0.005)
                                    .range(0.0..=5.0)
                                    .suffix("s"),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        ui.label("S");
                        if ui
                            .add(
                                egui::DragValue::new(&mut inst.sustain)
                                    .speed(0.01)
                                    .range(0.0..=1.0),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        ui.label("R");
                        if ui
                            .add(
                                egui::DragValue::new(&mut inst.release)
                                    .speed(0.005)
                                    .range(0.0..=5.0)
                                    .suffix("s"),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                    });
                }
            }
        }
    }

    ui.weak("Drag clips on the timeline to move · click to select");
    ui.separator();
    ui.label("Markers");
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.marker_name)
                .desired_width(80.0)
                .hint_text("name"),
        );
        ui.add(
            egui::DragValue::new(&mut state.marker_time)
                .speed(0.05)
                .prefix("t="),
        );
        ui.add(
            egui::TextEdit::singleline(&mut state.marker_target)
                .desired_width(140.0)
                .hint_text("target event path"),
        );
        if ui.button("+ Marker").clicked() {
            if let Some(e) = project.events.get_mut(event_index) {
                let target = if state.marker_target.trim().is_empty() {
                    None
                } else {
                    Some(state.marker_target.trim().to_string())
                };
                e.markers.push(crate::project::MarkerEntry {
                    name: if state.marker_name.is_empty() {
                        format!("m{}", e.markers.len())
                    } else {
                        state.marker_name.clone()
                    },
                    time: state.marker_time,
                    target_event: target,
                });
                dirty = true;
            }
        }
    });

    if let Some(e) = project.events.get(event_index) {
        for m in &e.markers {
            let tgt = m.target_event.as_deref().unwrap_or("—");
            ui.weak(format!("  {} @ {:.2}s → {}", m.name, m.time, tgt));
        }
    }

    if dirty {
        EventEditorAction::Changed
    } else {
        EventEditorAction::None
    }
}
