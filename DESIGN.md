# RaMOD — Official Design Document

**Version:** 0.1 (Draft)  
**Date:** 2026-10-18  
**Status:** Living document  
**Authors:** Simone Rizzo (SimonTek27) + design collaboration  
**Repository:** https://github.com/SimonTek27/RaMOD  
**License:** GPL-3.0-or-later

---

## 1. Vision

**RaMOD** aims to become a complete, open-source audio middleware for games, with a primary focus on the **Bevy** engine.

It combines three complementary capabilities:

1. **FMOD-like runtime engine**  
   Professional adaptive audio: hierarchical mixer, events, parameters, 3D spatialization, virtual voices, DSP graph, snapshots, and streaming.

2. **GoldWave-style sound editor**  
   Offline and real-time audio editing: non-destructive waveform/spectral editing, rich effects suite, batch processing, restoration tools, cue points, and analysis.

3. **Native Bevy integration**  
   Idiomatic ECS components, systems, AssetLoaders, and live-update support that feel native to Bevy developers.

The project starts from a solid foundation: a pure-Rust, zero-dependency FSB5 reader/writer/editor. It will evolve into a full middleware while remaining:

- Written primarily in pure Rust
- Free of proprietary runtimes
- Cross-platform (desktop + WASM targets)
- Suitable both as a standalone library and as a Bevy plugin

---

## 2. Goals & Non-Goals

### 2.1 Primary Goals

- Provide a high-quality, open-source alternative to commercial audio middleware for indie and mid-size teams using Bevy.
- Offer both a powerful **runtime** (playback, mixing, spatial, adaptive) and a practical **authoring/editing** toolset in the same ecosystem.
- Keep the core library dependency-light (std + carefully chosen optional crates).
- Deliver excellent developer experience: clear API, good documentation, examples, and Bevy-first ergonomics.
- Support FSB5 natively (for compatibility with existing content and modding workflows) while adding modern formats and a custom bank format if needed.

### 2.2 Non-Goals (at least for v1.0)

- Full feature parity with the commercial FMOD Studio UI (we aim for the *spirit* and core capabilities, not pixel-perfect cloning).
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
- Components: `AudioEmitter`, `AudioListener`, `RamodEventInstance`, `Bus`, etc.
- Systems that keep spatial data in sync with `Transform` and velocity
- Support for hot-reload and Live Update
- Examples and migration guides from `bevy_audio`, `bevy_kira_audio`, and `bevy_fmod`

---

## 4. Key Design Principles

1. **Pure Rust first**  
   Prefer pure-Rust implementations. Optional FFI (e.g. for legacy encoders) is allowed behind features and must degrade gracefully.

2. **Zero or minimal mandatory dependencies**  
   The foundation stays as light as possible. Heavier crates (audio backends, GUI, etc.) are optional features.

3. **Self-verifying and faithful**  
   Writers re-parse their own output. Editing preserves untouched data byte-for-byte when possible.

4. **ECS-native for Bevy**  
   The Bevy plugin should feel idiomatic: components, resources, events, and systems rather than a thin C wrapper.

5. **Progressive disclosure**  
   Simple API for common cases (play a sound, set volume). Full power available when needed (custom DSP graphs, complex events).

6. **Modding-friendly**  
   Keep excellent FSB5 support so existing banks and modding workflows continue to work.

7. **Open and inspectable**  
   Clear data structures, good error messages, profiling hooks, and documentation.

---

## 5. Feature Roadmap Overview

See [ROADMAP.md](ROADMAP.md) for the full 50-item operational plan.

### Phase 0 — Foundation (current → 0.2)
- Stabilise and complete FSB5 support
- Pure-Rust Vorbis encode/decode
- Official CLI
- crates.io publication
- Comprehensive tests + fuzzing
- Documentation (mdBook)

### Phase 1 — Runtime Core
- Audio backend (cpal / Firewheel / rodio)
- Hierarchical mixer + voices
- Basic DSP effects
- 3D spatial + Doppler
- Parameter and simple Event system
- Streaming support

### Phase 2 — GoldWave-style Editor
- Interactive waveform + spectral views
- Non-destructive editing operations
- Effects suite and batch processing
- Cue points, merger/splitter
- GUI (egui/iced) + headless mode

### Phase 3 — Studio-like Authoring
- Event Designer (timeline, instruments, automation)
- Adaptive music support
- Live Update protocol
- Advanced bank building and platform optimisation
- Snapshot system

### Phase 4 — Bevy First-Class Support
- `bevy_ramod` plugin
- Full ECS integration
- Asset pipeline and hot-reload
- Official examples and migration guides
- Interoperability with future Bevy audio engine

---

## 6. Bevy Integration Design Notes

The Bevy plugin should expose:

```rust
// Example conceptual API (subject to change)
commands.spawn((
    AudioEmitter::new("event:/weapons/gunshot"),
    Transform::from_xyz(1.0, 0.0, 2.0),
));

// Listener
commands.spawn((
    Camera3dBundle::default(),
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

## 7. Technology Choices (current direction)

| Area                    | Preferred approach                          | Notes |
|-------------------------|---------------------------------------------|-------|
| Language                | Rust 2021 / 2024                            | — |
| Foundation              | Pure `std`                                  | Keep as long as practical |
| Audio backend           | cpal / Firewheel / rodio                    | Feature-gated |
| DSP                     | Custom graph + optional crates              | Keep control |
| GUI (Editor)            | egui or iced                                | Lightweight |
| Bevy                    | Official plugin crate                       | Match Bevy release cadence |
| Serialization           | Custom binary + optional RON/JSON           | For events and projects |
| Testing                 | cargo test + fuzzing + real bank fixtures   | Critical |

---

## 8. Licensing & Community

- Core library: **GPL-3.0-or-later** (current). Dual-licensing may be considered later if it helps adoption.
- Contributions welcome under the same license.
- Code of Conduct is already present in the repository.
- Goal: grow a small but healthy community of Bevy audio developers and modders.

---

## 9. Success Metrics (v1.0)

- Stable runtime that can replace basic FMOD Core usage in a Bevy game.
- Functional waveform editor with the most common GoldWave-style operations.
- Official `bevy_ramod` plugin with good documentation and examples.
- Ability to load and play real commercial FSB5 banks.
- Positive feedback from early Bevy adopters.
- Clean CI, good test coverage, and published crates.

---

## 10. Document History

| Version | Date       | Changes                          |
|---------|------------|----------------------------------|
| 0.1     | 2026-10-18 | Initial official design document |

This is a living document. It will be updated as architecture decisions solidify and phases are completed.

---

*RaMOD — Open-source adaptive audio middleware for Bevy and beyond.*
