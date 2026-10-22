# S7.22 — Send / Return buses

**Status:** mixer + API on `main` (runtime_core_b partially restored; full blob in `artifacts/runtime_core_b_S722_RESTORE.rs`)

## Model

```
SFX  ──send %──► Reverb (return + Rev insert) ──► Master
Music ─send %──►
SFX / Music ──────────────────────────────► Master (dry)
```

Post-fader sends. Default buses: Master, SFX, Music, **Reverb** (`is_return`).

## API

```rust
rt.set_bus_send_level(BusId(sfx), "Reverb", 0.25);
// BusMeterInfo { send_reverb, is_return, ... }
```

## Profiler

Non-return rows: DragValue **`%→Rev`** (0–100).

## Recovery note

If `runtime_core_b.rs` is incomplete after a partial push, restore from commit
`f5f32b57` then re-apply `set_bus_send_level` / `send_reverb` fields, or copy
`runtime_core_b_S722_RESTORE.rs` from the project artifacts folder.
