# RaMOD Architecture Blueprint

**Open-source FMOD-class audio middleware and audio editor for Bevy and Renzora**  
**Version:** 0.1 · 2026-10-18  
**Status:** Canonical technical blueprint  
**Companion docs:** [DESIGN.md](DESIGN.md) · [ROADMAP.md](ROADMAP.md)

---

## 0. Mandate

RaMOD **must** become a complete, open-source alternative to FMOD and WWISE.

- No proprietary runtime  
- No mandatory closed binaries  
- Pure Rust core (optional backends only)  
- First-class Bevy and Renzora integration  
- Authoring tools (GoldWave-style editor + light Studio) in the same ecosystem  

This document defines **how** the pieces fit together.

---

## 1. System Overview

```
┌────────────────────────────────────────────────────────────────────────┐
│                         GAME / BEVY / RAMOD                            |
|                             APPLICATION                                │
│                                                                        │
│   Components: AudioEmitter, AudioListener, Velocity, BusVolume…        │
│   Systems: spatial_sync, runtime_tick, event_lifecycle                 │
│   Assets: Bank, EventDescription, Snapshot                             │
└────────────────────────────────┬───────────────────────────────────────┘
                                 │  bevy_ramod (plugin layer)
                                 │  renzora_ramod (plugin layer)
┌────────────────────────────────▼───────────────────────────────────────┐
│                         RaMOD RUNTIME (Core)                           │
│                                                                        │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐  ┌──────────────┐  │
│  │   Mixer     │  │   Voices     │  │   Events    │  │  Parameters  │  │
│  │ hierarchical│  │ + virtual    │  │  system     │  │  + automation│  │
│  └─────────────┘  └──────────────┘  └─────────────┘  └──────────────┘  │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐  ┌──────────────┐  │
│  │  3D Spatial │  │  DSP Graph   │  │  Streaming  │  │  Snapshots   │  │
│  └─────────────┘  └──────────────┘  └─────────────┘  └──────────────┘  │
└────────────────────────────────┬───────────────────────────────────────┘
                                 │
┌────────────────────────────────▼───────────────────────────────────────┐
│                    FOUNDATION + CODECS + BANKS                         │
│                                                                        │
│  FSB5 R/W/Edit  ·  PCM / Vorbis  ·  Bank format  ·  Sample store       │
│  (current RaMOD crate – pure std)                                      │
└────────────────────────────────┬───────────────────────────────────────┘
                                 │
┌────────────────────────────────▼───────────────────────────────────────┐
│                         AUTHORING TOOLS                                │
│                                                                        │
│  Waveform / Spectral Editor (GoldWave-style)                           │
│  Event Designer (timeline, instruments, automation)                    │
│  Bank Builder · Batch processor · Live Update client                   │
└────────────────────────────────────────────────────────────────────────┘
```

Three crates (recommended layout):

| Crate | Responsibility |
|-------|----------------|
| `ramod` | Foundation: FSB5, codecs, bank I/O, sample data, pure-Rust encode/decode |
| `ramod_runtime` | Real-time engine: mixer, voices, events, 3D, DSP, parameters |
| `engine` | Bevy and Renzora plugin: components, systems, AssetLoaders, Live Update |

Optional later: `ramod_editor` (standalone or embedded GUI).

---

## 2. Lessons from bevy_fmod (what we keep, what we replace)

### Keep (ECS integration pattern)

```text
FmodPlugin                    →  RamodPlugin
FmodStudio (Resource)         →  RamodRuntime (Resource)
AudioSource + EventInstance   →  AudioEmitter + EventHandle
AudioListener                 →  AudioListener
Velocity (auto Doppler)       →  Velocity (same idea)
SpatialAudioBundle            →  SpatialEmitterBundle
studio.update() every frame   →  runtime.tick() every frame
Observer on Remove            →  same: stop + release on despawn
```

### Replace (proprietary parts)

| bevy_fmod | RaMOD open-source equivalent |
|-----------|------------------------------|
| `libfmod` + FMOD DLLs | `ramod_runtime` (pure Rust) |
| `.bank` (FMOD Studio) | FSB5 + future native bank / event format |
| FMOD Studio desktop | RaMOD Event Designer + waveform editor |
| FMOD Live Update protocol | Own lightweight Live Update protocol |
| Commercial license | GPL-3.0-or-later (or dual later) |

---

## 3. Runtime Core – FMOD Core equivalent

### 3.1 Hierarchical Mixer

