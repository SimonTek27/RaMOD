# RaMOD — Operational Roadmap

**Version:** 0.1  
**Date:** 2026-10-18  
**Status:** Living document  
**Aligned with:** [DESIGN.md](DESIGN.md) · [ARCHITECTURE.md](ARCHITECTURE.md)  
**Repository:** https://github.com/SimonTek27/RaMOD

This roadmap translates the design vision into **50 concrete, actionable items**.  
Each item is intended to be trackable as a GitHub Issue or Milestone task.

**Legend**  
- **P0** = Critical / Foundation  
- **P1** = High priority (next release)  
- **P2** = Medium (important but can wait)  
- **P3** = Nice-to-have / later  

**Status**  
- `[ ]` Not started  
- `[~]` In progress  
- `[x]` Done  

---

## Phase 0 — Foundation (stabilize current FSB5 core)

**Goal:** Make the existing pure-Rust FSB5 library production-ready, publishable, and extensible.

| # | Priority | Status | Item | Notes |
|---|----------|--------|------|-------|
| 1 | P0 | [ ] | Complete pure-Rust Vorbis **encoder** (remove dependency on `libfsbvorbis64.dll`) | Critical for cross-platform |
| 2 | P0 | [ ] | Complete pure-Rust Vorbis **decoder** | Needed for playback without external crates |
| 3 | P0 | [ ] | Improve error types: richer context, less stringly-typed errors | `FsbError`, `VorbisError`, etc. |
| 4 | P0 | [ ] | Expand test suite with real commercial FSB5 banks (fixtures + round-trip tests) | Keep fixtures private or synthetic if needed |
| 5 | P0 | [ ] | Add cargo-fuzz targets for parser and Ogg muxer/demuxer | Security & robustness |
| 6 | P1 | [ ] | Official CLI (`ramod`) with subcommands: `list`, `info`, `extract`, `build`, `edit`, `convert` | Make the tool usable outside the library |
| 7 | P1 | [ ] | Publish crate on **crates.io** (`ramod`) | Version 0.1.0 or 0.2.0 |
| 8 | P1 | [ ] | Write mdBook documentation (or docs.rs + examples) | User & developer docs |
| 9 | P1 | [ ] | Support PCM24 and PCMFLOAT write paths fully | Currently strongest on PCM16 |
| 10 | P2 | [ ] | Graceful handling / pass-through of unsupported codecs (XMA, ADPCM, CELT, AT9…) | Don’t crash on unknown mode |
| 11 | P2 | [ ] | Fuzz + property-based tests for BankEditor (in-place edits preserve untouched data) | Faithfulness guarantee |
| 12 | P2 | [ ] | C ABI / cdylib bindings for non-Rust tools | Optional feature |

**Exit criteria for Phase 0:**  
Crate published, pure-Rust encode/decode working, solid tests, usable CLI, good docs.

---

## Phase 1 — Runtime Core (FMOD Core equivalent)

**Goal:** A real-time audio engine that can play, mix, spatialize and parameterize sounds.

| # | Priority | Status | Item | Notes |
|---|----------|--------|------|-------|
| 13 | P0 | [ ] | Choose and integrate audio backend (cpal / Firewheel / rodio) behind a feature flag | Prefer Firewheel if Bevy direction aligns |
| 14 | P0 | [ ] | Hierarchical mixer: Master → Groups → Buses → Channels | Core of FMOD-like design |
| 15 | P0 | [ ] | Voice management + **virtual voice** system (audibility-based) | Essential for performance |
| 16 | P0 | [ ] | Sample playback (memory-decoded + streaming) | Support PCM + Vorbis first |
| 17 | P1 | [ ] | Basic DSP graph (Gain, Pan, Simple EQ, Low/High-pass) | Modular node system |
| 18 | P1 | [ ] | 3D spatial audio: position, orientation, distance attenuation curves | Listener + emitters |
| 19 | P1 | [ ] | Doppler effect based on relative velocity | |
| 20 | P1 | [ ] | Parameter system (named float parameters + basic automation) | Foundation for events |
| 21 | P1 | [ ] | Simple Event system: one-shot, looping, multi-instrument, randomization | “play event by name” |
| 22 | P2 | [ ] | Streaming support for large files / music | |
| 23 | P2 | [ ] | Basic occlusion / obstruction model | Can start simple (raycast or distance-based) |
| 24 | P2 | [ ] | Profiling hooks (CPU time, active voices, bus levels) | |
| 25 | P2 | [ ] | Snapshot system (save/restore mixer state) | |
| 26 | P3 | [ ] | Side-chain / send buses | Advanced mixing |

**Exit criteria for Phase 1:**  
Can load a bank, play events in 3D space, change parameters, and mix through a hierarchical bus structure from pure Rust (and later from Bevy).

