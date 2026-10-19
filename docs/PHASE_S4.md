# Phase S4 — Mixer + snapshots

**Status:** On `main` (2026-10-19)

## Goal

Hierarchical mixer with mute/solo, insert DSP, snapshot blend, and metering.

## Done

| ID | Item |
|----|------|
| S4.1 | Bus hierarchy: Master → SFX / Music; runtime routing |
| S4.2 | Per-bus volume (dB), mute, solo |
| S4.3 | Inserts: Gain, LowPass, Compressor |
| S4.4 | `Snapshot` + `trigger_snapshot(name, weight)` |
| S4.5 | Peak/RMS `Meter` on each bus |

## API

```rust
let master = rt.mixer().master_id();
rt.set_bus_volume_db(master, -6.0);
rt.set_bus_mute(master, false);
rt.add_bus_insert(master, InsertEffect::lowpass(4000.0));

rt.capture_snapshot_base();
let mut snap = Snapshot::new("battle");
snap.set_volume(master, -3.0);
rt.register_snapshot(snap);
rt.trigger_snapshot("battle", 1.0);

let meter = rt.bus_meter(master);
```

## Test

```bash
cargo test -p ramod_runtime --no-default-features
```

## Follow-ups

- Per-voice bus routing from event metadata
- Editor Mixer window with live meters (S4.5 UI)
- More DSP (EQ bands, send/return)
