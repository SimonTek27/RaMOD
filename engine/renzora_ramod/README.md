# renzora_ramod

**RaMOD** as a [Renzora Engine](https://github.com/renzora/engine) native plugin — open-source FMOD-class audio (events, hierarchical mixer, banks/FSB5, spatial, Live Update).

| | |
|---|---|
| Marketplace id | `ramod` |
| Scope | **Runtime** (editor + exported games) |
| Bevy | 0.19 (aligned with Renzora) |
| License | GPL-3.0-or-later (same as RaMOD) |

## What it does

On load, the plugin calls `replace_renzora_audio_with_ramod()` so RaMOD is the primary mixer. **Do not** also register Renzora’s default Kira backend in the same app.

Use the normal `bevy_ramod` API in your game code:

```rust
use bevy::prelude::*;
use bevy_ramod::prelude::*;

fn setup(mut commands: Commands, mut audio: ResMut<RamodAudio>) {
    commands.spawn((Camera3d::default(), AudioListener));
    audio.register_demo_tone("event:/sfx/beep", 440.0);
    commands.spawn((
        SpatialAudioBundle::new("event:/sfx/beep", Vec3::new(0.0, 0.0, -3.0)),
        PlayOnSpawn,
    ));
}
```

Live Update listens on **TCP :17420** (RaMOD Studio editor → game).

## Install

### Marketplace (when published)

Install **ramod** from the Renzora plugin store. The editor compiles the plugin against the staged SDK.

### Local / from this repo

1. Clone [RaMOD](https://github.com/SimonTek27/RaMOD).
2. Copy or symlink `engine/renzora_ramod/` into your engine’s `plugins/` directory as `ramod/` (or install via your usual native-plugin flow).
3. Ensure a local engine checkout exists so the `renzora` path in `Cargo.toml` resolves for rust-analyzer, **or** rely on the SDK build only.
4. Restart the editor — it will compile the dylib.

⚠ **Do not** run `cargo build` inside this folder against crates.io Bevy. See [Native Plugins](https://github.com/renzora/engine/blob/main/docs/r1-alpha8/extending/native-plugins.md).

## Option B (Renzora `Backend` trait)

If you need the engine command queue to drive RaMOD instead of the Bevy plugin path, use the glue in [`docs/renzora_backend_glue.rs`](../../docs/renzora_backend_glue.rs) inside a game/plugin crate that already depends on `renzora`.

## Publish (maintainers)

```sh
# from a machine with the Renzora CLI and publisher credentials
renzora publish
```

`[package.metadata.renzora]` in `Cargo.toml` holds `marketplace_id`, tags, and `min_engine_version`.

## Links

- RaMOD: https://github.com/SimonTek27/RaMOD
- Integration guide: https://github.com/SimonTek27/RaMOD/blob/main/docs/RENZORA.md
- Renzora Engine: https://github.com/renzora/engine
