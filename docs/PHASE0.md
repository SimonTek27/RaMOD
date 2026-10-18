# Phase 0 — Progress

Tracking foundation work toward a production-ready pure-Rust FSB5 middleware.

## Done

- [x] Official design docs (`DESIGN.md`, `ROADMAP.md`, `ARCHITECTURE.md`)
- [x] CLI binary `ramod`
  - [x] `list` — list samples
  - [x] `info` — bank / sample details
  - [x] `extract` — PCM → WAV (`sample_to_wav`), Vorbis → Ogg (`build_ogg`)
  - [x] `build` — WAV files → FSB5 PCM16 bank
- [x] Library already exposes `Fsb5::sample_to_wav` (used by CLI)

## In progress / next

| # | Item | Notes |
|---|------|-------|
| 1 | Pure-Rust Vorbis **encoder** | Still Windows DLL (`libfsbvorbis64.dll`). Priority P0. |
| 2 | Pure-Rust Vorbis **decoder** | Needed for runtime playback without external crates. |
| 3 | Richer error types | Context on all parse/write paths. |
| 4 | Expanded fixtures / round-trip tests | Synthetic + optional commercial banks. |
| 5 | cargo-fuzz | Parser + Ogg muxer. |
| 6 | CLI polish | `edit`, `convert`, better exit codes/messages — mostly done for MVP. |
| 7 | crates.io publish | After encoder path is clear / feature-gated. |
| 8 | mdBook / docs.rs | |
| 9 | PCM24 / PCMFLOAT write | Builder currently strongest on PCM16. |

## How to try the CLI

```bash
cargo run --bin ramod -- list path/to/bank.fsb
cargo run --bin ramod -- info path/to/bank.fsb --index 0
cargo run --bin ramod -- extract path/to/bank.fsb --index 0 --out sample.ogg   # Vorbis
cargo run --bin ramod -- extract path/to/bank.fsb --index 0 --out sample.wav   # PCM
cargo run --bin ramod -- build --out out.fsb a.wav b.wav
```

## Encoder strategy (item 1)

1. Keep current DLL path behind feature `encoder-dll` (Windows).
2. Add pure-Rust or optional-crate encoder path (must produce setup CRC32 in the 161-header table).
3. Default builds must not require proprietary binaries.

See [ROADMAP.md](../ROADMAP.md) Phase 0 for the full checklist.
