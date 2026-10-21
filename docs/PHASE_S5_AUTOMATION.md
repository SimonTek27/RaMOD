# S5.3 — Automation + stop scheduled voices

**Status:** on `main`

## Stop

`stop_instance` / `release_instance`:
- stops primary voice
- stops all multi-track `DesignedPlayback` voices
- drops `ScheduledVoice` rows for that instance

`clear_scheduled()` empties the delay queue (editor `AudioBridge::stop` calls it).

## Automation

`AutomationCurve` (piecewise linear) on `DesignedEvent`:
- evaluated each `tick` against playback `elapsed`
- writes instance parameters (e.g. `Intensity`)
- spatial/gain sync already multiplies Intensity into voice gain

Editor: **+ Intensity fade** on the event panel (0s value → end value over event length).

## Verify

```bash
cargo run -p ramod_editor
# Play event with default Intensity fade 1.0 → 0.2 over 2s
# Stop (■) must silence immediately, including delayed clips
```
