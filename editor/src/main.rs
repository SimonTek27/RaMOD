//! RaMOD Event Editor — multi-format import, DnD, audible preview, bank build, multi-track.

mod audio_bridge;
mod live_client;
mod project;
mod event_editor;
mod waveform_panel;
mod profiler_panel;

use audio_bridge::AudioBridge;
use eframe::egui;
use event_editor::{ensure_default_track, show_event_editor, EventEditorAction, EventEditorState};
use live_client::EditorLiveClient;
use profiler_panel::{ProfilerAction, ProfilerPanel};
use project::Project;
use std::path::PathBuf;
use waveform_panel::{WaveformAction, WaveformPanel};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("RaMOD — Event Editor")
            .with_drag_and_drop(true),
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
    selected_event: Option<usize>,
    selected_asset: Option<usize>,
    live_update: bool,
    status_msg: String,
    audio: Option<AudioBridge>,
    intensity: f32,
    live_client: Option<EditorLiveClient>,
    waveform: WaveformPanel,
    profiler: ProfilerPanel,
    asset_import_path: String,
    event_editor: EventEditorState,
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
            selected_event: Some(0),
            selected_asset: Some(0),
            live_update: false,
            status_msg: "Ready — multi-track Event Editor".into(),
            audio,
            intensity: 1.0,
            live_client: None,
            waveform,
            profiler: ProfilerPanel::default(),
            asset_import_path: String::new(),
            event_editor: EventEditorState::default(),
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

    fn register_asset(&mut self, path: PathBuf) {
        let idx = self.project.add_asset_file(&path);
        self.selected_asset = Some(idx);
        self.browser_tab = BrowserTab::Assets;
        self.status_msg = format!("Asset added: {}", path.display());
    }

    fn build_bank(&mut self) {
        let bank = self
            .project
            .banks
            .first()
            .map(|b| b.name.clone())
            .unwrap_or_else(|| "Master".into());
        let out = PathBuf::from("build");
        match self.project.export_bank_build(&bank, &out) {
            Ok(path) => {
                self.status_msg = format!("Built {}", path.display());
            }
            Err(e) => self.status_msg = format!("Build failed: {e}"),
        }
    }

    fn handle_file_drops(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !ramod_waveform::is_supported_extension(&ext) {
                self.status_msg = format!("Unsupported drop: {}", path.display());
                continue;
            }
            if self.waveform.import_wav(&path) {
                self.register_asset(path.clone());
                self.waveform.show = true;
                self.status_msg = format!("Dropped + imported {}", path.display());
            } else {
                self.status_msg = self.waveform.status.clone();
            }
        }
    }
}

