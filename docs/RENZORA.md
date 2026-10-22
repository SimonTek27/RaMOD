# RaMOD × Renzora Engine

**Status:** Bevy **0.19** alignment + feature `renzora` (2026-10-22)

[Renzora Engine](https://github.com/renzora/engine) is a modular 2D/3D game engine and scene editor built on **Bevy 0.19**. Its default audio path is Kira (`renzora_audio` + `renzora::audio_backend::Backend`).

RaMOD is an **FMOD-class** open-source middleware (events, hierarchical mixer, FSB5, Live Update, Studio-style editor). It must remain usable outside Renzora; integration is **optional** and one-way:

```
Renzora game / plugin  ──depends on──►  bevy_ramod (feature = "renzora")
                                              │
                                              ▼
                                       ramod_runtime (cpal)
```

RaMOD does **not** depend on the Renzora monorepo.

## Why Bevy 0.19

Renzora pins Bevy 0.19.1. `bevy_ramod` targets the same major so a Renzora project can link RaMOD without dual Bevy versions.

## Enable

```toml
[dependencies]
bevy_ramod = { git = "https://github.com/SimonTek27/RaMOD", features = ["renzora"] }
```

```rust
use bevy::prelude::*;
use bevy_ramod::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_ramod_for_renzora() // RamodPlugin + RenzoraHost markers
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut audio: ResMut<RamodAudio>) {
    commands.spawn((Camera3d::default(), AudioListener));
    audio.register_demo_tone("event:/sfx/beep", 440.0);
    commands.spawn((
        SpatialAudioBundle::new("event:/sfx/beep", Vec3::new(0.0, 0.0, -3.0)),
        PlayOnSpawn,
    ));
}
```

Without the feature, use the normal API:

```rust
.add_plugins(RamodPlugin::default())
```

## Coexistence with Kira

Renzora’s `AppAudioBackendExt::add_audio_backend` registers a single mixer. Options:

1. **RaMOD-only games** — do not register a Kira backend; only add `RamodPlugin`.
2. **Side-by-side** — keep Kira for simple clips; use RaMOD events/banks for adaptive audio (two devices is undesirable; prefer one primary path).
3. **Future** — implement `renzora::audio_backend::Backend` on top of `ramod_runtime` so Renzora’s command queue drives RaMOD (tracked as a follow-up; not required for feature `renzora` v0).

## Live Update

`RamodPlugin { enable_live_update: true }` binds TCP **:17420**. The RaMOD editor can push parameters into a Renzora (or plain Bevy) game the same way as a standalone Bevy app.

## Version matrix

| Crate / host        | Bevy  |
|---------------------|-------|
| Renzora Engine      | 0.19  |
| bevy_ramod (this)   | 0.19  |
| bevy_ramod (legacy)  | 0.15  |

## Follow-ups

- [ ] Optional `renzora::audio_backend::Backend` adapter (RaMOD as Renzora mixer)
- [ ] Renzora plugin crate / marketplace package that wraps `bevy_ramod`
- [ ] Shared bus naming conventions (Master / SFX / Music)
