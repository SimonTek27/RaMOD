//! Minimal bevy_ramod example (S8.4).
//!
//! ```bash
//! cargo run -p bevy_ramod --example basic
//! ```

use bevy::prelude::*;
use bevy_ramod::prelude::*;
use bevy::prelude::Vec3;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_ramod basic".into(),
                resolution: bevy::window::WindowResolution::new(800, 600),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RamodPlugin { open_device: true, ..Default::default() })
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut audio: ResMut<RamodAudio>) {
    commands.spawn(Camera3d::default());

    // Listener on camera root
    commands.spawn((Camera3d::default(), AudioListener, Velocity::default()));

    // Demo event (no bank file needed)
    audio.register_demo_tone("event:/sfx/beep", 440.0);

    // 3D emitter in front of origin
    commands.spawn((
        SpatialAudioBundle::new("event:/sfx/beep", Vec3::new(2.0, 0.0, -5.0)),
        PlayOnSpawn,
    ));

    log::info!("bevy_ramod: playing event:/sfx/beep");
}
