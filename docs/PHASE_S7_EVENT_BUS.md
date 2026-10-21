# S7.6 — Event → mixer bus routing

**Status:** on `main`

## Model

`EventEntry.output_bus: String` — default `"SFX"`.

Choices in Event Editor: **SFX** | **Music** | **Master**.

## Runtime

- `Mixer::find_by_name`
- `DesignedEvent.output_bus` / `EventDescription.with_output_bus`
- On `play_event`, voices route to that bus (`current_output_bus`)

## Verify

```bash
cargo run -p ramod_editor
# Select event → Bus: Music → Play → mute SFX in Profiler → still hear Music bus
```
