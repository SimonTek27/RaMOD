# S7.8 — Profiler CPU / voices timeline

**Status:** on `main`

## UI

Profiler panel shows two sparklines (last ≤120 frames):

- **CPU timeline (µs)** — `cpu_render_us`
- **Voices timeline** — `active_voices`

## API

```rust
ProfilerSnapshot.cpu_history: Vec<u32>
ProfilerSnapshot.voice_history: Vec<u32>
```

## Verify

```bash
cargo run -p ramod_editor
# ▶ Play a few seconds — blue CPU + green voices curves update
```
