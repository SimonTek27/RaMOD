# RaMOD × Renzora Engine

**Status:** complete for Bevy **0.19** — Option A/B + native plugin `engine/renzora_ramod/` (marketplace id `ramod`) — 2026-10-22

[Renzora Engine](https://github.com/renzora/engine) is a modular 2D/3D game engine on **Bevy 0.19**. Default audio is Kira (`renzora_audio` + `renzora::audio_backend::Backend`).

RaMOD is an FMOD-class open-source stack. Integration is **optional** and **one-way**:

```
Renzora game / plugin  ──depends on──▶  bevy_ramod (feature = "renzora")
                                              │
                                              ▼
                                       ramod_runtime (cpal)
```

RaMOD does **not** depend on the Renzora monorepo (the engine is too large and workspace-bound for a hard git dep). Zero-glue `impl Backend` lives in the **consumer** crate; the reference is [`docs/renzora_backend_glue.rs`](renzora_backend_glue.rs).

---

## Enable

```toml
[dependencies]
bevy_ramod = { git = "https://github.com/SimonTek27/RaMOD", features = ["renzora"] }
```

---

## Option A — RaMOD-only (recommended)

Skip Kira. Full events, banks, Live Update:

```rust
use bevy::prelude::*;
use bevy_ramod::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .replace_renzora_audio_with_ramod()
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

`RenzoraHost.replace_kira == true` → do not open a Kira device.

---

## Option B — Renzora `Backend` (zero-glue)

1. Copy [`docs/renzora_backend_glue.rs`](renzora_backend_glue.rs) into your Renzora game/plugin crate (or depend on it as a module).
2. Register **once**:

```rust
use renzora::audio_backend::AppAudioBackendExt;
// after copying glue:
app.add_ramod_audio_backend(true)?;
// or: app.add_audio_backend(RamodBackend::new(true)?);
```

3. Do **not** also register the default Kira backend.

`RamodRenzoraBackend` already implements the Renzora method surface (load_clip, play, stop, update, spatial, buses). The glue only maps mirror types ↔ `renzora::audio_backend::*`.

### Shared bus names

| Key    | Constant |
|--------|----------|
| Master | `bevy_ramod::renzora::buses::MASTER` |
| SFX    | `bevy_ramod::renzora::buses::SFX` |
| Music  | `bevy_ramod::renzora::buses::MUSIC` |

---

## Live Update

`RamodPlugin { enable_live_update: true }` binds TCP **:17420**. The RaMOD editor can push parameters into the game.

---

## API summary (`feature = "renzora"`)

| API | Purpose |
|-----|---------|
| `replace_renzora_audio_with_ramod()` | Option A |
| `add_ramod_for_renzora()` | RamodPlugin + markers, Kira still allowed |
| `insert_ramod_renzora_backend(open)` | NonSend `RamodBackendCell` |
| `RamodRenzoraBackend` | Mixer implementation |
| `docs/renzora_backend_glue.rs` | Zero-glue `impl Backend` |

---

## Version matrix

| Host / crate | Bevy |
|--------------|------|
| Renzora Engine | 0.19 |
| bevy_ramod | 0.19 |

---

## Native plugin / marketplace (`engine/renzora_ramod/`)

Ship RaMOD as a Renzora **native plugin** (dylib, compiled by the SDK):

| Field | Value |
|-------|--------|
| Crate folder | [`engine/renzora_ramod/`](../engine/renzora_ramod/) |
| Package name | `renzora_ramod` |
| Marketplace id | `ramod` |
| Scope | **Runtime** (editor + game exports) |
| Entry | `RamodRenzoraPlugin` → `replace_renzora_audio_with_ramod()` |

```sh
# local: symlink into engine plugins dir, or install from marketplace when published
# renzora publish   # maintainers
```

See [`engine/renzora_ramod/README.md`](../engine/renzora_ramod/README.md). Do **not** `cargo build` that folder against crates.io Bevy.

## Follow-ups

- [x] Bevy 0.19 alignment
- [x] Feature `renzora` + host markers
- [x] `RamodRenzoraBackend` + bus constants
- [x] `replace_renzora_audio_with_ramod`
- [x] Zero-glue reference (`renzora_backend_glue.rs`)
- [x] Native plugin crate `engine/renzora_ramod/` (marketplace id `ramod`, Runtime)
- [ ] Publish to Renzora marketplace (`renzora publish` with credentials)
