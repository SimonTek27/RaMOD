# S7.12 — Bus insert FX (Profiler)

**Status:** on `main`

## UI

Per bus row in Profiler:

| Button | Effect |
|--------|--------|
| **LP** | Low-pass 2 kHz |
| **HP** | High-pass 200 Hz |
| **Comp** | Compressor −18 dB / 4:1 |
| **Rev** | Delay reverb 45 ms |
| **✕** | Clear all inserts on bus |

Shows `N ins` when inserts are active.

## API

```rust
rt.add_bus_insert(bus, InsertEffect::lowpass(2000.0));
rt.clear_bus_inserts(bus);
rt.bus_insert_count(bus);
```

## Verify

```bash
cargo run -p ramod_editor
# Play → Profiler → SFX → Rev / LP → hear difference → ✕
```
