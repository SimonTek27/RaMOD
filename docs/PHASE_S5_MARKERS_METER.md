# S5.6 — Transition markers + master meter

**Status:** on `main`

## Markers

Timeline cues on events (`MarkerEntry`: name, time, optional target_event).

- Drawn as yellow lines on the event editor timeline
- **+ Marker** adds a cue at half length
- Runtime: when designed playback `elapsed >= marker.time`, name is pushed to `recent_markers`
- Editor transport shows last fired marker name

## Master meter

`Runtime::master_levels() -> (peak, rms)` from mixer master bus.

Transport bar: Peak/RMS labels + green/red fill (red if peak > 0.9).

## Verify

```bash
cargo run -p ramod_editor
# Play event:/sfx/beep — meter moves; at t≈2s status shows marker: end
```
