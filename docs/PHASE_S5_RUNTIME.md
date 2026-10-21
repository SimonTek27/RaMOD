# S5.runtime — Multi-instrument mix-down

**Status:** on `main`

## Behaviour

When an event has timeline **tracks** with instruments:

1. Editor `AudioBridge::sync_project` builds a `DesignedEvent` + one PCM buffer per instrument.
2. `Runtime::register_designed_event` stores path → buffers.
3. `play_event` / `play_designed_registered`:
   - respects **mute / solo**
   - probability
   - Multi / Scatter kinds
   - **`start_time`**: instruments with `start_time > 0` are queued in `scheduled` and fired from `tick`
4. Primary `EventInstance` is created for Live Update parameters without double-starting the primary buffer.

## Legacy

Events with empty `tracks` still use single-buffer `EventDescription`.

## Verify

```bash
cargo run -p ramod_editor
# + Track, + Instrument with Start=0.5, Play → second clip after 0.5s
```
