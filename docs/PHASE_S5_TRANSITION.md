# S5.6b — Marker → target_event jump

**Status:** on `main`

## Behaviour

When a designed playback reaches a marker with `target_event` set:

1. Marker is recorded in `recent_markers`
2. Runtime queues `pending_transition = (instance_id, target_path)`
3. End of `tick`: stop/release current instance, `play_event(target_path)`

Each marker fires at most once per playback (no infinite self-loops in one session unless the target event re-introduces the marker).

## Editor

Marker row: **name | t | → event:/path | x**

Default project: `event:/sfx/beep` marker `to_b` @ 1.5s → `event:/sfx/loop_b`.

## Verify

```bash
cargo run -p ramod_editor
# Play event:/sfx/beep — after ~1.5s jumps to loop_b
```
