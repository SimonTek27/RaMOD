# Phase S8 — Bevy first-class

**Status:** On `main` (2026-10-19)

## Goal

First-class Bevy integration without proprietary FMOD.

## Done

| ID | Item |
|----|------|
| S8.1 | `bevy_ramod::RamodPlugin` + `RamodAudio` resource |
| S8.2 | `AudioEmitter`, `AudioListener`, `Velocity`, `SpatialAudioBundle` |
| S8.3 | `RamodBankAsset` loader for `*.ramodbank.json` |
| S8.4 | Example `basic` + [BEVY_MIGRATION.md](BEVY_MIGRATION.md) |

## Run

```bash
git pull
cargo run -p bevy_ramod --example basic
```

## Systems (PostUpdate chain)

1. `sync_listener` — listener transform → runtime
2. `start_emitters` — `PlayOnSpawn` → `play_event`
3. `sync_emitters_3d` — emitter positions / velocity
4. `audio_tick_render` — `tick` + `render`

## Follow-ups

- Wire bank events to real PCM / FSB5 samples
- Live Update from Bevy (`live-update` feature)
- Profiler gizmo for emitters
