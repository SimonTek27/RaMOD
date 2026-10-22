# S7.22 full restore

## Already on `main`

- **Mixer** send/return (`BusSend`, Reverb return, post-fader `finish`)
- **`BusMeterInfo.send_reverb` / `is_return`**
- **`build.rs`** auto-restores `runtime_core_b.rs` + `audio_bridge.rs` when truncated (from `recovery/*.part*` or `*.z*`)

## Restore sources (local project artifacts)

In the linked project folder:

```text
artifacts/recovery/core_b.rs          # full runtime_core_b
artifacts/recovery/ab.rs              # full audio_bridge
artifacts/recovery/core_b.part0..3    # same, split for offline concat
artifacts/recovery/ab.part0..4
```

### Manual copy into the clone

```bash
cd /path/to/RaMOD
cp /path/to/artifacts/recovery/core_b.rs runtime/src/runtime_core_b.rs
cp /path/to/artifacts/recovery/ab.rs     editor/src/audio_bridge.rs
# optional: also copy parts into recovery/ for build.rs
cp /path/to/artifacts/recovery/core_b.part* recovery/
cp /path/to/artifacts/recovery/ab.part* recovery/
cargo build -p ramod_runtime -p ramod_editor
```

### Auto path

After `recovery/core_b.part*` and `recovery/ab.part*` are in the repo:

```bash
cargo build
# cargo:warning=restored runtime/src/runtime_core_b.rs (...)
# cargo:warning=restored editor/src/audio_bridge.rs (...)
```

## Profiler UI

Bus rows (SFX/Music): **`%→Rev`** 0–100 → `set_bus_send_reverb`.
