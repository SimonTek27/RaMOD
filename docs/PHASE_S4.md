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

## Runtime API

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

Tests: `bus_mute_silences_output`, `snapshot_blends_volume`, `meter_updates_after_render`.

## Follow-ups

- Per-event bus routing
- Editor Mixer window with live meter bars
- EQ / send-return
