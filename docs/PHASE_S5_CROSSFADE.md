# S5.6c — Marker crossfade

**Status:** on `main`

## Behaviour

When a transition marker fires:

1. Current designed voices move into `fading_out` (not hard-stopped)
2. Target event starts immediately
3. Each `tick`: gain of departing voices = remaining/duration
4. At remaining ≤ 0 voices are stopped

Default fade: **0.3 s** (`Runtime::set_transition_fade`).

## API

```rust
rt.set_transition_fade(0.5);
```

## Verify

```bash
cargo run -p ramod_editor
# Play event:/sfx/beep — at 1.5s jumps to loop_b with 300ms fade-out
```
