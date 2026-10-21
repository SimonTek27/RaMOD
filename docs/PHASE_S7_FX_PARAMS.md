# S7.13 — Editable bus FX parameters

**Status:** on `main`

## UI

After adding LP / HP / Comp / Rev on a bus row:

| Kind | Controls |
|------|----------|
| LowPass / HighPass | cutoff Hz |
| Compressor | threshold, ratio, makeup |
| Reverb | delay ms, feedback, wet |
| Gain | dB |

**×** removes that insert slot.

## API

```rust
rt.list_bus_inserts(bus);
rt.set_bus_insert(bus, index, effect);
rt.remove_bus_insert(bus, index);
```

## Verify

```bash
cargo run -p ramod_editor
# Play → SFX → Rev → drag wet / delay → hear change
```
