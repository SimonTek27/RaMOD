# Phase S9 — Waveform / GoldWave suite

**Status:** On `main` (2026-10-19)

## Goal

Sample-level editing (GoldWave-style) feeding RaMOD project assets / banks.

## Done

| ID | Item |
|----|------|
| S9.1 | `Selection` + `peak_overview` for waveform UI |
| S9.2 | Cut / copy / paste / delete / silence / reverse |
| S9.3 | Gain, normalize, fade in/out + undo stack |
| S9.4 | `export_wav` (16-bit PCM) |

## Crate

```bash
cargo test -p ramod_waveform
```

```rust
use ramod_waveform::WaveBuffer;

let mut w = WaveBuffer::new(pcm, 1, 48_000)?;
w.set_selection(0, 4800);
w.apply_fade_in();
w.apply_normalize(0.95);
w.export_wav(Path::new("out.wav"), false)?;
```

## Follow-ups

- egui waveform panel in `ramod_editor`
- Load WAV into `WaveBuffer`
- Push exported assets into `project.ramod.json` Assets list
- Optional: EQ / compressor effects (reuse runtime DSP)
