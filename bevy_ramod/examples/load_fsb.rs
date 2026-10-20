//! Load a binary Master.fsb bank and play the first sample.
//!
//! ```bash
//! # 1) restore parser + build editor bank
//! cargo build -p ramod
//! cargo run -p ramod_editor   # import → assign → Build bank
//!
//! # 2) play FSB
//! cargo run -p bevy_ramod --example load_fsb
//! ```

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_ramod::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_ramod load_fsb".into(),
                resolution: (800., 600.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RamodPlugin { open_device: true })
        .add_systems(Startup, setup)
        .add_systems(Update, spawn_when_ready)
        .run();
}

#[derive(Resource)]
struct Ready {
    spawned: bool,
}

fn setup(mut commands: Commands, mut audio: ResMut<RamodAudio>) {
    commands.spawn((
        Camera3d::default(),
        AudioListener,
        Velocity::default(),
    ));

    let candidates = [
        PathBuf::from("build/Master/Master.fsb"),
        PathBuf::from("build/Master.fsb"),
        PathBuf::from("assets/Master.fsb"),
    ];

    let mut found = false;
    for p in &candidates {
        if p.exists() {
            commands.spawn(LoadBank::new(p.clone()));
            found = true;
            break;
        }
    }

    if !found {
        // Directory form also works: load_bank_path picks Master.fsb inside
        let dir = PathBuf::from("build/Master");
        if dir.is_dir() {
            commands.spawn(LoadBank::new(dir));
            found = true;
        }
    }

    if !found {
        bevy::log::warn!("No Master.fsb found — demo tone fallback");
        audio.register_demo_tone("event:/fsb/demo", 440.0);
    }

    commands.insert_resource(Ready { spawned: false });
}

fn spawn_when_ready(
    mut commands: Commands,
    audio: Res<RamodAudio>,
    mut ready: ResMut<Ready>,
) {
    if ready.spawned {
        return;
    }
    let Some(event) = audio.registered_events.first().cloned() else {
        return;
    };
    commands.spawn((
        SpatialAudioBundle::new(event.clone(), Vec3::new(0.0, 0.0, -2.0)),
        PlayOnSpawn,
        AudioParameter::new("Intensity", 0.9),
    ));
    ready.spawned = true;
    bevy::log::info!("Playing {event}");
}
