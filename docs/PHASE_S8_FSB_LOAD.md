# S8 — Native FSB5 load in Bevy

**Status:** on `main`

## Pipeline

```
Editor Build bank
  → build/Master/bank.json   (events + relative assets)
  → build/Master/Master.fsb  (PCM16 samples)
       ↓
LoadBank::new("build/Master/Master.fsb")
  → RamodAudio::load_fsb
  → event:/fsb/<sample_name>
```

`LoadBank` / `load_bank_path` also accept:

| Path | Behaviour |
|------|-----------|
| `*.fsb` | PCM samples → `event:/fsb/<name>` |
| `bank.json` | editor manifest + assets |
| directory | prefers `Master.fsb`, else `bank.json` |

## Requirements

```bash
cargo build -p ramod   # restores full src/fsb.rs via recovery/fsb.z*
```

PCM8 / PCM16 / PCM32 / PcmFloat are decoded. Vorbis-only banks still need
`bank.json` + decoded assets (or a future Vorbis path in `load_fsb`).

## Example

```bash
cargo run -p bevy_ramod --example load_fsb
```
