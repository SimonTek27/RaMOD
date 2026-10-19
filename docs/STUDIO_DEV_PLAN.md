# RaMOD — FMOD Studio–Class Development Plan

**Version:** 1.0  
**Date:** 2026-10-19  
**Status:** Canonical execution plan  
**Goal:** Evolve RaMOD from FSB5 foundation into an **open-source FMOD Studio–class** stack (runtime + authoring + Bevy), without proprietary dependencies.

**Related docs:** [DESIGN.md](../DESIGN.md) · [ARCHITECTURE.md](../ARCHITECTURE.md) · [ROADMAP.md](../ROADMAP.md) · [UI_REFERENCE.md](UI_REFERENCE.md) · [PHASE0.md](PHASE0.md)

---

## 1. North star

A Bevy (and standalone) team can:

1. Design adaptive audio in a **Studio-like Event Editor** (timeline, instruments, parameters).
2. Mix through a **hierarchical mixer** with snapshots.
3. **Play and spatialize** events from game code / ECS.
4. **Live-update** content while the game runs.
5. Ship with **only open-source** audio middleware (no FMOD license, no closed DLLs on the critical path).

**Non-goal:** Pixel-perfect clone of FMOD Studio UI or 100% API parity on day one. Match *capabilities and workflow*, not trademarks or proprietary formats as the sole source of truth.

---

## 2. Product pillars (what “FMOD Studio class” means)

| Pillar | FMOD equivalent | RaMOD delivery |
|--------|-----------------|----------------|
| **P1 Content foundation** | Sample banks, codecs | FSB5 + native banks, pure-Rust encode/decode |
| **P2 Runtime core** | FMOD Core / Studio API | `ramod_runtime`: mixer, voices, events, 3D, DSP, params |
| **P3 Authoring – samples** | (partially external tools) | GoldWave-style waveform editor |
| **P4 Authoring – events** | Event Editor | Timeline, instruments, automation |
| **P5 Authoring – mix** | Mixer + snapshots | Bus tree, metering, snapshot system |
| **P6 Live iteration** | Live Update + Profiler | Protocol + session capture |
| **P7 Engine integration** | Unity/UE plugins | `bevy_ramod` first-class |

---

## 3. Program phases (execution order)

Critical path: **Foundation → Runtime that makes sound → Editor bound to runtime → Studio features → Bevy**.

```
Phase S0  Foundation lock          (now → weeks)
Phase S1  Runtime MVP              (must hear audio)
Phase S2  Event model + API        (play event by name)
Phase S3  Editor vertical slice    (UI drives runtime)
Phase S4  Mixer + snapshots
Phase S5  Full Event Designer
Phase S6  Live Update
Phase S7  Profiler + polish
Phase S8  Bevy first-class
Phase S9  Waveform suite (parallel after S1)
```

Phases S0–S3 are **serial**. S4–S7 can partially overlap after S3. S9 can start after S1 in parallel with S4+.

---

## 4. Phase detail

### Phase S0 — Foundation lock

**Objective:** Production-ready content layer; no proprietary encoder required.

| ID | Deliverable | Exit check |
|----|-------------|------------|
| S0.1 | Pure-Rust Vorbis encode with setup CRC in bundled table | `encode_to_bank` works on Linux/macOS/Windows CI |
| S0.2 | Pure-Rust Vorbis decode (e.g. `lewton` feature) | PCM from FSB5 VORBIS samples |
| S0.3 | CLI complete: list, info, extract, build, edit | Documented in README |
| S0.4 | Rich errors + fixture tests + optional fuzz | CI green |
| S0.5 | crates.io `ramod` 0.2 | Published |

**Depends on:** current repo.  
**Blocks:** all runtime playback of Vorbis banks.

---

### Phase S1 — Runtime MVP (“it makes sound”)

**Objective:** Headless engine plays PCM/Vorbis through a device.

| ID | Deliverable | Exit check |
|----|-------------|------------|
| S1.1 | Crate `ramod_runtime` | Workspace member |
| S1.2 | Audio backend feature (`cpal` and/or Firewheel) | Output on desktop |
| S1.3 | Sample voice: load buffer, play, stop, volume, loop | Unit + manual test |
| S1.4 | Simple master bus + gain | |
| S1.5 | `tick(dt)` pump | Stable callback / thread model documented |

