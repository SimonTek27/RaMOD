# S7.21 — Timeline clip resize (right edge)

**Status:** on `main`

## Behavior

| Gesture | Effect |
|---------|--------|
| Click clip | Select |
| Drag clip **body** | Move `start_time` |
| Drag **right edge** (~8px) | Change `length` |

Selected clip shows a light resize handle on the right.

Clamps: `length ≥ 0.05s`, does not extend past event length.

## Verify

```bash
cargo run -p ramod_editor
# Select beep → drag right edge → Len rises → Play
```
