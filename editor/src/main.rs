//! RaMOD Event Editor — UI chrome matching the FMOD Studio Event Editor layout.
//!
//! Reference: docs/UI_REFERENCE.md
//!
//! This is a structural scaffold only (Phase 3 preview). No timeline logic yet.

use eframe::egui;

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
    browser_tab: BrowserTab,
    transport: TransportState,
    time_secs: f32,
    search: String,
    selected_event: Option<String>,
    live_update: bool,
    spatial: bool,
    platform: String,
    locale: String,
    project_name: String,
}

impl RamodEditorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_dark_theme(&cc.egui_ctx);
        Self {
            browser_tab: BrowserTab::Events,
            transport: TransportState::Stopped,
            time_secs: 0.0,
            search: String::new(),
            selected_event: None,
            live_update: false,
            spatial: false,
            platform: "Desktop".into(),
            locale: String::new(),
            project_name: "Untitled — Event Editor".into(),
        }
    }
}

impl eframe::App for RamodEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ---- Menu bar -------------------------------------------------------
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    let _ = ui.button("New Project");
                    let _ = ui.button("Open…");
                    let _ = ui.button("Save");
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    let _ = ui.button("Undo");
                    let _ = ui.button("Redo");
                });
                ui.menu_button("Create", |ui| {
                    if ui.button("New Event").clicked() {
                        self.selected_event = Some("event:/new_event".into());
                    }
                    let _ = ui.button("New Folder");
                });
                ui.menu_button("View", |_| {});
                ui.menu_button("Window", |_| {});
                ui.menu_button("Scripts", |_| {});

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(&self.project_name).weak());
                });
            });
        });

        // ---- Status bar -----------------------------------------------------
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let live = if self.live_update {
                    "Live Update On"
                } else {
                    "Live Update Off"
                };
                if ui.selectable_label(self.live_update, live).clicked() {
                    self.live_update = !self.live_update;
                }
                ui.separator();
                ui.label("Platform");
                egui::ComboBox::from_id_salt("platform")
                    .selected_text(&self.platform)
                    .show_ui(ui, |ui| {
                        for p in ["Desktop", "Console", "Mobile"] {
                            ui.selectable_value(&mut self.platform, p.into(), p);
                        }
                    });
                ui.separator();
                ui.label("Locale");
                ui.add(
                    egui::TextEdit::singleline(&mut self.locale)
                        .desired_width(80.0)
                        .hint_text("—"),
                );
            });
        });

        // ---- Left browser ---------------------------------------------------
        egui::SidePanel::left("browser")
            .default_width(260.0)
            .resizable(true)
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
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Search…")
                            .desired_width(f32::INFINITY),
                    );
                });
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        match self.browser_tab {
                            BrowserTab::Events => {
                                if let Some(name) = &self.selected_event {
                                    let _ = ui.selectable_label(true, name);
                                } else {
                                    ui.weak("(no events — Create → New Event)");
                                }
                            }
                            BrowserTab::Banks => {
                                ui.weak("(no banks loaded)");
                            }
                            BrowserTab::Assets => {
                                ui.weak("(no assets)");
                            }
                        }
                    });

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("New Event").clicked() {
                            self.selected_event = Some("event:/new_event".into());
                            self.browser_tab = BrowserTab::Events;
                        }
                        let _ = ui.button("New Folder");
                        let _ = ui.button("Flatten");
                    });
                    ui.separator();
                    ui.add_space(4.0);
                    if let Some(name) = &self.selected_event {
                        ui.label("Properties");
                        ui.label(egui::RichText::new(name).monospace().small());
                    } else {
                        ui.weak("Select an item to view its properties");
                    }
                });
            });

        // ---- Right overview -------------------------------------------------
        egui::SidePanel::right("overview")
            .default_width(220.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Overview");
                ui.separator();
                if self.selected_event.is_some() {
                    ui.label("3D");
                    ui.checkbox(&mut self.spatial, "Spatial");
                    ui.label("Parameters");
                    ui.weak("(none)");
                    ui.label("Buses");
                    ui.weak("Master");
                } else {
                    ui.weak("Open an event to see overview");
                }
            });

        // ---- Center: transport + canvas + inspector -------------------------
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_label(true, "Editor");
                let _ = ui.button("+");
            });
            ui.separator();

            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new("⏹")
                            .selected(matches!(self.transport, TransportState::Stopped)),
                    )
                    .clicked()
                {
                    self.transport = TransportState::Stopped;
                    self.time_secs = 0.0;
                }

                let play_text =
                    egui::RichText::new("▶").color(egui::Color32::from_rgb(220, 180, 40));
                if ui
                    .add(
                        egui::Button::new(play_text)
                            .selected(matches!(self.transport, TransportState::Playing)),
                    )
                    .clicked()
                {
                    self.transport = TransportState::Playing;
                }

                if ui
                    .add(
                        egui::Button::new("⏸")
                            .selected(matches!(self.transport, TransportState::Paused)),
                    )
                    .clicked()
                {
                    self.transport = TransportState::Paused;
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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let _ = ui.small_button("▦");
                    let _ = ui.small_button("☰");
                });
            });

            ui.add_space(6.0);

            let canvas_h = (ui.available_height() - 80.0).max(120.0);
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(28, 28, 28))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50)))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_min_height(canvas_h);
                    ui.set_width(ui.available_width());
                    if self.selected_event.is_none() {
                        ui.centered_and_justified(|ui| {
                            ui.weak("Create or select an event to edit tracks and instruments");
                        });
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.weak("Timeline / tracks — Phase 3 (not implemented yet)");
                        });
                    }
                });

            ui.add_space(4.0);

            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(18, 18, 18))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_min_height(64.0);
                    ui.weak("Select a track or instrument to view its properties");
                });
        });

        if matches!(self.transport, TransportState::Playing) {
            self.time_secs += ctx.input(|i| i.stable_dt);
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
