# S7.3 — Per-bus meters in Profiler

**Status:** on `main`

## Runtime

```rust
rt.list_bus_meters() -> Vec<BusMeterInfo>
// id, name, peak, rms, mute, solo, volume_db
```

## Editor

Profiler panel **Buses** section: one row per mixer bus with peak bar, mute/solo flags, volume dB.

Default mixer buses: Master + SFX (and any added via `add_bus`).

## Verify

```bash
cargo run -p ramod_editor
# Play — Profiler shows Master / SFX peak bars
```