impl eframe::App for RamodEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_file_drops(ctx);

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        self.save_project();
                    }
                    if ui.button("Build bank").clicked() {
                        self.build_bank();
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Window", |ui| {
                    ui.checkbox(&mut self.waveform.show, "Waveform Editor");
                    ui.checkbox(&mut self.profiler.show, "Profiler");
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
                }
                ui.separator();
                let (peak, rms) = self
                    .audio
                    .as_ref()
                    .map(|a| a.master_levels())
                    .unwrap_or((0.0, 0.0));
                ui.label(format!("Peak {peak:.2}  RMS {rms:.2}"));
            });
        });

        if self.waveform.show {
            egui::TopBottomPanel::bottom("waveform")
                .default_height(240.0)
                .resizable(true)
                .show(ctx, |ui| {
                    match self.waveform.ui(ui) {
                        WaveformAction::ImportedAndRegister(p) => self.register_asset(p),
                        WaveformAction::Imported(p) => {
                            self.status_msg = format!("Loaded {}", p.display());
                        }
                        WaveformAction::PlayPreview {
                            samples,
                            channels,
                            sample_rate,
                        } => {
                            if let Some(a) = self.audio.as_mut() {
                                match a.play_pcm(samples, channels, sample_rate) {
                                    Ok(()) => {
                                        self.transport = TransportState::Playing;
                                        self.status_msg = "Waveform preview".into();
                                    }
                                    Err(e) => {
                                        self.status_msg = format!("Preview failed: {e}")
                                    }
                                }
                            }
                        }
                        WaveformAction::StopPreview => {
                            if let Some(a) = self.audio.as_mut() {
                                a.stop();
                            }
                            self.transport = TransportState::Stopped;
                        }
                        WaveformAction::None => {}
                    }
                });
        }

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(&self.status_msg);
        });

        egui::SidePanel::left("browser")
            .default_width(280.0)
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
                match self.browser_tab {
                    BrowserTab::Events => {
                        for (i, ev) in self.project.events.iter().enumerate() {
                            if ui
                                .selectable_label(self.selected_event == Some(i), &ev.path)
                                .clicked()
                            {
                                self.selected_event = Some(i);
                            }
                        }
                    }
                    BrowserTab::Banks => {
                        for b in &self.project.banks {
                            ui.label(&b.name);
                        }
                    }
                    BrowserTab::Assets => {
                        for (i, a) in self.project.assets.iter().enumerate() {
                            if ui
                                .selectable_label(self.selected_asset == Some(i), &a.name)
                                .clicked()
                            {
                                self.selected_asset = Some(i);
                            }
                        }
                    }
                }
            });

        if self.profiler.show {
            egui::SidePanel::right("profiler")
                .default_width(280.0)
                .resizable(true)
                .show(ctx, |ui| {
                    let snap = self
                        .audio
                        .as_ref()
                        .map(|a| a.profiler_snapshot())
                        .unwrap_or_default();
                    match self.profiler.ui(ui, &snap) {
                        ProfilerAction::Clear => {
                            if let Some(a) = self.audio.as_mut() {
                                a.profiler_clear();
                            }
                            self.status_msg = "Profiler cleared".into();
                        }
                        ProfilerAction::ExportNdjson => {
                            if let Some(a) = self.audio.as_ref() {
                                let ndjson = a.profiler_ndjson();
                                let path = PathBuf::from("profiler_session.ndjson");
                                match std::fs::write(&path, &ndjson) {
                                    Ok(()) => {
                                        self.status_msg =
                                            format!("Exported → {}", path.display());
                                    }
                                    Err(e) => {
                                        self.status_msg = format!("Export failed: {e}");
                                    }
                                }
                            }
                        }
                        ProfilerAction::ToggleMute(id) => {
                            if let Some(a) = self.audio.as_mut() {
                                a.toggle_bus_mute(id);
                            }
                        }
                        ProfilerAction::ToggleSolo(id) => {
                            if let Some(a) = self.audio.as_mut() {
                                a.toggle_bus_solo(id);
                            }
                        }
                        ProfilerAction::SetVolume(id, db) => {
                            if let Some(a) = self.audio.as_mut() {
                                a.set_bus_volume_db(id, db);
                            }
                        }
                        ProfilerAction::CaptureBase => {
                            if let Some(a) = self.audio.as_mut() {
                                a.capture_snapshot_base();
                            }
                            self.status_msg = "Snapshot base captured".into();
                        }
                        ProfilerAction::SaveSnapshot => {
                            let name = self.profiler.snap_name.clone();
                            if let Some(a) = self.audio.as_mut() {
                                a.capture_named_snapshot(&name);
                            }
                            self.status_msg = format!("Snapshot saved: {name}");
                        }
                        ProfilerAction::TriggerSnapshot { name, weight } => {
                            if let Some(a) = self.audio.as_mut() {
                                a.trigger_snapshot(&name, weight);
                            }
                            self.status_msg =
                                format!("Snapshot '{name}' @ {:.0}%", weight * 100.0);
                        }
                        ProfilerAction::ClearSnapshot => {
                            if let Some(a) = self.audio.as_mut() {
                                a.clear_snapshot();
                            }
                            self.status_msg = "Snapshot cleared (base restored)".into();
                        }
                        ProfilerAction::SetEmitterPos { x, z } => {
                            if let Some(a) = self.audio.as_mut() {
                                a.set_emitter_xz(x, z);
                            }
                        }
                        ProfilerAction::SetListenerPos { x, z } => {
                            if let Some(a) = self.audio.as_mut() {
                                a.set_listener_xz(x, z);
                            }
                        }
                        ProfilerAction::AddLowPass(id) => {
                            if let Some(a) = self.audio.as_mut() {
                                a.add_bus_lowpass(id);
                            }
                            self.status_msg = "Bus FX: LowPass".into();
                        }
                        ProfilerAction::AddHighPass(id) => {
                            if let Some(a) = self.audio.as_mut() {
                                a.add_bus_highpass(id);
                            }
                            self.status_msg = "Bus FX: HighPass".into();
                        }
                        ProfilerAction::AddCompressor(id) => {
                            if let Some(a) = self.audio.as_mut() {
                                a.add_bus_compressor(id);
                            }
                            self.status_msg = "Bus FX: Compressor".into();
                        }
                        ProfilerAction::AddReverb(id) => {
                            if let Some(a) = self.audio.as_mut() {
                                a.add_bus_reverb(id);
                            }
                            self.status_msg = "Bus FX: Reverb".into();
                        }
                        ProfilerAction::ClearInserts(id) => {
                            if let Some(a) = self.audio.as_mut() {
                                a.clear_bus_inserts(id);
                            }
                            self.status_msg = "Bus inserts cleared".into();
                        }
                        ProfilerAction::UpdateInsert(info) => {
                            if let Some(a) = self.audio.as_mut() {
                                a.set_bus_insert_info(&info);
                            }
                        }
                        ProfilerAction::RemoveInsert { bus_id, index } => {
                            if let Some(a) = self.audio.as_mut() {
                                a.remove_bus_insert(bus_id, index);
                            }
                            self.status_msg = "Insert removed".into();
                        }
                        ProfilerAction::None => {}
                    }
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ei) = self.selected_event {
                if let Some(ev) = self.project.events.get_mut(ei) {
                    ensure_default_track(ev);
                }
                match show_event_editor(ui, &mut self.project, ei, &mut self.event_editor) {
                    EventEditorAction::Changed => {
                        self.status_msg = "Event timeline updated".into();
                        if let Some(a) = self.audio.as_mut() {
                            a.sync_project(&self.project);
                        }
                    }
                    EventEditorAction::None => {}
                }
            } else {
                ui.heading("Overview");
            }
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
