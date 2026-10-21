# S7.15 — FX presets (project)

**Status:** on `main`

## UI (Profiler)

1. Add inserts on buses (LP / HP / Comp / Rev)
2. Name → **Save FX**
3. **Apply** / **Del** on listed presets

Presets persist in `project.ramod.json` under `fx_presets`.

## Schema

```json
{
  "fx_presets": [
    {
      "name": "Combat",
      "inserts": [
        { "bus": "SFX", "kind": "LowPass", "p0": 2000, "p1": 0, "p2": 0 },
        { "bus": "SFX", "kind": "Reverb", "p0": 45, "p1": 0.35, "p2": 0.25 }
      ]
    }
  ]
}
```

## Verify

```bash
cargo run -p ramod_editor
# Play → SFX Rev+LP → Save FX "Combat" → Clear ✕ → Apply Combat
# File → Save project → reload
```
