# S8 — Hybrid bank.json + Master.fsb

**Status:** on `main`

## Goal

Keep **Studio event paths** from the editor (`event:/sfx/footstep`) while
streaming sample data from the **binary FSB** when present.

## Flow

```
build/Master/
  bank.json      ← event paths, 3D, params, asset names
  Master.fsb     ← PCM/Vorbis payloads
  assets/…       ← optional fallback files

LoadBank::new("build/Master")  or  "…/bank.json"
  → parse bank.json
  → index Master.fsb samples by name
  → for each event, match:
       asset name → file stem → event tail
  → register EventDescription with Studio path + FSB PCM
```

## Matching keys

1. `BankEventAsset.asset` (editor asset name)
2. `asset_file` stem / file name
3. last segment of `path` (`event:/sfx/beep` → `beep`)

If no FSB match, falls back to `assets/` file or demo tone.

## API

```rust
audio.load_bank_dir(Path::new("build/Master/bank.json"));
// or
commands.spawn(LoadBank::new("build/Master"));
```

Log line example:

```text
hybrid: indexed 3 samples from build/Master/Master.fsb
RamodAudio: registered 3/3 events from bank 'Master' (3 from FSB)
```
