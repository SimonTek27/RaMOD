# S7.14 — Doppler from 3D map drag

**Status:** on `main`

## Interaction

| Gesture | Effect |
|---------|--------|
| Drag emitter | position + velocity → Doppler pitch |
| Drag listener | same for listener velocity |
| Release | velocity → 0 (pitch back to 1.0) |

Velocity is derived from egui `drag_delta` scaled to world units (boost ×12 for audible effect).

## Pipeline

```
map drag → SetEmitterPos{x,z,vx,vz}
  → Attributes3d { position, velocity }
  → Runtime::set_instance_3d
  → sync_event_spatial_gains → doppler_pitch(closing_speed)
  → VoicePool::set_pitch
```

## Verify

```bash
cargo run -p ramod_editor
# Event ☑ 3D → Play → drag emitter toward/away from L → pitch shifts
```
