# S7.5 — Bus volume drag in Profiler

**Status:** on `main`

## UI

Each bus row: **Name | M | S | ±dB drag | peak bar**

Volume range: **-48 … +12 dB** (DragValue).

## API

```rust
audio.set_bus_volume_db(bus_id, db);
// → Runtime::set_bus_volume_db(BusId, f32)
```

## Verify

```bash
cargo run -p ramod_editor
# Play → drag SFX volume down → quieter routing bus
```