```
Master
 ├── Music (bus)
 │    ├── Stems
 │    └── Adaptive layers
 ├── SFX (bus)
 │    ├── Weapons
 │    ├── Footsteps
 │    └── UI
 ├── Voice
 └── Ambience
```

- Each bus has volume, mute, solo, sends, effect chain.
- Unlimited depth in principle; practical limit for performance.
- Side-chain and ducking supported later.

### 3.2 Voices & Virtual Voices

- Real voices = currently audible / highest priority.
- Virtual voices = tracked but not mixed (steal by audibility / priority).
- Matches FMOD’s virtual voice system – critical for games with hundreds of emitters.

### 3.3 Event System (Studio-like)

An **Event** is the unit designers and programmers work with:

- One or more instruments (samples, multi-sounds, scatterers…)
- Parameters (float, labeled, timeline)
- Automation curves
- Logic (random, sequence, conditional)
- 3D settings (min/max distance, attenuation, Doppler scale…)

At runtime:

```rust
let handle = runtime.create_instance("event:/weapons/rifle_fire")?;
handle.set_parameter("Distance", 12.0);
handle.set_3d_attributes(pos, vel, forward, up);
handle.start();
```

### 3.4 DSP Graph

Modular nodes:

- Gain, Pan, Stereo balance
- EQ (parametric / graphic)
- Filters (LP/HP/BP)
- Dynamics (compressor, limiter, gate)
- Time-based (delay, reverb, chorus, flanger)
- Custom / user nodes (plugin interface later)

Nodes are connected into a graph per bus or per event instance.

### 3.5 3D Spatial

- Listener attributes (position, velocity, forward, up) – right-handed to match Bevy.
- Emitter attributes.
- Distance attenuation curves (linear, inverse, custom).
- Doppler from relative velocity.
- Basic occlusion / obstruction (phase 2+).

### 3.6 Parameters & Snapshots

- Named parameters (global or per-event).
- Snapshots = saved mixer state that can be blended in/out (game state changes).

---

## 4. Bevy Integration (`bevy_ramod`) – concrete design

Inspired directly by bevy_fmod’s structure, but pointing at our runtime.

### 4.1 Plugin

```rust
pub struct RamodPlugin {
    pub bank_paths: &'static [&'static str],
    // future: live_update: bool, max_channels: u32, …
}

impl Plugin for RamodPlugin {
    fn build(&self, app: &mut App) {
        let runtime = RamodRuntime::new(self.bank_paths)?;
        app.insert_resource(runtime)
           .add_plugins(VelocityPlugin)
           .add_systems(Update, (
               AudioEmitter::sync_3d,
               AudioListener::sync_3d,
           ))
           .add_systems(PostUpdate, runtime_tick)
           .add_observer(on_remove_emitter);
    }
}
```

### 4.2 Core components

```rust
#[derive(Component)]
pub struct AudioEmitter {
    pub instance: EventHandle,      // handle into ramod_runtime
    pub stop_mode: StopMode,
}

#[derive(Component, Default)]
pub struct AudioListener;

#[derive(Component, Default)]
pub struct Velocity {
    last_position: Vec3,
    pub(crate) current_velocity: Vec3,
}
```

### 4.3 Systems (same responsibilities as bevy_fmod)

1. **VelocityPlugin** – compute velocity from `GlobalTransform` delta.  
2. **sync_3d** – push position/velocity/orientation to runtime.  
3. **runtime_tick** – `runtime.tick(dt)` every frame.  
4. **on_remove_emitter** – stop + release instance on despawn.

### 4.4 Bundles

```rust
SpatialEmitterBundle  { AudioEmitter, Velocity, Transform }
SpatialListenerBundle { AudioListener, Velocity, Transform }
```

### 4.5 Asset pipeline

- `Bank` asset + `AssetLoader` for `.fsb` / native banks.
- `EventDescription` as asset or lookup by path/name.
- Hot-reload where possible.

### 4.6 Live Update (future)

- Lightweight TCP/WebSocket protocol between editor and running game.
- Push parameter changes, volume, mute, even hot-swap event definitions.
- Feature-gated so release builds stay clean.

---

## 5. Foundation (current `ramod` crate) – role in the stack

The existing pure-Rust FSB5 work becomes the **content layer**:

- Parse / write / edit FSB5 banks (already done).
- Extract samples to WAV / Ogg.
- Build banks from WAV / Ogg.
- Pure-Rust Vorbis encode/decode (roadmap items 1–2).
- Sample metadata (loop points, frequency, channels…).

