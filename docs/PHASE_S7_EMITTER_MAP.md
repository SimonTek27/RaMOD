# S7.10 — 3D emitter map (Profiler)

**Status:** on `main`

## UI

Profiler panel → **3D emitters (XZ top-down)**:

- Green **L** = listener
- Orange dots = emitters (radius ∝ audibility)
- Lines listener → emitter
- Scale ≈ ±50 m

## Data

```rust
ProfilerSnapshot.emitters: Vec<EmitterDebug>
ProfilerSnapshot.listener: (x, y, z)
```

Audibility in the frame uses distance attenuation × gain.

## Verify

```bash
cargo run -p ramod_editor
# Event → ☑ 3D → Play → Profiler shows emitter at origin + listener
```
