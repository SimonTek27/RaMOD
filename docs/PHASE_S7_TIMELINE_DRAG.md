# S7.20 — Timeline drag to move clips

**Status:** on `main`

## Behavior

- **Click** a clip → select (inspector + AHDSR)
- **Drag** a clip → updates `InstrumentEntry.start_time`
- Clamped to `[0, event_length]`

## Verify

```bash
cargo run -p ramod_editor
# Drag beep clip right → Start value rises → Play (delayed onset)
```