**API sketch:**

```rust
let mut rt = Runtime::new(Default::default())?;
let buf = rt.load_pcm(...);
let v = rt.play(buf, PlayParams { looped: false, gain: 1.0 });
rt.tick(dt);
```

**Blocks:** editor audition, Bevy plugin, events.

---

### Phase S2 — Event model + Studio-like API surface

**Objective:** Designer-facing *events*, not only raw samples.

| ID | Deliverable | Exit check |
|----|-------------|------------|
| S2.1 | EventDescription + EventInstance | create / start / stop / release |
| S2.2 | One-shot + looping instruments (single sample) | |
| S2.3 | Named parameters (float) + set/get | |
| S2.4 | Basic 3D: listener + emitter attributes, distance attenuation | |
| S2.5 | Doppler from velocity | |
| S2.6 | Virtual voices (priority / audibility steal) | Stress test N emitters |
| S2.7 | Bank load API (FSB5 samples mapped to events — simple scheme first) | |

**API sketch (mirrors Studio mental model):**

```rust
let ev = rt.get_event("event:/sfx/explosion")?;
let mut inst = ev.create_instance()?;
inst.set_parameter("Intensity", 0.8)?;
inst.set_3d(pos, vel)?;
inst.start()?;
```

**Blocks:** meaningful Event Editor, Live Update targets.

---

### Phase S3 — Editor vertical slice

**Objective:** `ramod_editor` is not a mockup: **Play auditions a real event**.

| ID | Deliverable | Exit check |
|----|-------------|------------|
| S3.1 | Project model on disk (RON/JSON): events, paths to samples/banks | Open/save |
| S3.2 | Browser lists real events/assets from project | |
| S3.3 | Transport bound to `ramod_runtime` | Hear output |
| S3.4 | Load FSB5/WAV into Assets tab | |
| S3.5 | Minimal event: one track, one single-instrument, play/stop | Demo video |

**UI:** Keep layout from [UI_REFERENCE.md](UI_REFERENCE.md); fill data, don’t redesign.

---

### Phase S4 — Mixer + snapshots

**Objective:** FMOD Mixer–class control of the mix graph.

| ID | Deliverable | Exit check |
|----|-------------|------------|
| S4.1 | Bus hierarchy UI + runtime routing | |
| S4.2 | Per-bus volume, mute, solo | |
| S4.3 | Insert effect chain (Gain, EQ, Filter, Compressor — minimal set) | |
| S4.4 | Snapshot definition + blend in runtime | Trigger from API |
| S4.5 | Basic metering (peak/RMS) in UI | |

**New UI window:** Mixer (can be second viewport or tab).

---

### Phase S5 — Full Event Designer

**Objective:** Competitive with FMOD Event Editor for indie/mid workflows.

| ID | Deliverable | Exit check |
|----|-------------|------------|
| S5.1 | Multi-track timeline | |
| S5.2 | Instrument types: single, multi, scatter (basic) | |
| S5.3 | Parameter sheets + automation curves | |
| S5.4 | Randomization / probability / loop regions | |
| S5.5 | AHDSR / simple modulators | |
| S5.6 | Transition markers (basic adaptive music) | |
| S5.7 | 3D event properties in Overview | |
| S5.8 | Build → bank export (native + FSB5 where applicable) | |

---

### Phase S6 — Live Update

**Objective:** Edit while the game runs.

| ID | Deliverable | Exit check |
|----|-------------|------------|
| S6.1 | Protocol (TCP or WebSocket): parameter, volume, mute, hot-swap event def | Spec doc |
| S6.2 | Runtime server feature `live-update` | |
| S6.3 | Editor client + status bar “Live Update On” real | |
| S6.4 | Bevy example connecting to editor | |

**Constraint:** Same honesty as FMOD — structural renames may require rebuild; document limits.

---

### Phase S7 — Profiler + polish

| ID | Deliverable |
|----|-------------|
| S7.1 | Capture: active events, params, CPU time, voice count |
| S7.2 | Timeline view of a recorded session |
| S7.3 | Optional 3D debug view of emitters |
| S7.4 | Spectrum meter (DSP or UI-only) |

