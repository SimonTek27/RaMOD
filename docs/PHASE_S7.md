# Phase S7 — Profiler + polish

**Status:** On `main` (2026-10-19)

## Goal

See what the runtime is doing: voices, CPU, events, spectrum, 3D emitters.

## Done

| ID | Item |
|----|------|
| S7.1 | `ProfilerFrame` / `ProfilerSession` capture |
| S7.2 | Session NDJSON timeline export |
| S7.3 | `EmitterDebug` positions on frames |
| S7.4 | `SpectrumAnalyzer` magnitude bands |

## API

```rust
rt.profiler_enable(true);
// each game frame:
rt.tick(dt);
rt.render(&mut out); // records CPU + state when enabled

if let Some(f) = rt.profiler_latest() {
    println!("voices={} cpu_us={}", f.active_voices, f.cpu_render_us);
}
let ndjson = rt.profiler_session().to_ndjson();
rt.spectrum().levels // 0..1 band meters
```

## Test

```bash
cargo test -p ramod_runtime --no-default-features
```

## Follow-ups

- Editor Profiler panel (timeline graph)
- Full FFT spectrum
- Bevy gizmo 3D emitters (S8)
