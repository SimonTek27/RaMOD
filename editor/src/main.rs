//! RaMOD Event Editor — S3 vertical slice.
//!
//! Layout matches FMOD Studio Event Editor (docs/UI_REFERENCE.md).
//! Play transport drives `ramod_runtime`.

mod audio_bridge;
mod project;

use audio_bridge::AudioBridge;
use eframe::egui;
use project::Project;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 800.0])
            .with_title("RaMOD — Event Editor"),
        ..Default::default()
    };
    eframe::run_native(
        "RaMOD Event Editor",
        options,
        Box::new(|cc| Ok(Box::new(RamodEditorApp::new(cc)))),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserTab {
    Events,
    Banks,
    Assets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportState {
    Stopped,
    Playing,
    Paused,
}

struct RamodEditorApp {
    project: Project,
    browser_tab: BrowserTab,
    transport: TransportState,
    time_secs: f32,
    search: String,
    selected_event: Option<usize>,
    live_update: bool,
    spatial: bool,
    platform: String,
    status_msg: String,
    audio: Option<AudioBridge>,
    intensity: f32,
    new_event_name: String,
}

impl RamodEditorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_dark_theme(&cc.egui_ctx);
        let project = Project::default();
        let mut audio = AudioBridge::new().ok();
        if let Some(a) = audio.as_mut() {
            a.sync_project(&project);
        }
        Self {
            project,
            browser_tab: BrowserTab::Events,
            transport: TransportState::Stopped,
            time_secs: 0.0,
            search: String::new(),
            selected_event: Some(0),
            live_update: false,
            spatial: false,
            platform: "Desktop".into(),
            status_msg: "Ready".into(),
            audio,
            intensity: 1.0,
            new_event_name: "event:/sfx/new".into(),
        }
    }

    fn selected_path(&self) -> Option<&str> {
        self.selected_event
            .and_then(|i| self.project.events.get(i))
            .map(|e| e.path.as_str())
    }

    fn play_selected(&mut self) {
        let Some(path) = self.selected_path().map(|s| s.to_string()) else {
            self.status_msg = "No event selected".into();
            return;
        };
        if let Some(audio) = self.audio.as_mut() {
            audio.sync_project(&self.project);
            match audio.play_event(&path) {
                Ok(()) => {
                    audio.set_parameter("Intensity", self.intensity);
                    self.transport = TransportState::Playing;
                    self.time_secs = 0.0;
                    self.status_msg = format!("Playing {path}");
                }
                Err(e) => {
                    self.status_msg = format!("Play failed: {e}");
                    self.transport = TransportState::Stopped;
                }
            }
        } else {
            self.status_msg = "Audio bridge unavailable".into();
        }
    }

    fn stop_playback(&mut self) {
        if let Some(audio) = self.audio.as_mut() {
            audio.stop();
        }
        self.transport = TransportState::Stopped;
        self.status_msg = "Stopped".into();
    }

    fn save_project(&mut self) {
        let path = self
            .project
            .path
            .clone()
            .unwrap_or_else(|| PathBuf::from("project.ramod.json"));
        match self.project.save(&path) {
            Ok(()) => {
                self.project.path = Some(path.clone());
                self.status_msg = format!("Saved {}", path.display());
            }
            Err(e) => self.status_msg = format!("Save failed: {e}"),
        }
    }

    fn load_project(&mut self, path: PathBuf) {
        match Project::load(&path) {
            Ok(p) => {
                self.project = p;
                self.selected_event = if self.project.events.is_empty() {
                    None
                } else {
                    Some(0)
                };
                if let Some(a) = self.audio.as_mut() {
                    a.sync_project(&self.project);
                }
                self.status_msg = format!("Loaded {}", path.display());
            }
            Err(e) => self.status_msg = format!("Load failed: {e}"),
        }
    }
}