---

### Phase S8 — Bevy first-class

| ID | Deliverable |
|----|-------------|
| S8.1 | `bevy_ramod` plugin (pattern from bevy_fmod, pure runtime) |
| S8.2 | Components: `AudioEmitter`, `AudioListener`, `Velocity` |
| S8.3 | AssetLoader for banks/events |
| S8.4 | Examples + migration guide from `bevy_fmod` / `bevy_kira_audio` |

---

### Phase S9 — Waveform / GoldWave suite (parallel track)

Can proceed after S1 without blocking Studio event work.

| ID | Deliverable |
|----|-------------|
| S9.1 | Waveform view + selection |
| S9.2 | Non-destructive cut/copy/paste/silence/reverse |
| S9.3 | Core effects + undo |
| S9.4 | Export into project Assets / banks |

---

## 5. Milestone releases

| Release | Phase gate | User-visible outcome |
|---------|------------|----------------------|
| **0.2** | S0 | Solid FSB5 crate + CLI + pure-Rust codecs |
| **0.3** | S1–S2 | Headless runtime: play events in 3D from Rust |
| **0.4** | S3 | Editor plays a real one-instrument event |
| **0.5** | S4–S5 (partial) | Mixer + multi-track events |
| **0.6** | S6–S8 | Live Update + `bevy_ramod` |
| **1.0** | S5–S7 complete enough | Studio-class indie workflow documented |

---

## 6. Team / workstream split (if multiple contributors)

| Stream | Owner focus | Phases |
|--------|-------------|--------|
| **Runtime** | Audio thread, DSP, 3D, voices | S1, S2, S4, S7 |
| **Formats** | FSB5, banks, encode/decode | S0, S5.8 |
| **Editor** | egui UX, project model | S3, S5, S9 |
| **Net** | Live Update protocol | S6 |
| **Bevy** | ECS plugin, assets | S8 |

---

## 7. Technical principles (non-negotiable)

1. **No proprietary runtime** on the default path (FMOD DLLs never required).
2. **Runtime is source of truth** for audition — editor never “fakes” sound differently from game.
3. **ECS-friendly API** for Bevy; C ABI optional later.
4. **FSB5 remains first-class** for modding; native bank format for events/parameters when FSB5 is insufficient.
5. **Feature flags** for heavy deps (backend, encoder, live-update, editor).
6. **UI_REFERENCE.md** layout stays stable; avoid redesign churn.

---

## 8. Risk register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Vorbis setup CRC mismatch with `rusty_vorbis` | Encode unusable for FSB5 | Capture setup → extend header table; test in CI |
| Audio thread / Bevy scheduling complexity | Glitches, priority inversions | Isolate runtime thread; clear tick contract |
| Scope creep toward full FMOD parity | Never ship | Strict phase exits; cut adaptive music depth for 1.0 if needed |
| egui performance on large timelines | Poor UX | Virtualize tracks; profile early in S5 |
| Live Update security | Remote code/content injection | Localhost default; opt-in bind address |

---

## 9. Definition of “FMOD Studio class” for v1.0

Must-have:

- [ ] Event Editor: multi-track, parameters, automation, audition
- [ ] Runtime: mixer, virtual voices, 3D, parameters, snapshots (basic)
- [ ] Bank build + load in game
- [ ] Live Update for parameters and mix (documented limits)
- [ ] `bevy_ramod` with spatial components
- [ ] Pure open-source default path

Nice-to-have for 1.0 (can slip to 1.x):

- Full profiler suite
- Advanced adaptive music transitions
- Programmer instruments / complex logic boxes
- Waveform restoration tools
- Console-specific encoders

---

## 10. Immediate next actions (this week)

1. **S0.1** — Verify/fix `rusty_vorbis` setup CRC against bundled table; extend table if needed.  
2. **S0.2** — Add optional `lewton` decode path.  
3. **S1.1–S1.3** — Skeleton `ramod_runtime` + cpal play of a WAV/PCM buffer.  
4. **S3.3 (prep)** — Wire editor Play button to runtime once S1 works (even if event model is still “one sample”).

---

*This plan supersedes informal prioritization. Update ROADMAP.md checkboxes as phase IDs complete; keep this document as the Studio-track strategy.*
