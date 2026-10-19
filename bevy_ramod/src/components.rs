//! ECS components (S8.2) — patterned after bevy_fmod.

use bevy::prelude::*;
use ramod_runtime::Attenuation;

/// Marks the entity as the audio listener (typically the camera / player).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct AudioListener;

/// Linear velocity for Doppler (world units / second).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Velocity(pub Vec3);

/// Emitter attached to an entity; plays an event path when started.
#[derive(Component, Debug, Clone)]
pub struct AudioEmitter {
    /// Event path, e.g. `event:/sfx/footstep`.
    pub event: String,
    /// Runtime instance id when playing.
    pub instance_id: Option<u64>,
    pub auto_play: bool,
    pub is_3d: bool,
    pub attenuation: Attenuation,
    pub volume: f32,
}

impl AudioEmitter {
    pub fn new(event: impl Into<String>) -> Self {
        Self {
            event: event.into(),
            instance_id: None,
            auto_play: true,
            is_3d: true,
            attenuation: Attenuation::default(),
            volume: 1.0,
        }
    }

    pub fn two_d(event: impl Into<String>) -> Self {
        let mut e = Self::new(event);
        e.is_3d = false;
        e
    }
}

/// Play the emitter once on spawn (consumed after start).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct PlayOnSpawn;

/// Convenience bundle for a 3D emitter.
#[derive(Bundle, Clone)]
pub struct SpatialAudioBundle {
    pub emitter: AudioEmitter,
    pub velocity: Velocity,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl SpatialAudioBundle {
    pub fn new(event: impl Into<String>, translation: Vec3) -> Self {
        Self {
            emitter: AudioEmitter::new(event),
            velocity: Velocity::default(),
            transform: Transform::from_translation(translation),
            global_transform: GlobalTransform::default(),
        }
    }
}
