# S8 — AudioParameter + despawn cleanup

**Status:** on `main`

## Parameters from ECS

```rust
commands.spawn((
    SpatialAudioBundle::new("event:/sfx/engine", pos),
    PlayOnSpawn,
    AudioParameter::new("RPM", 0.5),
));

// or several:
commands.spawn((
    SpatialAudioBundle::new("event:/amb/wind", pos),
    PlayOnSpawn,
    AudioParameters::new([("Intensity", 0.8), ("Distance", 0.2)]),
));
```

Each frame, values are written with `Runtime::set_instance_parameter`.
`AudioEmitter.volume` still maps to **Intensity**.

## Despawn cleanup

`InstanceMap` (resource) stores `Entity → instance id` when `PlayOnSpawn` starts.
On `RemovedComponents<AudioEmitter>`, the runtime instance is `release_instance`'d
so voices do not leak.

## Related

- Bank load: `LoadBank` / `load_bank_dir`
- 3D: `AudioEmitter.is_3d` + `GlobalTransform` / `Velocity`
