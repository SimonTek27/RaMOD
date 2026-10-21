# S7.7 — Per-voice bus accumulate

**Status:** on `main`

## Change

`VoicePool::mix_each` mixes every live voice into a temp buffer and calls `on_voice(id, samples)`.

`Runtime::render` now:

```text
mixer.prepare
for each voice → mixer.accumulate_voice(voice_id, slice)  // uses route table
mixer.finish → Master
```

`play_event` sets `current_output_bus` from `DesignedEvent.output_bus` / `EventDescription.output_bus` so new voices route to SFX, Music, or Master.

## Verify

```bash
cargo run -p ramod_editor
# Event Bus = Music → Play → Profiler Music peak moves; mute SFX → still hear
# Event Bus = SFX → mute SFX → silence
```