impl eframe::App for RamodEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ---- Menu bar -------------------------------------------------------
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        self.project = Project::default();
                        self.selected_event = Some(0);
                        if let Some(a) = self.audio.as_mut() {
                            a.sync_project(&self.project);
                        }
                        self.status_msg = "New project".into();
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.save_project();
                        ui.close_menu();
                    }
                    if ui.button("Save as project.ramod.json").clicked() {
                        self.project.path = Some(PathBuf::from("project.ramod.json"));
                        self.save_project();
                        ui.close_menu();
                    }
                    if ui.button("Open project.ramod.json").clicked() {
                        self.load_project(PathBuf::from("project.ramod.json"));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Window", |ui| {
                    let _ = ui.button("Event Editor");
                    let _ = ui.button("Mixer (soon)");
                });
                ui.label(format!("  {}", self.project.name));
            });
        });

        // ---- Transport ------------------------------------------------------
        egui::TopBottomPanel::top("transport").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("▶").on_hover_text("Play").clicked() {
                    self.play_selected();
                }
                if ui.button("■").on_hover_text("Stop").clicked() {
                    self.stop_playback();
                }
                if ui.button("❚❚").on_hover_text("Pause").clicked() {
                    if matches!(self.transport, TransportState::Playing) {
                        self.transport = TransportState::Paused;
                    }
                }
                ui.separator();
                let state = match self.transport {
                    TransportState::Stopped => "STOPPED",
                    TransportState::Playing => "PLAYING",
                    TransportState::Paused => "PAUSED",
                };
                let mins = (self.time_secs as u32) / 60;
                let secs = self.time_secs % 60.0;
                ui.monospace(format!("{state}  {mins:02}:{secs:06.3}"));
                ui.separator();
                ui.label("Intensity");
                if ui
                    .add(egui::Slider::new(&mut self.intensity, 0.0..=2.0))
                    .changed()
                {
                    if let Some(a) = self.audio.as_mut() {
                        a.set_parameter("Intensity", self.intensity);
                    }
                }
                ui.checkbox(&mut self.spatial, "3D");
                ui.label(&self.platform);
            });
        });

        // ---- Status bar -----------------------------------------------------
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let live = if self.live_update {
                    "Live Update On"
                } else {
                    "Live Update Off"
                };
                if ui.selectable_label(self.live_update, live).clicked() {
                    self.live_update = !self.live_update;
                    self.status_msg = if self.live_update {
                        "Live Update On (protocol stub)".into()
                    } else {
                        "Live Update Off".into()
                    };
                }
                ui.separator();
                ui.label(&self.status_msg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(format!(
                        "events={} banks={} assets={}",
                        self.project.events.len(),
                        self.project.banks.len(),
                        self.project.assets.len()
                    ));
                });
            });
        });

        // ---- Left browser ---------------------------------------------------
        egui::SidePanel::left("browser")
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (tab, label) in [
                        (BrowserTab::Events, "Events"),
                        (BrowserTab::Banks, "Banks"),
                        (BrowserTab::Assets, "Assets"),
                    ] {
                        if ui
                            .selectable_label(self.browser_tab == tab, label)
                            .clicked()
                        {
                            self.browser_tab = tab;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Search…")
                            .desired_width(180.0),
                    );
                });
                ui.separator();

                match self.browser_tab {
                    BrowserTab::Events => {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.new_event_name)
                                    .desired_width(150.0),
                            );
                            if ui.button("+").on_hover_text("Add event").clicked() {
                                let name = self.new_event_name.clone();
                                self.project.add_event(name);
                                self.selected_event = Some(self.project.events.len() - 1);
                                if let Some(a) = self.audio.as_mut() {
                                    a.sync_project(&self.project);
                                }
                                self.status_msg = "Event added".into();
                            }
                        });
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (i, ev) in self.project.events.iter().enumerate() {
                                if !self.search.is_empty()
                                    && !ev.path.to_lowercase().contains(&self.search.to_lowercase())
                                {
                                    continue;
                                }
                                let selected = self.selected_event == Some(i);
                                if ui.selectable_label(selected, &ev.path).clicked() {
                                    self.selected_event = Some(i);
                                }
                            }
                        });
                    }
                    BrowserTab::Banks => {
                        for b in &self.project.banks {
                            ui.label(&b.name);
                        }
                        if ui.button("+ Bank").clicked() {
                            self.project.banks.push(project::BankEntry {
                                name: format!("Bank{}", self.project.banks.len() + 1),
                            });
                        }
                    }
                    BrowserTab::Assets => {
                        for a in &self.project.assets {
                            let extra = a
                                .demo_freq_hz
                                .map(|f| format!(" ({f} Hz)"))
                                .unwrap_or_default();
                            ui.label(format!("{}{extra}", a.name));
                        }
                        if ui.button("+ Demo tone 880 Hz").clicked() {
                            self.project.add_asset_demo(
                                format!("tone_{}", self.project.assets.len()),
                                880.0,
                            );
                        }
                    }
                }
            });

        // ---- Right deck (S3.4 stub) -----------------------------------------
        egui::SidePanel::right("deck")
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Deck");
                ui.weak("Parameter curves (stub)");
                ui.separator();
                if let Some(i) = self.selected_event {
                    if let Some(ev) = self.project.events.get(i) {
                        ui.label(&ev.path);
                        ui.checkbox(
                            &mut self.project.events[i].looping,
                            "Loop",
                        );
                        ui.checkbox(&mut self.project.events[i].is_3d, "3D");
                        ui.add(
                            egui::Slider::new(&mut self.project.events[i].priority, 0..=1000)
                                .text("Priority"),
                        );
                        for (name, val) in &ev.parameters {
                            ui.label(format!("{name} = {val:.2}"));
                        }
                        ui.separator();
                        ui.weak("Automation lanes — Phase S5");
                        // Fake curve canvas
                        let (rect, _r) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 80.0),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter();
                        painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(24, 24, 24));
                        let y = rect.center().y;
                        painter.line_segment(
                            [egui::pos2(rect.left() + 4.0, y), egui::pos2(rect.right() - 4.0, y)],
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 140, 200)),
                        );
                    }
                } else {
                    ui.weak("Select an event");
                }
            });

        // ---- Center: Overview -----------------------------------------------
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Overview");
                if let Some(path) = self.selected_path() {
                    ui.label(path);
                }
            });
            ui.separator();

            let canvas_h = (ui.available_height() - 72.0).max(120.0);
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(28, 28, 28))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50)))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_min_height(canvas_h);
                    if let Some(i) = self.selected_event {
                        if let Some(ev) = self.project.events.get(i) {
                            ui.label("Tracks");
                            ui.horizontal(|ui| {
                                ui.label("■ Audio track");
                                if let Some(ai) = ev.asset_index {
                                    if let Some(a) = self.project.assets.get(ai) {
                                        ui.label(format!("Instrument: {}", a.name));
                                    }
                                } else {
                                    ui.weak("No instrument");
                                }
                            });
                            ui.add_space(12.0);
                            ui.weak("Timeline scrubber — S4/S5");
                            let (rect, _r) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 48.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                rect,
                                2.0,
                                egui::Color32::from_rgb(35, 35, 40),
                            );
                            if matches!(self.transport, TransportState::Playing) {
                                let t = (self.time_secs / 2.0).fract();
                                let x = rect.left() + rect.width() * t;
                                ui.painter().line_segment(
                                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                                    egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 180, 80)),
                                );
                            }
                        }
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.weak("Create or select an event");
                        });
                    }
                });

            ui.add_space(4.0);
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(18, 18, 18))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_min_height(48.0);
                    ui.weak("Properties — select instrument in Overview");
                });
        });

        // Transport clock + audio pump
        if matches!(self.transport, TransportState::Playing) {
            let dt = ctx.input(|i| i.stable_dt);
            self.time_secs += dt;
            if let Some(a) = self.audio.as_mut() {
                a.pump(dt);
                if !a.is_playing() && self.time_secs > 0.6 {
                    self.transport = TransportState::Stopped;
                    self.status_msg = "Playback finished".into();
                }
            }
            ctx.request_repaint();
        }
    }
}

fn apply_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(45, 45, 45);
    visuals.window_fill = egui::Color32::from_rgb(40, 40, 40);
    visuals.extreme_bg_color = egui::Color32::from_rgb(28, 28, 28);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(55, 55, 55);
    visuals.selection.bg_fill = egui::Color32::from_rgb(70, 90, 120);
    ctx.set_visuals(visuals);
}
