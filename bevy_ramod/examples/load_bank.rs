//! Load editor-built bank.json and play an event.
//!
//! ```bash
//! # 1) Build a bank from the editor (File → Build bank) → build/Master/bank.json
//! # 2) Or use the sample under examples/sample_bank/
//! cargo run -p bevy_ramod --example load_bank
//! ```

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_ramod::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_ramod load_bank".into(),
                resolution: (800., 600.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RamodPlugin { open_device: true })
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut audio: ResMut<RamodAudio>) {
    commands.spawn((
        Camera3d::default(),
        AudioListener,
        Velocity::default(),
    ));

    // Prefer editor Build output; fall back to sample_bank shipped with the example.
    let candidates = [
        PathBuf::from("build/Master/bank.json"),
        PathBuf::from("bevy_ramod/examples/sample_bank/bank.json"),
        PathBuf::from("examples/sample_bank/bank.json"),
    ];

    let mut loaded = false;
    for path in &candidates {
        if path.exists() {
            match audio.load_bank_dir(path) {
                Ok(n) => {
                    bevy::log::info!("Loaded {n} events from {}", path.display());
                    loaded = true;
                    break;
                }
                Err(e) => bevy::log::warn!("load {}: {e}", path.display()),
            }
        }
    }

    if !loaded {
        bevy::log::warn!("No bank.json found — registering demo tone");
        audio.register_demo_tone("event:/sfx/beep", 523.25);
    }

    let event = audio
        .registered_events
        .first()
        .cloned()
        .unwrap_or_else(|| "event:/sfx/beep".into());

    commands.spawn((
        SpatialAudioBundle::new(event.clone(), Vec3::new(0.0, 0.0, -3.0)),
        PlayOnSpawn,
    ));

    bevy::log::info!("Playing {event}");
}
