# S7.22 full restore

**Mixer send/return** is on `main`.

Truncated `runtime_core_b.rs` / `editor/src/audio_bridge.rs` are restored **automatically** on `cargo build` from:

- `recovery/core_b.z0` + `recovery/core_b.z1`
- `recovery/ab.z0` + `recovery/ab.z1`

```bash
git pull
cargo build -p ramod_runtime -p ramod_editor
# cargo:warning=restored runtime/src/runtime_core_b.rs (...)
# cargo:warning=restored editor/src/audio_bridge.rs (...)
```

Profiler: bus rows show **`%→Rev`** (SFX/Music) for post-fader send to Reverb return.
