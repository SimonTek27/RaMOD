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

Also works with **game/mod Vorbis FSB5** banks (FSB mode = Vorbis):
`build_ogg` + `lewton` → interleaved f32 PCM.

`LoadBank` / `load_bank_path` accept:

| Path | Behaviour |
|------|-----------|
| `*.fsb` | samples → `event:/fsb/<name>` (PCM or Vorbis) |
| `bank.json` | editor manifest + assets |
| directory | prefers `Master.fsb`, else `bank.json` |

## Formats

| Format | Path |
|--------|------|
| PCM8 / PCM16 / PCM32 / Float | direct sample payload |
| **Vorbis** | `decode_sample_to_pcm` (pure Rust) |
| other (XMA, AT9, …) | skipped with warning |

## Requirements

```bash
cargo build -p ramod   # restores full src/fsb.rs via recovery/fsb.z*
```

## Example

```bash
cargo run -p bevy_ramod --example load_fsb
```
