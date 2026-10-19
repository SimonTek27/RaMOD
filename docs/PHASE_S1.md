# Phase S1 — Runtime MVP

**Status:** Scaffold on `main` (2026-10-19)

## Goal

Hear audio from pure Rust without FMOD: load PCM, play voices, mix, master gain.

## Done

| ID | Item | Notes |
|----|------|-------|
| S1.1 | Crate `ramod_runtime` | Workspace member `runtime/` |
| S1.2 | Backend feature `backend-cpal` | cpal 0.15; null fallback |
| S1.3 | Voices | load_pcm, play, stop, gain, loop |
| S1.4 | Master bus | linear gain |
| S1.5 | `tick` + `render` | pull-based mix; device callback still silent stub |

## Build / test

```bash
# Headless tests (no device)
cargo test -p ramod_runtime --no-default-features

# With cpal (needs ALSA/WASAPI/etc.)
cargo test -p ramod_runtime
```

## API sketch

```rust
use ramod_runtime::{PlayParams, Runtime, RuntimeConfig};

let mut rt = Runtime::new(RuntimeConfig {
    open_device: false, // CI / tests
    ..Default::default()
})?;
let buf = rt.load_pcm(pcm_f32_interleaved, channels, sample_rate)?;
let voice = rt.play(buf, PlayParams { gain: 1.0, looped: false })?;
let mut out = vec![0.0f32; 4800 * rt.channels() as usize];
rt.render(&mut out);
rt.stop(voice);
rt.tick(1.0 / 60.0);
```

## Follow-ups (still S1 / early S2)

1. Wire `Arc<Mutex<Runtime>>` into the cpal callback (true device pull).
2. Example binary `play_sine` that opens the default device.
3. Load PCM from `ramod` FSB5 banks once `src/fsb.rs` is restored.
4. Soft voice steal when pool is full.

## Blockers

- **`src/fsb.rs` is PLACEHOLDER on main** — restore from `git checkout d0e38a1 -- src/fsb.rs` or `artifacts/fsb.rs.RECOVERY` before full workspace `cargo test`.
- Runtime crate itself does **not** depend on `ramod` yet, so it builds in isolation.
