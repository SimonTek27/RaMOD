# S7.16 — Designed multi-track Doppler / 3D

**Status:** on `main`

## Behavior

`sync_event_spatial_gains` now also walks `designed_playbacks`:

1. Resolve `EventInstance` by `instance_id`
2. Compute distance attenuation + Doppler pitch (same as single-voice events)
3. Apply `set_gain` / `set_pitch` to **every** instrument voice on that playback

Dragging the emitter on the Profiler map while a multi-track event plays shifts pitch on all tracks.

## Verify

```bash
cargo run -p ramod_editor
# Multi-track event ☑ 3D → Play → drag emitter → all tracks Doppler
```
