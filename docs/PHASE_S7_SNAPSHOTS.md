# S7.9 — Bus snapshots (FMOD-style)

**Status:** on `main`

## Workflow

1. Set bus volumes (Profiler DragValue)
2. **Capture base** — store current levels as blend origin
3. Change volumes for a state (e.g. quieter SFX, louder Music)
4. **Save** with a name (`Combat`, `Menu`, …)
5. **50% / 100%** — blend toward that snapshot
6. **Clear active** — remove blend and restore base volumes

## API

```rust
rt.capture_snapshot_base();
rt.capture_named_snapshot("Combat");
rt.trigger_snapshot("Combat", 1.0);
rt.clear_snapshot();
rt.snapshot_names() -> Vec<String>
```

## Verify

```bash
cargo run -p ramod_editor
# Set Music -6 dB → Capture base → Music +6 → Save "Combat" → 100%
```
