# S7.4 — Clickable bus Mute / Solo in Profiler

**Status:** on `main`

## UI

Profiler **Buses** rows: **M** / **S** buttons toggle mute and solo.

- Mute active → red `M●`
- Solo active → yellow `S●`

## API

```rust
audio.toggle_bus_mute(bus_id);
audio.toggle_bus_solo(bus_id);
// → Runtime::set_bus_mute / set_bus_solo
```

## Verify

```bash
cargo run -p ramod_editor
# Play → click M on SFX → silence routed to SFX; S solos that bus
```
