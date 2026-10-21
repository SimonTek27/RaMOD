# S7.2 — Profiler NDJSON export

**Status:** on `main`

## UI

Profiler panel → **Export NDJSON** writes `profiler_session.ndjson` (cwd).

One JSON object per line (`ProfilerFrame`): index, time, cpu_render_us, active_voices, master_peak/rms, playing_events, …

## API

```rust
audio.profiler_ndjson() -> String  // Runtime::profiler_session().to_ndjson()
```

## Verify

```bash
cargo run -p ramod_editor
# Play a few seconds → Export NDJSON → open profiler_session.ndjson
```
