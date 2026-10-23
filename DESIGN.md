# RaMOD — Official Design Document

**Version:** 0.2  
**Date:** 2026-10-23  
**Status:** Living document  
**Authors:** Simone Rizzo (SimonTek27) + design collaboration  
**Repository:** https://github.com/SimonTek27/RaMOD  
**License:** GPL-3.0-or-later

---

## 1. Vision

**RaMOD** aims to become a complete, open-source audio middleware for games, with a primary focus on the **Bevy** engine and related Bevy-based hosts (notably **[Renzora Engine](https://github.com/renzora/engine)**).

It combines three complementary capabilities:

1. **FMOD / Wwise–class runtime engine**  
   Professional adaptive audio: hierarchical mixer, events, parameters (RTPC-style), 3D spatialization, virtual voices, DSP graph, snapshots, states/switches-inspired routing, and streaming — informed by both commercial middleware families, without proprietary runtimes.

2. **GoldWave-style sound editor**  
   Offline and real-time audio editing: non-destructive waveform/spectral editing, rich effects suite, batch processing, restoration tools, cue points, and analysis.

3. **Native Bevy integration**  
   Idiomatic ECS components, systems, AssetLoaders, and live-update support that feel native to Bevy developers, including optional first-class hooks for Renzora (Bevy 0.19).

The project starts from a solid foundation: a pure-Rust, zero-dependency FSB5 reader/writer/editor. It will evolve into a full middleware while remaining:

- Written primarily in pure Rust
- Free of proprietary runtimes (no FMOD or Wwise SDK linkage on the core path)
- Cross-platform (desktop + WASM targets)
- Suitable both as a standalone library and as a Bevy / Renzora plugin

---

## 2. Goals & Non-Goals

### 2.1 Primary Goals

- Provide a high-quality, open-source alternative to commercial audio middleware for indie and mid-size teams using Bevy.
- Offer both a powerful **runtime** (playback, mixing, spatial, adaptive) and a practical **authoring/editing** toolset in the same ecosystem.
- Keep the core library dependency-light (std + carefully chosen optional crates).
- Deliver excellent developer experience: clear API, good documentation, examples, and Bevy-first ergonomics.
- Support FSB5 natively (for compatibility with existing content and modding workflows) while adding modern formats and a custom bank format if needed.

### 2.2 Non-Goals (at least for v1.0)

- Full feature parity with commercial **FMOD Studio** or **Wwise Authoring Tool** UIs (we aim for the *spirit* and core capabilities of both, not pixel-perfect cloning).
- Real-time collaborative multi-user editing.
- High-end cinema/film post-production features.
- Supporting every obscure hardware codec from 20 years ago.
- Replacing Bevy’s eventual upstream audio engine if/when Firewheel or similar is integrated; instead we will interoperate or complement it.

---

## 3. High-Level Architecture

RaMOD is organised in three clear layers:

```
┌─────────────────────────────────────────────────────────────┐
│                    bevy_ramod (Plugin)                      │
│  ECS components • AssetLoaders • Systems • Live Update      │
└────────────────────────────┬────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────┐
│                 RaMOD Runtime (Core)                        │
│  Mixer • Voices • Events • Parameters • 3D • DSP Graph      │
└────────────────────────────┬────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────┐
│              RaMOD Foundation + Editor                      │
│  FSB5 R/W • Codecs • Waveform Editor • Effects • Bank Build │
└─────────────────────────────────────────────────────────────┘
```

### 3.1 Foundation Layer (already largely present)

- FSB5 container parse / write / in-place edit
- PCM and Vorbis codecs
- Setup-header table for FSB5 Vorbis
- WAV import/export
- Self-verifying writers
- Pure-Rust implementation (std only in current 0.1.0)

### 3.2 Runtime Layer (FMOD Core equivalent)

- Hierarchical mixer (Master → Groups → Buses → Channels)
- Voice management and virtual voice system
- Sample playback (memory + streaming)
- DSP graph (modular effects)
- 3D spatial audio (position, velocity, attenuation, Doppler, basic occlusion)
- Parameter system and automation
- Event system (one-shots, loops, multi-instrument, randomization, layers)
- Snapshots (mixer state presets)
- Profiling and diagnostics

### 3.3 Editor / Authoring Layer (GoldWave + light Studio)

- Waveform and spectral display with interactive selection
- Non-destructive editing (cut, copy, paste, mix, crossfade, silence, reverse…)
- Effects suite (EQ, dynamics, reverb, delay, pitch, time-stretch, noise reduction, de-click, etc.)
- Batch processing
- Cue points / regions / markers
- Event Designer (timeline, instruments, automation curves)
- Bank builder and optimiser
- Real-time preview that can connect to a running game (Live Update)

### 3.4 Bevy Integration Layer (`bevy_ramod`)

- Bevy Plugin
- AssetLoaders for banks and events
- ECS components (`AudioEmitter`, `AudioListener`, parameters, …)
- Systems that tick the runtime and sync transforms
- Live Update client/server for editor ↔ game communication
- Optional feature `renzora` for Renzora Engine hosts (see §7)

---

## 4. Development Phases (summary)

Detailed operational checklist lives in `ROADMAP.md`. High-level phases:

| Phase | Focus |
|-------|--------|
| **0** | Foundation solidification (FSB5, codecs, tests, CI) |
| **1** | Runtime core (mixer, voices, 3D, events, DSP) |
| **2** | Bevy plugin (`bevy_ramod`) + Live Update |
| **3** | Authoring tools (waveform + Event Designer) |
| **4** | Polish, docs, packaging, host integrations (Renzora) |

---

## 5. Runtime Design Notes

- **Mixer hierarchy** is the spine of the system; mute/solo/volume/sends and inserts live here.
- **Virtual voices** allow many logical emitters with a capped real voice pool.
- **Events** are the game-facing API: paths like `event:/sfx/footstep`, parameters, multi-instrument timelines.
- **Parameters** are continuous (RTPC-style) and drive automation, intensity, and adaptive layers.
- **3D** uses listener/emitter attributes, distance attenuation, Doppler; occlusion is progressive.
- **DSP** is a modular insert/send graph per bus and per voice where needed.
- **Banks** load descriptions + sample data (FSB5 and hybrid `bank.json` paths).

---

## 6. Bevy Integration Design Notes

```rust
// Illustrative API shape
app.add_plugins(RamodPlugin::default());

commands.spawn((
    SpatialAudioBundle::new("event:/sfx/engine", transform.translation),
    PlayOnSpawn,
    AudioListener,
));

// Parameters
event_instance.set_parameter("Intensity", 0.8);
```

Key design choices:

- Spatial data is driven by Bevy’s `Transform` and (optionally) velocity components.
- Banks and events are Bevy Assets.
- Live Update allows an external editor to connect to a running game and push changes.
- The plugin can coexist with or replace simpler Bevy audio solutions depending on project needs.

---

## 7. Renzora Engine Integration

[Renzora Engine](https://github.com/renzora/engine) is a modular 2D/3D game engine and scene editor built on **Bevy 0.19**. Its default audio path is Kira (`renzora_audio` + `renzora::audio_backend::Backend`).

RaMOD treats Renzora as a **first-class optional host**, not as a hard dependency of the core library.

### 7.1 Design principles

| Principle | Implication |
|-----------|-------------|
| **One-way dependency** | A Renzora game/plugin may depend on `bevy_ramod`; RaMOD does **not** depend on the Renzora monorepo (too large and workspace-bound). |
| **Bevy version alignment** | `bevy_ramod` tracks Bevy **0.19** so a Renzora project links a single Bevy major. |
| **Optional feature** | Integration code lives behind Cargo feature `renzora` on `bevy_ramod`. |
| **Replace or coexist** | Games may use RaMOD only (recommended), or drive RaMOD through Renzora’s `Backend` trait (Option B). |
| **Native plugin path** | `renzora_plugin/` packages RaMOD as a Renzora **Runtime** native plugin (marketplace id `ramod`) for SDK/marketplace install. |

### 7.2 Integration options

**Option A — RaMOD-only (recommended)**

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .replace_renzora_audio_with_ramod()  // feature = "renzora"
    .run();
```

Skips Kira. Full events, banks, spatial emitters, Live Update on TCP **:17420**.

**Option B — Renzora `Backend`**

`RamodRenzoraBackend` mirrors `renzora::audio_backend::Backend`. Zero-glue `impl Backend` lives in the consumer crate (reference: `docs/renzora_backend_glue.rs`) because RaMOD cannot hard-depend on the engine’s contract crate.

**Native plugin**

`renzora_plugin/` (`renzora_ramod`) calls Option A on load via `renzora::plugin!(…, Runtime)` so exported games ship the audio stack.

### 7.3 Shared conventions

- Bus names aligned with Studio-style routing: **Master**, **SFX**, **Music** (`bevy_ramod::renzora::buses`).
- Live Update protocol is RaMOD’s own (not FMOD’s); same port/behaviour for plain Bevy and Renzora hosts.

### 7.4 Documentation

Full operational guide: [`docs/RENZORA.md`](docs/RENZORA.md).

---

## 8. Lessons from Wwise (what we absorb, what we do not copy)

Audiokinetic **Wwise** is the other major commercial adaptive-audio middleware beside FMOD. RaMOD is **not** a Wwise clone and does **not** link the Wwise SDK. We study Wwise the same way we study FMOD: as a proven design vocabulary for game audio.

### 8.1 Concepts we deliberately absorb

| Wwise idea | RaMOD direction |
|------------|-----------------|
| **Actor-Mixer Hierarchy** | Hierarchical mixer (Master → groups/buses → voices) already in `ramod_runtime`; deepen naming and routing ergonomics over time. |
| **Game Syncs** (States, Switches, Triggers, RTPCs) | **Parameters** (RTPC-like continuous values) are first-class; discrete **states/switches** map to event branching, transition markers, and future lightweight state tables. |
| **SoundBanks** | FSB5 + `bank.json` hybrid banks today; future native bank format may borrow Wwise-style load/unload granularity without proprietary containers. |
| **Event-driven design** | Play/stop/set-parameter on named events (`event:/…`) — shared with FMOD Studio mental model. |
| **Interactive music** | Multi-track designed events, transition markers, crossfades (S5.x); expand toward segment/playlist patterns where useful. |
| **Profiling** | Bus meters, CPU/voices sparklines, 3D emitter map — Wwise Profiler–inspired, open implementation. |
| **Side-chaining / ducking** | Sidechain compressor (e.g. SFX → Music) in the mixer path. |

### 8.2 Explicit non-goals relative to Wwise

- No dependency on **Wwise SDK**, Authoring Tool, or proprietary bank formats as a required runtime.
- No pixel-perfect **Wwise Authoring Tool** UI; RaMOD Studio targets FMOD Studio *spirit* + GoldWave editing, not WAAPI-centric workflows.
- No commitment to full **Rooms & Portals** / acoustic geometry parity in v1.0 (may appear as research/later phases).
- No WAAPI-compatible remote control surface in v1.0 (Live Update is RaMOD-native).

### 8.3 Positioning vs FMOD and Wwise

```
                    Commercial          Open-source (RaMOD)
Runtime model       FMOD Core /         ramod_runtime
                    Wwise Sound Engine
Authoring           FMOD Studio /       RaMOD Studio (events)
                    Wwise Authoring     + waveform (GoldWave-style)
Bevy binding        bevy_fmod (FMOD)    bevy_ramod
Host engines        Engine-agnostic     Bevy-first; Renzora optional
License             Proprietary         GPL-3.0-or-later
```

RaMOD’s mandate remains: **ship adaptive game audio without proprietary middleware**. FMOD and Wwise define the bar for capability; they are not dependencies and not UI blueprints to replicate line-by-line.

---

## 9. Technology Choices (current direction)

| Area                    | Preferred approach                          | Notes |
|-------------------------|---------------------------------------------|-------|
| Language                | Rust 2021 / 2024                            | — |
| Foundation              | Pure `std`                                  | Keep as long as practical |
| Audio backend           | cpal / Firewheel / rodio                    | Feature-gated |
| DSP                     | Custom graph + optional crates              | Keep control |
| GUI (Editor)            | egui or iced                                | Lightweight |
| Bevy                    | Official plugin crate (`bevy_ramod`)        | Track Bevy **0.19** (Renzora-aligned); feature-gate older if needed |
| Host engines            | Bevy standalone + Renzora (optional)        | One-way dependency; see §7 |
| Serialization           | Custom binary + optional RON/JSON           | For events and projects |
| Testing                 | cargo test + fuzzing + real bank fixtures   | Critical |

---

## 10. Licensing & Community

- Core library: **GPL-3.0-or-later** (current). Dual-licensing may be considered later if it helps adoption.
- Contributions welcome under the same license.
- Code of Conduct is already present in the repository.
- Goal: grow a small but healthy community of Bevy audio developers and modders.

---

## 11. Success Metrics (v1.0)

- Stable runtime that can replace basic FMOD Core usage in a Bevy game.
- Functional waveform editor with the most common GoldWave-style operations.
- Official `bevy_ramod` plugin with good documentation and examples.
- Ability to load and play real commercial FSB5 banks.
- Positive feedback from early Bevy (and Renzora) adopters.
- Documented, optional Renzora path (`feature = "renzora"`, native plugin `ramod`).
- Clean CI, good test coverage, and published crates.

---

## 12. Document History

| Version | Date       | Changes                          |
|---------|------------|----------------------------------|
| 0.2     | 2026-10-23 | Renzora integration (§7); Wwise lessons (§8); Bevy 0.19 host note |
| 0.1     | 2026-10-18 | Initial official design document |

This is a living document. It will be updated as architecture decisions solidify and phases are completed.

---

*RaMOD — Open-source adaptive audio middleware for Bevy, Renzora, and beyond.*
