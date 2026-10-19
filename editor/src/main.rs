//! RaMOD Event Editor — S3 + S6 Live Update + S9 Waveform panel.

mod audio_bridge;
mod live_client;
mod project;
mod waveform_panel;

use audio_bridge::AudioBridge;
use eframe::egui;
use live_client::EditorLiveClient;
use project::Project;
use std::path::PathBuf;
use waveform_panel::WaveformPanel;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
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
    live_client: Option<EditorLiveClient>,
    waveform: WaveformPanel,
}

impl RamodEditorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_dark_theme(&cc.egui_ctx);
        let project = Project::default();
        let mut audio = AudioBridge::new().ok();
        if let Some(a) = audio.as_mut() {
            a.sync_project(&project);
        }
        let mut waveform = WaveformPanel::default();
        waveform.load_demo_tone();
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
            live_client: None,
            waveform,
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
}

impl eframe::App for RamodEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        self.save_project();
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Window", |ui| {
                    if ui
                        .checkbox(&mut self.waveform.show, "Waveform Editor")
                        .changed()
                    {
                        // toggle only
                    }
                });
                ui.label(format!("  {}", self.project.name));
            });
        });

        egui::TopBottomPanel::top("transport").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("▶").clicked() {
                    self.play_selected();
                }
                if ui.button("■").clicked() {
                    self.stop_playback();
                }
                ui.separator();
                ui.label("Intensity");
                if ui
                    .add(egui::Slider::new(&mut self.intensity, 0.0..=2.0))
                    .changed()
                {
                    if let Some(a) = self.audio.as_mut() {
                        a.set_parameter("Intensity", self.intensity);
                    }
                    if let Some(path) = self.selected_path().map(|s| s.to_string()) {
                        if let Some(c) = self.live_client.as_mut() {
                            let _ = c.set_param(&path, "Intensity", self.intensity);
                        }
                    }
                }
            });
        });

        // Waveform bottom panel (S9)
        if self.waveform.show {
            egui::TopBottomPanel::bottom("waveform")
                .default_height(220.0)
                .resizable(true)
                .show(ctx, |ui| {
                    self.waveform.ui(ui);
                });
        }

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let live = if self.live_update {
                    "Live Update On"
                } else {
                    "Live Update Off"
                };
                if ui.selectable_label(self.live_update, live).clicked() {
                    if self.live_update {
                        self.live_client = None;
                        self.live_update = false;
                        self.status_msg = "Live Update Off".into();
                    } else {
                        match EditorLiveClient::connect(None) {
                            Ok(c) => {
                                self.live_client = Some(c);
                                self.live_update = true;
                                self.status_msg = "Live Update On — 127.0.0.1:17420".into();
                            }
                            Err(err) => {
                                self.live_update = false;
                                self.status_msg = format!("Live Update failed: {err}");
                            }
                        }
                    }
                }
                ui.separator();
                ui.label(&self.status_msg);
            });
        });

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
                ui.separator();
                if self.browser_tab == BrowserTab::Events {
                    for (i, ev) in self.project.events.iter().enumerate() {
                        if ui
                            .selectable_label(self.selected_event == Some(i), &ev.path)
                            .clicked()
                        {
                            self.selected_event = Some(i);
                        }
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Overview");
            if let Some(p) = self.selected_path() {
                ui.label(p);
            }
            ui.weak("Waveform panel: Window → Waveform Editor (bottom)");
        });

        if matches!(self.transport, TransportState::Playing) {
            let dt = ctx.input(|i| i.stable_dt);
            self.time_secs += dt;
            if let Some(a) = self.audio.as_mut() {
                a.pump(dt);
            }
            ctx.request_repaint();
        }
    }
}

fn apply_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(45, 45, 45);
    ctx.set_visuals(visuals);
}
