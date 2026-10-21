# S7.12 — Bus insert FX

**Status:** on `main`

## UI

Each Profiler bus row:

| Button | Effect |
|--------|--------|
| **LP** | Low-pass 2 kHz |
| **HP** | High-pass 200 Hz |
| **Comp** | Compressor (−18 dB, ratio 4, +3 dB) |
| **Rev** | Feedback delay reverb |
| **✕** | Clear all inserts on bus |

## Runtime

```rust
InsertEffect::lowpass(2000.0)
InsertEffect::highpass(200.0)
InsertEffect::compressor(-18.0, 4.0, 3.0)
InsertEffect::reverb(45.0, 0.35, 0.25)
rt.add_bus_insert(bus, effect);
rt.clear_bus_inserts(bus);
```

## Verify

```bash
cargo run -p ramod_editor
# Play → Profiler → SFX → LP or Rev → hear the filter
```