Later the foundation also owns:

- Native bank format (if we outgrow FSB5 for events + parameters).
- Streaming file readers.
- Sample decoding to the runtime’s internal PCM buffers.

**Rule:** Foundation stays dependency-light (`std` + carefully chosen optional crates). Runtime may pull in an audio backend (cpal / Firewheel / etc.).

---

## 6. Authoring Layer (GoldWave + light Studio)

### 6.1 Waveform Editor (GoldWave-style)

- Interactive waveform + spectral view
- Non-destructive cut / copy / paste / mix / crossfade
- Effects chain with preview
- Batch processing
- Cue points / regions
- Export back into banks or standalone files

### 6.2 Event Designer

- Multi-track timeline
- Instruments (single, multi, scatter, program…)
- Parameter tracks and automation
- Logic boxes (random, sequence, probability)
- 3D settings per event
- Preview against the same runtime used in-game

### 6.3 Bank Builder

- Collect events + samples
- Compression options
- Platform variants (optional)
- Validation + size report

---

## 7. Data Flow (end-to-end)

```
[Designer]
   │  edits samples in Waveform Editor
   │  builds events in Event Designer
   ▼
[Bank file]  ──────────────────────────────►  [Bevy AssetServer]
   │                                              │
   │                                              ▼
   │                                         RamodRuntime
   │                                         (loads bank)
   │                                              │
   │                                              ▼
   │                                         create_instance("event:/…")
   │                                              │
   │                                              ▼
[Game code / ECS]  ◄──── AudioEmitter component ──┘
   │
   │  Transform + Velocity updated every frame
   ▼
spatial_sync systems → runtime.set_3d_attributes(...)
   │
   ▼
runtime.tick() → mixer → audio backend → speakers
```

Live Update path (later):

```
[Event Designer]  ←→  Live Update protocol  ←→  RamodRuntime (in game)
```

---

## 8. Module Map (target crate layout)

```
ramod/                          # Foundation (existing + growth)
├── fsb / fsb_write
├── vorbis / vorbis_encode
├── bank (higher-level bank API)
└── sample

ramod_runtime/                  # Real-time engine
├── mixer/
├── voice/
├── event/
├── parameter/
├── spatial/
├── dsp/
├── stream/
└── backend/   (cpal, firewheel, …)

bevy_ramod/                     # Bevy glue
├── plugin.rs
├── runtime_resource.rs
├── components/
├── systems/
├── assets/
└── bundles.rs

ramod_editor/                   # Optional GUI / CLI tools
├── waveform/
├── event_designer/
└── live_update/
```

---

## 9. Implementation Order (aligned with ROADMAP)

1. **Phase 0** – Finish foundation (pure-Rust Vorbis, CLI, tests, crates.io).  
2. **Phase 1** – Minimal runtime: mixer + voices + play sample + 3D position.  
3. **bevy_ramod skeleton** – Plugin + AudioEmitter/Listener + Velocity + tick (mirror bevy_fmod).  
4. **Phase 1 continued** – Events + parameters + basic DSP.  
5. **Phase 2** – Waveform editor MVP.  
6. **Phase 3** – Event Designer + Live Update.  
7. **Polish** – Snapshots, virtual voices, streaming, occlusion, docs, examples.

At every step the **open-source mandate** is non-negotiable: no proprietary FMOD libraries in the dependency tree of the core path.

---

## 10. Compatibility & Interop

- **FSB5**: first-class (modding + existing content).  
- **Future native bank**: designed for events, parameters, snapshots.  
- **Bevy audio / Kira / Firewheel / Renzora**: can coexist; `bevy_ramod` and `renzora_ramod` are an alternative, not a forced replacement.  
- **WASM**: target after desktop is solid (backend choice matters).  

---

## 11. Success Definition (open-source FMOD / WWISE)

RaMOD is successful when a Bevy team and/or Renzora team can:

1. Design adaptive audio without FMOD Studio.  
2. Ship a game using only open-source audio middleware.  
3. Edit samples with a GoldWave-class tool in the same ecosystem.  
4. Use idiomatic ECS components (`AudioEmitter`, `AudioListener`, …).  
5. Hot-update parameters and content while the game is running.  
6. Keep full ownership of their audio pipeline.

---

*This blueprint is the technical North Star.  
DESIGN.md = why · ROADMAP.md = what & when · ARCHITECTURE.md = how.*
