//! ECS components (S8.2) — patterned after bevy_fmod.

use std::path::PathBuf;

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
    /// Linear gain 0…1+ applied as Intensity / voice gain.
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

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }
}

/// One Studio-style parameter driven from ECS.
///
/// ```ignore
/// commands.spawn((
///     SpatialAudioBundle::new("event:/sfx/engine", pos),
///     PlayOnSpawn,
///     AudioParameter::new("RPM", 0.5),
/// ));
/// ```
#[derive(Component, Debug, Clone)]
pub struct AudioParameter {
    pub name: String,
    pub value: f32,
}

impl AudioParameter {
    pub fn new(name: impl Into<String>, value: f32) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// Multiple parameters on one emitter.
#[derive(Component, Debug, Clone, Default)]
pub struct AudioParameters(pub Vec<(String, f32)>);

impl AudioParameters {
    pub fn new(pairs: impl IntoIterator<Item = (impl Into<String>, f32)>) -> Self {
        Self(
            pairs
                .into_iter()
                .map(|(n, v)| (n.into(), v))
                .collect(),
        )
    }

    pub fn set(&mut self, name: impl Into<String>, value: f32) {
        let name = name.into();
        if let Some(slot) = self.0.iter_mut().find(|(n, _)| *n == name) {
            slot.1 = value;
        } else {
            self.0.push((name, value));
        }
    }
}

/// Play the emitter once on spawn (consumed after start).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct PlayOnSpawn;

/// Load a bank.json from disk on spawn, then remove this component.
#[derive(Component, Debug, Clone)]
pub struct LoadBank {
    pub path: PathBuf,
}

impl LoadBank {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

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
