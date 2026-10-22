# S7.22 — Send / Return buses

**Status:** on `main`

## Model

```
SFX ──send──► Reverb (return, has Rev insert) ──► Master
Music ─send─►
SFX ──────────────────────────────► Master
```

- **Post-fader** sends: after volume + inserts on the source bus
- Default buses: Master, SFX, Music, **Reverb** (`is_return`, preloaded reverb insert)
- Send level linear 0..=1 (UI: 0–100% →Rev)

## API

```rust
rt.set_bus_send_level(BusId(sfx), "Reverb", 0.25);
mixer.send_level(id, "Reverb");
```

## Profiler

Each non-return bus row: DragValue **`%→Rev`**.

## Verify

```bash
cargo run -p ramod_editor
# Play SFX → raise SFX %→Rev to 40 → hear wet tail on Reverb meter
```
