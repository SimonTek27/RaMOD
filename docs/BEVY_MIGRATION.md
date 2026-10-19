# Migration guide: bevy_fmod / bevy_kira_audio → bevy_ramod

## Why

`bevy_ramod` targets the same ECS patterns as **bevy_fmod** but uses the **open-source** `ramod_runtime` (no FMOD Studio license, no closed DLLs on the critical path).

## bevy_fmod → bevy_ramod

| bevy_fmod | bevy_ramod |
|-----------|------------|
| `FmodPlugin` | `RamodPlugin` |
| `FmodStudio` resource | `RamodAudio` |
| `AudioSource` | `AudioEmitter` |
| `AudioListener` | `AudioListener` |
| `Velocity` | `Velocity` |
| Studio banks (`.bank`) | `*.ramodbank.json` (+ future FSB5) |
| `studio.update()` in PostUpdate | `audio_tick_render` |

### Example

```rust
// before (bevy_fmod)
app.add_plugins(FmodPlugin { audio_path: "assets/audio" });

// after
app.add_plugins(RamodPlugin::default());
```

Event paths stay Studio-like: `event:/sfx/explosion`.

Parameters:

```rust
audio.runtime.set_instance_parameter(id, "Intensity", 0.8);
```

## bevy_kira_audio → bevy_ramod

| kira | ramod |
|------|-------|
| `Audio` / `AudioSource` | `RamodAudio` + `AudioEmitter` |
| Clip handles | Event paths + PCM buffers |
| Spatial emitter | `SpatialAudioBundle` |

Kira is excellent for music stacks; RaMOD aims at **middleware** (events, buses, snapshots, live update). You can use both: kira for music, ramod for adaptive SFX — or migrate SFX first.

## Gaps vs FMOD (honest)

- No binary `.bank` FMOD format (by design)
- Sample data still needs PCM / future FSB5 restore
- Programmer instruments / AHDSR full authoring still maturing (S5+)

## Live Update

Enable `ramod_runtime` feature `live-update` and poll `LiveUpdateServer` from a Bevy system so the editor can drive parameters while the game runs.
