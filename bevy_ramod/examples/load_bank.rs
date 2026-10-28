//! Load editor-built bank.json via [`LoadBank`] and play an event.
//!
//! ```bash
//! cargo run -p bevy_ramod --example load_bank
//! ```

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_ramod::prelude::*;
use bevy::prelude::Vec3;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_ramod load_bank".into(),
                resolution: bevy::window::WindowResolution::new(800, 600),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RamodPlugin { open_device: true, ..Default::default() })
        .add_systems(Startup, setup)
        .add_systems(Update, spawn_emitter_when_ready)
        .run();
}

#[derive(Resource)]
struct BankReady {
    event: String,
    spawned: bool,
}

fn setup(mut commands: Commands, mut audio: ResMut<RamodAudio>) {
    commands.spawn((
        Camera3d::default(),
        AudioListener,
        Velocity::default(),
    ));

    let candidates = [
        PathBuf::from("build/Master/bank.json"),
        PathBuf::from("bevy_ramod/examples/sample_bank/bank.json"),
        PathBuf::from("examples/sample_bank/bank.json"),
    ];

    let mut path = None;
    for p in &candidates {
        if p.exists() {
            path = Some(p.clone());
            break;
        }
    }

    if let Some(p) = path {
        commands.spawn(LoadBank::new(p));
        commands.insert_resource(BankReady {
            event: String::new(), // filled next frame from registered_events
            spawned: false,
        });
    } else {
        audio.register_demo_tone("event:/sfx/beep", 523.25);
        commands.insert_resource(BankReady {
            event: "event:/sfx/beep".into(),
            spawned: false,
        });
    }
}

fn spawn_emitter_when_ready(
    mut commands: Commands,
    audio: Res<RamodAudio>,
    mut ready: Option<ResMut<BankReady>>,
) {
    let Some(mut ready) = ready else {
        return;
    };
    if ready.spawned {
        return;
    }

    if ready.event.is_empty() {
        if let Some(first) = audio.registered_events.first() {
            ready.event = first.clone();
        } else {
            return; // wait for LoadBank system
        }
    }

    commands.spawn((
        SpatialAudioBundle::new(ready.event.clone(), Vec3::new(0.0, 0.0, -3.0)),
        PlayOnSpawn,
    ));
    ready.spawned = true;
    log::info!("Playing {}", ready.event);
}
