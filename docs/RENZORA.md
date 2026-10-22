# RaMOD × Renzora Engine

**Status:** Bevy **0.19** + feature `renzora` + **Backend adapter** (replace Kira) — 2026-10-22

[Renzora Engine](https://github.com/renzora/engine) is a modular 2D/3D game engine on **Bevy 0.19**. Default audio is Kira (`renzora_audio` + `renzora::audio_backend::Backend`).

RaMOD is an FMOD-class open-source stack (events, mixer, FSB5, Live Update, Studio editor). Integration is **optional** and **one-way**:

```
Renzora game / plugin  ──depends on──►  bevy_ramod (feature = "renzora")
                                              │
                                              ▼
                                       ramod_runtime (cpal)
```

RaMOD does **not** depend on the Renzora monorepo.

---

## Enable

```toml
[dependencies]
bevy_ramod = { git = "https://github.com/SimonTek27/RaMOD", features = ["renzora"] }
```

---

## Option A — RaMOD-only (recommended)

Skip Kira. Use RaMOD events, banks, and Live Update:

```rust
use bevy::prelude::*;
use bevy_ramod::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Primary audio = RaMOD; do not register a Kira backend.
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

`RenzoraHost.replace_kira == true` signals the host that Kira must not open a device.

Lighter variant (markers only, Kira still allowed):

```rust
.add_ramod_for_renzora()
```

---

## Option B — Replace Kira via Renzora `Backend`

Renzora registers a single mixer with `App::add_audio_backend`. Use `RamodRenzoraBackend` so the engine command queue drives RaMOD.

### Glue (inside a Renzora plugin / game crate)

```rust
// Compile in a crate that depends on both `renzora` and `bevy_ramod`
use bevy::prelude::*;
use bevy_ramod::renzora::backend::RamodRenzoraBackend;
use renzora::audio_backend::{Backend, AppAudioBackendExt};

struct Glue(RamodRenzoraBackend);

impl Backend for Glue {
    fn name(&self) -> &str { self.0.name() }
    fn init(&mut self) -> Result<renzora::audio_backend::BackendInfo, String> {
        let i = self.0.init()?;
        Ok(renzora::audio_backend::BackendInfo {
            sample_rate: i.sample_rate,
            caps: renzora::audio_backend::Caps(i.caps.0),
            device: i.device,
        })
    }
    fn shutdown(&mut self) { self.0.shutdown(); }
    fn set_buses(&mut self, buses: &[renzora::audio_backend::BusState]) {
        let _ = buses; // map → bevy_ramod::renzora::BusState then self.0.set_buses
    }
    fn load_clip(&mut self, clip: u64, extension: &str, bytes: &[u8])
        -> Result<renzora::audio_backend::ClipInfo, String>
    {
        let c = self.0.load_clip(clip, extension, bytes)?;
        Ok(renzora::audio_backend::ClipInfo {
            duration: c.duration,
            sample_rate: c.sample_rate,
        })
    }
    fn play(&mut self, request: &renzora::audio_backend::PlayRequest) -> Result<(), String> {
        let _ = request; // map + self.0.play
        Ok(())
    }
    fn stop(&mut self, request: &renzora::audio_backend::StopRequest) {
        let _ = request;
    }
    fn update(&mut self, request: &renzora::audio_backend::UpdateRequest)
        -> renzora::audio_backend::UpdateReply
    {
        let _ = request;
        renzora::audio_backend::UpdateReply::default()
    }
}

fn build_app(app: &mut App) {
    let backend = RamodRenzoraBackend::new(true).expect("ramod backend");
    app.add_audio_backend(Glue(backend));
    // Do NOT also add the default Kira backend.
}
```

Wire-compatible mirror types live in `bevy_ramod::renzora::backend` (`Caps`, `PlayRequest`, …).

### Shared bus names

| Key    | Constant |
|--------|----------|
| Master | `bevy_ramod::renzora::buses::MASTER` |
| SFX    | `bevy_ramod::renzora::buses::SFX` |
| Music  | `bevy_ramod::renzora::buses::MUSIC` |

---

## Live Update

`RamodPlugin { enable_live_update: true }` binds TCP **:17420**.

---

## Version matrix

| Crate / host   | Bevy |
|----------------|------|
| Renzora Engine | 0.19 |
| bevy_ramod     | 0.19 |

---

## Follow-ups

- [x] `RamodRenzoraBackend` (contract mirror + Runtime wiring)
- [x] `replace_renzora_audio_with_ramod` / bus name constants
- [ ] Optional Cargo feature `renzora-backend` with git dep on `renzora` for zero-glue `impl Backend`
- [ ] Marketplace plugin package wrapping `bevy_ramod`
