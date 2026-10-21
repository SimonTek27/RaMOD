//! Live Update end-to-end: Bevy game listens; editor connects on :17420.
//!
//! ```bash
//! cargo run -p bevy_ramod --example live_update
//! # then: cargo run -p ramod_editor  → status bar "Live Update On"
//! # drag Intensity — parameter reaches the game
//! ```

use bevy::prelude::*;
use bevy_ramod::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_ramod Live Update (listen :17420)".into(),
                resolution: (640., 360.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RamodPlugin {
            open_device: true,
            enable_live_update: true,
        })
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut audio: ResMut<RamodAudio>) {
    commands.spawn((
        Camera3d::default(),
        AudioListener,
        Velocity::default(),
    ));

    audio.register_demo_tone("event:/sfx/beep", 440.0);
    commands.spawn((
        SpatialAudioBundle::new("event:/sfx/beep", Vec3::new(0.0, 0.0, -2.0)),
        PlayOnSpawn,
        AudioParameter::new("Intensity", 1.0),
    ));

    bevy::log::info!("Open ramod_editor → enable Live Update → tweak Intensity");
}
