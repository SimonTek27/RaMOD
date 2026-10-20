//! RamodPlugin + systems + bank registration.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use ramod_runtime::{
    Attributes3d, Attenuation, EventDescription, Runtime, RuntimeConfig, Vec3 as RVec3,
};
use ramod_waveform::WaveBuffer;

use crate::assets::{BankEventAsset, RamodBankAsset, RamodBankLoader};
use crate::components::{AudioEmitter, AudioListener, PlayOnSpawn, Velocity};

/// Global audio runtime resource.
#[derive(Resource)]
pub struct RamodAudio {
    pub runtime: Runtime,
    /// Event paths registered from banks (for debug).
    pub registered_events: Vec<String>,
    dt_accum: f32,
}

impl RamodAudio {
    pub fn new(open_device: bool) -> Result<Self, String> {
        let runtime = Runtime::new(RuntimeConfig {
            open_device,
            sample_rate: 48_000,
            channels: 2,
            max_voices: 64,
        })
        .map_err(|e| e.to_string())?;
        Ok(Self {
            runtime,
            registered_events: Vec::new(),
            dt_accum: 0.0,
        })
    }

    /// Register a one-shot tone event (demo without bank files).
    pub fn register_demo_tone(&mut self, path: &str, freq_hz: f32) {
        let rate = self.runtime.sample_rate();
        let frames = rate as usize / 2;
        let pcm: Vec<f32> = (0..frames)
            .map(|i| {
                let t = i as f32 / rate as f32;
                (2.0 * std::f32::consts::PI * freq_hz * t).sin() * 0.25
            })
            .collect();
        if let Ok(buf) = self.runtime.load_pcm(pcm, 1, rate) {
            self.runtime.register_event(
                EventDescription::new(path, buf).with_parameter("Intensity", 1.0),
            );
            self.registered_events.push(path.to_string());
        }
    }

    /// Load `bank.json` from disk (editor Build output) and register all events.
    ///
    /// `path` points at `build/Master/bank.json`. Asset files are resolved
    /// relative to that file's directory.
    pub fn load_bank_dir(&mut self, bank_json: &Path) -> Result<usize, String> {
        let bank = RamodBankAsset::from_json_path(bank_json).map_err(|e| e.to_string())?;
        self.register_bank_asset(&bank)
    }

    /// Register events from an already-parsed [`RamodBankAsset`].
    pub fn register_bank_asset(&mut self, bank: &RamodBankAsset) -> Result<usize, String> {
        let base = bank
            .base_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));
        let mut count = 0;
        for ev in &bank.events {
            match self.register_one_event(&base, ev) {
                Ok(()) => {
                    count += 1;
                    self.registered_events.push(ev.path.clone());
                }
                Err(e) => {
                    bevy::log::warn!("bank event {}: {e}", ev.path);
                }
            }
        }
        bevy::log::info!(
            "RamodAudio: registered {count}/{} events from bank '{}'",
            bank.events.len(),
            bank.name
        );
        Ok(count)
    }

    fn register_one_event(&mut self, base: &Path, ev: &BankEventAsset) -> Result<(), String> {
        let (samples, channels, sample_rate) = resolve_pcm(base, ev)?;
        let buf = self
            .runtime
            .load_pcm(samples, channels, sample_rate)
            .map_err(|e| e.to_string())?;

        let mut desc = EventDescription::new(&ev.path, buf)
            .with_loop(ev.looping)
            .with_priority(ev.priority);

        if ev.is_3d {
            desc = desc.with_3d(Attenuation {
                min_distance: ev.min_distance,
                max_distance: ev.max_distance,
            });
        }
        for (name, default) in &ev.parameters {
            desc = desc.with_parameter(name.clone(), *default);
        }
        // Ensure Intensity exists for transport compatibility.
        if !ev.parameters.iter().any(|(n, _)| n == "Intensity") {
            desc = desc.with_parameter("Intensity", 1.0);
        }

        self.runtime.register_event(desc);
        Ok(())
    }

    pub fn play_event(&mut self, path: &str) -> Option<u64> {
        self.runtime.play_event(path).ok()
    }

    pub fn stop_instance(&mut self, id: u64) {
        self.runtime.release_instance(id);
    }
}

