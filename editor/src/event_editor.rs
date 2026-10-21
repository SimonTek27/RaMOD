//! S5 — Multi-track Event Editor panel (FMOD Studio–style timeline).

use crate::project::{EventEntry, Project};
use eframe::egui;

#[derive(Debug, Clone, Copy, Default)]
pub struct EventEditorState {
    pub selected_track: Option<usize>,
    pub selected_instrument: Option<(usize, usize)>,
}

pub enum EventEditorAction {
    None,
    Changed,
}

pub fn show_event_editor(
    ui: &mut egui::Ui,
    project: &mut Project,
    event_index: usize,
    state: &mut EventEditorState,
) -> EventEditorAction {
    let Some(ev) = project.events.get(event_index).cloned() else {
        ui.label("No event selected");
        return EventEditorAction::None;
    };

    let mut dirty = false;

    ui.horizontal(|ui| {
        ui.heading(&ev.path);
        ui.separator();
        ui.label("Length");
        let mut len = ev.length;
        if ui
            .add(egui::DragValue::new(&mut len).speed(0.05).range(0.5..=60.0))
            .changed()
        {
            if let Some(e) = project.events.get_mut(event_index) {
                e.length = len;
                dirty = true;
            }
        }
        ui.separator();
        if ui.button("+ Track").clicked() {
            let n = project
                .events
                .get(event_index)
                .map(|e| e.tracks.len())
                .unwrap_or(0);
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

    let timeline_w = ui.available_width().max(200.0);
    let header_h = 18.0;
    let (header_rect, _) =
        ui.allocate_exact_size(egui::vec2(timeline_w, header_h), egui::Sense::hover());
    let painter = ui.painter_at(header_rect);
    painter.rect_filled(header_rect, 0.0, egui::Color32::from_rgb(35, 35, 38));
    for t in 0..=(length.ceil() as i32) {
        let x = header_rect.left() + (t as f32 / length) * header_rect.width();
        painter.line_segment(
            [
                egui::pos2(x, header_rect.top()),
                egui::pos2(x, header_rect.bottom()),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 70, 70)),
        );
        painter.text(
            egui::pos2(x + 2.0, header_rect.top()),
            egui::Align2::LEFT_TOP,
            format!("{t}s"),
            egui::FontId::monospace(10.0),
            egui::Color32::LIGHT_GRAY,
        );
    }

    // Markers row
    let (mark_rect, _) =
        ui.allocate_exact_size(egui::vec2(timeline_w, 16.0), egui::Sense::click());
    {
        let painter = ui.painter_at(mark_rect);
        painter.rect_filled(mark_rect, 0.0, egui::Color32::from_rgb(30, 30, 32));
        if let Some(e) = project.events.get(event_index) {
            for m in &e.markers {
                let x = mark_rect.left() + (m.time / length) * mark_rect.width();
                painter.line_segment(
                    [
                        egui::pos2(x, mark_rect.top()),
                        egui::pos2(x, mark_rect.bottom()),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 180, 40)),
                );
                painter.text(
                    egui::pos2(x + 3.0, mark_rect.top()),
                    egui::Align2::LEFT_TOP,
                    &m.name,
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(255, 200, 80),
                );
            }
        }
    }

    let track_h = 48.0;
    let colors = [
        egui::Color32::from_rgb(70, 130, 180),
        egui::Color32::from_rgb(60, 160, 100),
        egui::Color32::from_rgb(180, 120, 60),
        egui::Color32::from_rgb(140, 90, 170),
        egui::Color32::from_rgb(180, 80, 90),
    ];

    for ti in 0..track_count {
        let track_name = project.events[event_index].tracks[ti].name.clone();

        ui.horizontal(|ui| {
            let selected = state.selected_track == Some(ti);
            if ui
                .selectable_label(selected, format!("Track: {track_name}"))
                .clicked()
            {
                state.selected_track = Some(ti);
            }
            if let Some(e) = project.events.get_mut(event_index) {
                if let Some(tr) = e.tracks.get_mut(ti) {
                    if ui.checkbox(&mut tr.mute, "M").changed() {
                        dirty = true;
                    }
                    if ui.checkbox(&mut tr.solo, "S").changed() {
                        dirty = true;
                    }
                }
            }
        });

        let muted = project.events[event_index].tracks[ti].mute;
        let solo = project.events[event_index].tracks[ti].solo;

        let (row_rect, row_resp) =
            ui.allocate_exact_size(egui::vec2(timeline_w, track_h), egui::Sense::click());
        if row_resp.clicked() {
            state.selected_track = Some(ti);
        }
        let painter = ui.painter_at(row_rect);
        let bg = if muted {
            egui::Color32::from_rgb(40, 40, 40)
        } else {
            egui::Color32::from_rgb(45, 45, 50)
        };
        painter.rect_filled(row_rect, 2.0, bg);
        if solo {
            painter.rect_stroke(
                row_rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 180, 40)),
            );
        }

        let instruments = project.events[event_index].tracks[ti].instruments.clone();
        for (ii, inst) in instruments.iter().enumerate() {
            let x0 = row_rect.left() + (inst.start_time / length) * row_rect.width();
            let w = (inst.length / length) * row_rect.width();
            let clip = egui::Rect::from_min_size(
                egui::pos2(x0, row_rect.top() + 6.0),
                egui::vec2(w.max(8.0), track_h - 12.0),
            );
            let color = colors[ti % colors.len()];
            let sel = state.selected_instrument == Some((ti, ii));
            painter.rect_filled(
                clip,
                3.0,
                color.gamma_multiply(if sel { 1.2 } else { 0.85 }),
            );
            painter.text(
                clip.left_top() + egui::vec2(4.0, 4.0),
                egui::Align2::LEFT_TOP,
                &inst.name,
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
            let id = ui.id().with(("clip", ti, ii));
            let resp = ui.interact(clip, id, egui::Sense::click());
            if resp.clicked() {
                state.selected_track = Some(ti);
                state.selected_instrument = Some((ti, ii));
            }
        }
    }

    if let Some((ti, ii)) = state.selected_instrument {
        ui.separator();
        ui.label("Instrument");
        if let Some(e) = project.events.get_mut(event_index) {
            if let Some(tr) = e.tracks.get_mut(ti) {
                if let Some(inst) = tr.instruments.get_mut(ii) {
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        if ui.text_edit_singleline(&mut inst.name).changed() {
                            dirty = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Start");
                        if ui
                            .add(
                                egui::DragValue::new(&mut inst.start_time)
                                    .speed(0.01)
                                    .range(0.0..=60.0),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        ui.label("Length");
                        if ui
                            .add(
                                egui::DragValue::new(&mut inst.length)
                                    .speed(0.01)
                                    .range(0.05..=60.0),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        if ui.checkbox(&mut inst.looping, "Loop").changed() {
                            dirty = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Asset #");
                        let mut idx = inst.asset_index.map(|i| i as i32).unwrap_or(-1);
                        if ui
                            .add(egui::DragValue::new(&mut idx).range(-1..=255))
                            .changed()
                        {
                            inst.asset_index = if idx < 0 { None } else { Some(idx as usize) };
                            dirty = true;
                        }
                    });
                }
            }
        }
    }

    ui.separator();
    ui.label("Markers");
    if let Some(e) = project.events.get_mut(event_index) {
        ui.horizontal(|ui| {
            if ui.button("+ Marker").clicked() {
                let t = (e.length * 0.5).max(0.0);
                e.markers
                    .push(crate::project::MarkerEntry::new(format!("M{}", e.markers.len()), t));
                dirty = true;
            }
        });
        let mut remove = None;
        for (mi, m) in e.markers.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if ui.text_edit_singleline(&mut m.name).changed() {
                    dirty = true;
                }
                if ui
                    .add(egui::DragValue::new(&mut m.time).speed(0.01).range(0.0..=60.0).prefix("t "))
                    .changed()
                {
                    dirty = true;
                }
                if ui.small_button("x").clicked() {
                    remove = Some(mi);
                }
            });
        }
        if let Some(i) = remove {
            e.markers.remove(i);
            dirty = true;
        }
    }

    ui.separator();
    ui.label("Automation");
    if let Some(e) = project.events.get_mut(event_index) {
        if e.automation.is_empty() {
            if ui.button("+ Intensity fade").clicked() {
                e.automation.push(crate::project::AutomationEntry::intensity_fade(
                    1.0,
                    0.0,
                    e.length.max(0.5),
                ));
                dirty = true;
            }
        } else {
            for (ai, a) in e.automation.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(&a.parameter);
                    if a.keys.len() >= 2 {
                        let mut v0 = a.keys[0].1;
                        let mut v1 = a.keys[a.keys.len() - 1].1;
                        if ui
                            .add(
                                egui::DragValue::new(&mut v0)
                                    .speed(0.01)
                                    .range(0.0..=2.0)
                                    .prefix("0s "),
                            )
                            .changed()
                        {
                            a.keys[0].1 = v0;
                            dirty = true;
                        }
                        if ui
                            .add(
                                egui::DragValue::new(&mut v1)
                                    .speed(0.01)
                                    .range(0.0..=2.0)
                                    .prefix("end "),
                            )
                            .changed()
                        {
                            let last = a.keys.len() - 1;
                            a.keys[last].1 = v1;
                            dirty = true;
                        }
                    }
                    if ui.small_button("x").clicked() {
                        let _ = ai;
                        a.keys.clear();
                        dirty = true;
                    }
                });
            }
            e.automation.retain(|a| !a.keys.is_empty());
        }
    }

    if dirty {
        EventEditorAction::Changed
    } else {
        EventEditorAction::None
    }
}

pub fn ensure_default_track(ev: &mut EventEntry) {
    if ev.tracks.is_empty() {
        let mut track = crate::project::TrackEntry::new("Audio 1");
        if let Some(ai) = ev.asset_index {
            let mut inst = crate::project::InstrumentEntry::new("clip");
            inst.asset_index = Some(ai);
            inst.looping = ev.looping;
            track.instruments.push(inst);
        }
        ev.tracks.push(track);
    }
}
