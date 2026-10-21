# S7.6 — Event → mixer bus routing

**Status:** on `main` (model + UI + API; per-voice accumulate still shared buffer)

## Editor

Event Editor → **Bus** combo: `SFX` | `Music` | `Master`.

Stored as `EventEntry.output_bus` (default `SFX`).

## Runtime

- `DesignedEvent.output_bus` / `EventDescription.output_bus`
- `Mixer::find_by_name`
- `Runtime.current_output_bus` set in `play_event` from the event's bus name
- `play_with_priority` routes the voice to that bus

> Note: `render` still mixes voices into one dry buffer before bus accumulate; true isolation of SFX vs Music meters improves when per-voice `accumulate_voice` is wired. Mute/solo/volume on Master still affect the sum.

## Verify

```bash
cargo run -p ramod_editor
# Set event Bus to Music → Play → Profiler Music row reacts when per-voice path is complete
```
