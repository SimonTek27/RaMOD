# Phase S2 — Event model

**Status:** Scaffold on `main` (2026-10-19)

## Goal

Studio-like API: play events by path, parameters, 3D, virtual voices — not only raw PCM handles.

## Done

| ID | Item |
|----|------|
| S2.1 | `EventDescription` + `EventInstance` (create / start / stop / release) |
| S2.2 | One-shot + looping single instrument |
| S2.3 | Named float parameters |
| S2.4 | Listener + emitter, distance attenuation |
| S2.5 | Doppler from velocity |
| S2.6 | Virtual voice steal (priority × audibility) |
| S2.7 | `Bank` / `BankEventSpec` registry (PCM paths; FSB5 when `fsb.rs` restored) |

## API

```rust
use ramod_runtime::*;

let mut rt = Runtime::new(RuntimeConfig { open_device: false, ..Default::default() })?;
let buf = rt.load_pcm(pcm, 1, 48_000)?;

rt.register_event(
    EventDescription::new("event:/sfx/explosion", buf)
        .with_3d(Attenuation { min_distance: 1.0, max_distance: 40.0 })
        .with_parameter("Intensity", 1.0)
        .with_priority(200),
);

rt.set_listener(Attributes3d {
    position: Vec3::new(0.0, 0.0, 0.0),
    ..Default::default()
});

let mut inst = rt.create_instance("event:/sfx/explosion")?;
inst.set_parameter("Intensity", 0.8);
inst.set_3d(Attributes3d {
    position: Vec3::new(10.0, 0.0, 0.0),
    velocity: Vec3::new(-5.0, 0.0, 0.0),
    ..Default::default()
});
let id = rt.start_instance(inst)?;
// ... render / tick ...
rt.stop_instance(id);
rt.release_instance(id);
```

## Test

```bash
cargo test -p ramod_runtime --no-default-features
```

## Follow-ups

- Parameter → gain/pitch automation curves
- Multi-instrument / scatter
- FSB5 bank load via `ramod` after `src/fsb.rs` restore
- Wire instances into cpal callback