fn resolve_pcm(base: &Path, ev: &BankEventAsset) -> Result<(Vec<f32>, u16, u32), String> {
    if let Some(ref rel) = ev.asset_file {
        let full = base.join(rel);
        if full.exists() {
            let w = WaveBuffer::from_file(&full).map_err(|e| format!("{}: {e}", full.display()))?;
            return Ok((w.samples, w.channels, w.sample_rate));
        }
        return Err(format!("asset file not found: {}", full.display()));
    }

    // Fallback: short demo tone so events without files still register.
    let rate = 48_000u32;
    let frames = rate as usize / 4;
    let freq = 440.0;
    let samples: Vec<f32> = (0..frames)
        .map(|i| {
            let t = i as f32 / rate as f32;
            (2.0 * std::f32::consts::PI * freq * t).sin() * 0.2
        })
        .collect();
    Ok((samples, 1, rate))
}

pub struct RamodPlugin {
    pub open_device: bool,
}

impl Default for RamodPlugin {
    fn default() -> Self {
        Self { open_device: true }
    }
}

impl Plugin for RamodPlugin {
    fn build(&self, app: &mut App) {
        let audio = RamodAudio::new(self.open_device).unwrap_or_else(|e| {
            bevy::log::error!("RamodAudio init failed: {e}; falling back to headless");
            RamodAudio::new(false).expect("headless runtime")
        });

        app.insert_resource(audio)
            .init_asset::<RamodBankAsset>()
            .register_asset_loader(RamodBankLoader)
            .add_systems(
                PostUpdate,
                (
                    sync_listener,
                    start_emitters,
                    sync_emitters_3d,
                    audio_tick_render,
                )
                    .chain(),
            );
    }
}

fn sync_listener(
    mut audio: ResMut<RamodAudio>,
    q: Query<(&GlobalTransform, Option<&Velocity>), With<AudioListener>>,
) {
    let Ok((gt, vel)) = q.get_single() else {
        return;
    };
    let t = gt.translation();
    let velocity = vel.map(|v| v.0).unwrap_or(Vec3::ZERO);
    audio.runtime.set_listener(Attributes3d {
        position: RVec3 {
            x: t.x,
            y: t.y,
            z: t.z,
        },
        velocity: RVec3 {
            x: velocity.x,
            y: velocity.y,
            z: velocity.z,
        },
        ..Default::default()
    });
}

fn start_emitters(
    mut commands: Commands,
    mut audio: ResMut<RamodAudio>,
    mut q: Query<(Entity, &mut AudioEmitter, &GlobalTransform), Added<PlayOnSpawn>>,
) {
    for (entity, mut emitter, gt) in &mut q {
        if emitter.instance_id.is_some() {
            commands.entity(entity).remove::<PlayOnSpawn>();
            continue;
        }
        match audio.runtime.play_event(&emitter.event) {
            Ok(id) => {
                if emitter.is_3d {
                    let t = gt.translation();
                    audio.runtime.set_instance_3d(
                        id,
                        Attributes3d {
                            position: RVec3 {
                                x: t.x,
                                y: t.y,
                                z: t.z,
                            },
                            ..Default::default()
                        },
                    );
                }
                emitter.instance_id = Some(id);
            }
            Err(e) => {
                bevy::log::warn!("play_event {}: {e}", emitter.event);
            }
        }
        commands.entity(entity).remove::<PlayOnSpawn>();
    }
}

fn sync_emitters_3d(
    mut audio: ResMut<RamodAudio>,
    q: Query<(&AudioEmitter, &GlobalTransform, Option<&Velocity>)>,
) {
    for (emitter, gt, vel) in &q {
        let Some(id) = emitter.instance_id else {
            continue;
        };
        if !emitter.is_3d {
            continue;
        }
        let t = gt.translation();
        let velocity = vel.map(|v| v.0).unwrap_or(Vec3::ZERO);
        audio.runtime.set_instance_3d(
            id,
            Attributes3d {
                position: RVec3 {
                    x: t.x,
                    y: t.y,
                    z: t.z,
                },
                velocity: RVec3 {
                    x: velocity.x,
                    y: velocity.y,
                    z: velocity.z,
                },
                ..Default::default()
            },
        );
    }
}

fn audio_tick_render(time: Res<Time>, mut audio: ResMut<RamodAudio>) {
    let dt = time.delta_secs();
    audio.runtime.tick(dt);
    let frames = ((dt * audio.runtime.sample_rate() as f32) as usize).clamp(64, 4096);
    let ch = audio.runtime.channels() as usize;
    let mut out = vec![0.0f32; frames * ch.max(1)];
    audio.runtime.render(&mut out);
}
