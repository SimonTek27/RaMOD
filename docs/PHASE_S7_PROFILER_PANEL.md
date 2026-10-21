# S7 — Editor Profiler panel

**Status:** on `main`

## UI

**Window → Profiler** (right dock, default on).

| KPI | Source |
|-----|--------|
| CPU µs | last `render` |
| Avg CPU | session average |
| Voices / peak | active + session peak |
| Instances | EventSystem count |
| Peak / RMS | master bus meter + bar |
| Playing events | paths from frame |
| Spectrum | 16 smoothed bands |

**Clear** resets the profiler session.

## API

```rust
audio.profiler_snapshot() -> ProfilerSnapshot
audio.profiler_clear()
```

## Verify

```bash
cargo run -p ramod_editor
# Play an event — right panel CPU/voices/spectrum update
```
