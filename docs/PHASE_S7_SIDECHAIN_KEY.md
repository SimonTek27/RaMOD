# S7.18 — Selectable side-chain key bus

**Status:** on `main`

## UI

Profiler → Sidechain insert row → **key:** combo (`SFX` / `Music` / `Master`).

Changing the key rewrites the insert via `UpdateInsert` → `to_effect` with `source_bus`.

## Schema

```json
{
  "kind": "Sidechain",
  "p0": -24,
  "p1": 6,
  "p2": 0,
  "source_bus": "SFX"
}
```

Presets (`fx_presets`) persist `source_bus`.

## Verify

```bash
cargo run -p ramod_editor
# Music → SC → change key to Master → Play SFX/Music and hear ducking source change
```
