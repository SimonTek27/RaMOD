# S8 — bevy_ramod bank.json loader

**Status:** on `main`

## API

```rust
// From editor Build output
audio.load_bank_dir(Path::new("build/Master/bank.json"))?;

// Or from parsed asset
let bank = RamodBankAsset::from_json_path(path)?;
audio.register_bank_asset(&bank)?;
```

Events get PCM from `asset_file` (relative to `bank.json`), decoded via
`ramod_waveform` (wav / mp3 / ogg / flac). If `asset_file` is missing, a short
demo tone is registered so the event path still exists.

## Pipeline

```
Editor: import → assign → Build bank
        ↓
build/Master/bank.json + assets/*
        ↓
bevy_ramod: load_bank_dir → register_event → PlayOnSpawn
```

## Example

```bash
cargo run -p bevy_ramod --example load_bank
```

## AssetServer note

`RamodBankLoader` registers the `.ramodbank.json` extension for Bevy’s asset
pipeline. Day-to-day games should use `load_bank_dir` on the filesystem path
produced by the editor (relative asset files are resolved correctly).
