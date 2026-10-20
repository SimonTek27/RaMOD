# S8 — bevy_ramod bank loader

**Status:** on `main`

## Ways to load a bank

### 1. Component (preferred in ECS)

```rust
commands.spawn(LoadBank::new("build/Master/bank.json"));
```

Processed once on `Added<LoadBank>`, then the component is removed.

### 2. Imperative API

```rust
audio.load_bank_dir(Path::new("build/Master/bank.json"))?;
```

### 3. Parsed asset

```rust
let bank = RamodBankAsset::from_json_path(path)?;
audio.register_bank_asset(&bank)?;
```

## Emitter lifecycle

| Component / field | Behaviour |
|-------------------|-----------|
| `PlayOnSpawn` | Starts event once, then removed |
| `AudioEmitter.volume` | Written each frame as `Intensity` |
| `AudioEmitter.is_3d` | Position/velocity synced from `GlobalTransform` |

## Pipeline

```
Editor Build → build/Master/bank.json
       ↓
LoadBank / load_bank_dir
       ↓
register_event (PCM via ramod_waveform)
       ↓
SpatialAudioBundle + PlayOnSpawn
```

## Example

```bash
cargo run -p bevy_ramod --example load_bank
```
