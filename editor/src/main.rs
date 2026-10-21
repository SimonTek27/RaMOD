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

    fn assign_selected_asset_to_event(&mut self) {
        let (Some(ei), Some(ai)) = (self.selected_event, self.selected_asset) else {
            self.status_msg = "Select an event and an asset first".into();
            return;
        };
        match self.project.assign_asset_to_event(ei, ai) {
            Ok(()) => {
                let ev = &self.project.events[ei].path;
                let asset = &self.project.assets[ai].name;
                self.status_msg = format!("Assigned {asset} → {ev}");
                if let Some(a) = self.audio.as_mut() {
                    a.sync_project(&self.project);
                }
            }
            Err(e) => self.status_msg = e,
        }
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
                let fsb = out.join(&bank).join(format!("{bank}.fsb"));
                let fsb_note = if fsb.exists() {
                    format!(" + {}", fsb.display())
                } else {
                    " (FSB skipped — restore parser / add file assets)".into()
                };
                self.status_msg = format!("Built {}{}", path.display(), fsb_note);
            }
            Err(e) => self.status_msg = format!("Build failed: {e}"),
        }
    }

    fn open_asset_in_waveform(&mut self, index: usize) {
        if let Some(asset) = self.project.assets.get(index) {
            if let Some(ref sp) = asset.source_path {
                let p = PathBuf::from(sp);
                if self.waveform.import_wav(&p) {
                    self.waveform.show = true;
                    self.status_msg = format!("Opened {}", sp);
                } else {
                    self.status_msg = self.waveform.status.clone();
                }
            } else if let Some(freq) = asset.demo_freq_hz {
                let rate = 48_000u32;
                let frames = rate as usize / 2;
                let samples: Vec<f32> = (0..frames)
                    .map(|i| {
                        let t = i as f32 / rate as f32;
                        (2.0 * std::f32::consts::PI * freq * t).sin() * 0.4
                    })
                    .collect();
                if let Ok(mut w) = ramod_waveform::WaveBuffer::new(samples, 1, rate) {
                    w.select_all();
                    self.waveform.wave = Some(w);
                    self.waveform.show = true;
                }
            }
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
                self.status_msg = format!(
                    "Unsupported drop: {} (wav/mp3/ogg/flac)",
                    path.display()
                );
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

    fn drop_hover_hint(&self, ctx: &egui::Context) -> bool {
        ctx.input(|i| !i.raw.hovered_files.is_empty())
    }
}

impl eframe::App for RamodEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_file_drops(ctx);

        if self.drop_hover_hint(ctx) {
            egui::Area::new(egui::Id::new("drop_overlay"))
                .fixed_pos(egui::pos2(0.0, 0.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let screen = ctx.screen_rect();
                    ui.allocate_ui_at_rect(screen, |ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                        let painter = ui.painter();
                        painter.rect_filled(
                            rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(20, 40, 80, 160),
                        );
                        painter.text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "Drop WAV / MP3 / OGG / FLAC to import",
                            egui::FontId::proportional(28.0),
                            egui::Color32::WHITE,
                        );
                    });
                });
        }

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
                    if let (Some(c), Some(path)) =
                        (self.live_client.as_mut(), self.selected_path())
                    {
                        let _ = c.set_param(path, "Intensity", self.intensity);
                    }
                }
                ui.separator();
                let (peak, rms) = self
                    .audio
                    .as_ref()
                    .map(|a| a.master_levels())
                    .unwrap_or((0.0, 0.0));
                ui.label(format!("Peak {peak:.2}  RMS {rms:.2}"));
                let meter_w = 80.0;
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(meter_w, 14.0), egui::Sense::hover());
                let painter = ui.painter();
                painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(30, 30, 30));
                let fill = (peak.clamp(0.0, 1.0)) * meter_w;
                if fill > 0.5 {
                    painter.rect_filled(
                        egui::Rect::from_min_size(rect.min, egui::vec2(fill, 14.0)),
                        2.0,
                        if peak > 0.9 {
                            egui::Color32::from_rgb(220, 60, 60)
                        } else {
                            egui::Color32::from_rgb(60, 180, 90)
                        },
                    );
                }
                if let Some(a) = self.audio.as_ref() {
                    let marks = a.last_fired_markers();
                    if let Some(last) = marks.last() {
                        ui.label(format!("marker: {last}"));
                    }
                }
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
                            self.status_msg = "Preview stopped".into();
                        }
                        WaveformAction::None => {}
                    }
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
                                self.status_msg = "Live Update On".into();
                            }
                            Err(err) => {
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
                        if ui.button("Assign selected asset → event").clicked() {
                            self.assign_selected_asset_to_event();
                        }
                        if ui.button("+ Event").clicked() {
                            self.project.add_event(format!(
                                "event:/sfx/new_{}",
                                self.project.events.len()
                            ));
                        }
                        ui.separator();
                        for (i, ev) in self.project.events.iter().enumerate() {
                            let asset_label = ev
                                .asset_index
                                .and_then(|ai| self.project.assets.get(ai))
                                .map(|a| a.name.as_str())
                                .unwrap_or("—");
                            let label = format!("{}  [{}]", ev.path, asset_label);
                            if ui
                                .selectable_label(self.selected_event == Some(i), label)
                                .clicked()
                            {
                                self.selected_event = Some(i);
                            }
                        }
                    }
                    BrowserTab::Banks => {
                        if ui.button("Build bank").clicked() {
                            self.build_bank();
                        }
                        ui.separator();
                        for b in &self.project.banks {
                            ui.label(&b.name);
                            let n = self
                                .project
                                .events
                                .iter()
                                .filter(|e| e.bank == b.name)
                                .count();
                            ui.weak(format!("  {n} events"));
                        }
                    }
                    BrowserTab::Assets => {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.asset_import_path)
                                    .desired_width(160.0)
                                    .hint_text("file.wav / .mp3 / .ogg"),
                            );
                            if ui.button("Import").clicked() {
                                let p = PathBuf::from(self.asset_import_path.trim());
                                if p.exists() {
                                    if self.waveform.import_wav(&p) {
                                        self.register_asset(p);
                                        self.waveform.show = true;
                                    } else {
                                        self.status_msg = self.waveform.status.clone();
                                    }
                                } else {
                                    self.status_msg = format!("Not found: {}", p.display());
                                }
                            }
                        });
                        ui.label("or drag & drop wav/mp3/ogg/flac");
                        ui.separator();
                        for (i, a) in self.project.assets.iter().enumerate() {
                            let label = if let Some(ref sp) = a.source_path {
                                format!("{} ({})", a.name, sp)
                            } else if let Some(f) = a.demo_freq_hz {
                                format!("{} (demo {} Hz)", a.name, f)
                            } else {
                                a.name.clone()
                            };
                            if ui
                                .selectable_label(self.selected_asset == Some(i), label)
                                .clicked()
                            {
                                self.selected_asset = Some(i);
                            }
                            if self.selected_asset == Some(i)
                                && ui.small_button("Open in Waveform").clicked()
                            {
                                self.open_asset_in_waveform(i);
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
                ui.weak("Select an event — multi-track timeline appears here");
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