---

## Phase 2 — GoldWave-style Sound Editor

**Goal:** Practical offline (and real-time preview) audio editing tools.

| # | Priority | Status | Item | Notes |
|---|----------|--------|------|-------|
| 27 | P0 | [ ] | Interactive waveform display with zoom, selection, markers | Core of the editor |
| 28 | P0 | [ ] | Non-destructive editing operations: Cut, Copy, Paste, Delete, Silence, Reverse | |
| 29 | P1 | [ ] | Mix and Crossfade operations | |
| 30 | P1 | [ ] | Unlimited Undo / Redo stack | |
| 31 | P1 | [ ] | Spectral / spectrogram view | GoldWave-style analysis |
| 32 | P1 | [ ] | Core effects suite: EQ, Compressor/Limiter, Normalize, Fade In/Out, Pitch, Time-stretch | |
| 33 | P1 | [ ] | Restoration tools: Noise Reduction, De-click / De-pop | |
| 34 | P2 | [ ] | Additional effects: Reverb, Delay, Echo, Flanger, Chorus, Phaser | |
| 35 | P2 | [ ] | Batch processing (folder → chain of effects → export) | |
| 36 | P2 | [ ] | Cue points / regions / markers management | |
| 37 | P2 | [ ] | File Merger and Splitter tools | |
| 38 | P3 | [ ] | Expression evaluator (tone / filter generation) | Nice-to-have |
| 39 | P3 | [ ] | Real-time effect preview while editing | |

**Exit criteria for Phase 2:**  
A usable standalone or embedded editor that can open, edit, process and save audio (and feed results back into banks).

---

## Phase 3 — Studio-like Authoring & Advanced Features

**Goal:** Move from “play samples” to “design adaptive audio”.

| # | Priority | Status | Item | Notes |
|---|----------|--------|------|-------|
| 40 | P1 | [ ] | Event Designer: multi-track timeline, instruments, basic automation curves | Light FMOD Studio |
| 41 | P1 | [ ] | Adaptive music support (stems, layers, transitions) | |
| 42 | P2 | [ ] | Live Update protocol (editor ↔ running game) | Very valuable for Bevy workflow |
| 43 | P2 | [ ] | Advanced bank builder (compression options, platform variants) | |
| 44 | P2 | [ ] | Preset library and shared effect/parameter presets | |
| 45 | P3 | [ ] | Localization / multi-bank support | |

**Exit criteria for Phase 3:**  
Sound designers can create events with parameters and adaptive behaviour and push live changes into a running game.

---

## Phase 4 — Bevy First-Class Integration

**Goal:** Make RaMOD the natural choice for serious audio in Bevy projects.

| # | Priority | Status | Item | Notes |
|---|----------|--------|------|-------|
| 46 | P0 | [ ] | Create official `bevy_ramod` crate / plugin | Match Bevy version cadence |
| 47 | P0 | [ ] | AssetLoader for banks and events | Integrate with `bevy_asset` |
| 48 | P1 | [ ] | Idiomatic ECS components (`AudioEmitter`, `AudioListener`, `RamodEventInstance`, `Bus`…) | |
| 49 | P1 | [ ] | Systems that keep spatial data in sync with `Transform` + velocity | Automatic Doppler & positioning |
| 50 | P1 | [ ] | Official examples + migration guides (from `bevy_audio`, `bevy_kira_audio`, `bevy_fmod`) | Documentation & adoption |

**Additional recommended follow-ups after #50:**
- Hot-reload support
- WASM / web target validation
- Interoperability layer with future Bevy upstream audio (Firewheel)
- Performance benchmarks vs existing solutions

---

## Suggested Release Plan

| Version | Focus | Key items | Target |
|---------|-------|-----------|--------|
| **0.2** | Foundation solid | 1–12 | Short term |
| **0.3** | Runtime MVP | 13–21 | Medium term |
| **0.4** | Editor MVP | 27–33 | Medium term |
| **0.5** | Events + Spatial polish | 18–26, 40 | Later |
| **0.6+** | Bevy plugin + Live Update | 42, 46–50 | Later |
| **1.0** | Stable API + docs + examples | All P0/P1 done | Goal |

---

## How to use this roadmap

1. Turn high-priority items into GitHub Issues (one issue per item or small group).
2. Create Milestones that match the Phases or the version plan above.
3. Update the Status column in this file (or link to issues) as work progresses.
4. Keep DESIGN.md as the “why”, this ROADMAP.md as the “what & when”, ARCHITECTURE.md as the “how”.

---

*Last updated: 2026-10-18*  
*This is a living document — adjust priorities as real usage and feedback arrive.*
