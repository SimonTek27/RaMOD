# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Renzora integration** (`bevy_ramod` feature `renzora`, Bevy **0.19**):
  `replace_renzora_audio_with_ramod`, `RamodRenzoraBackend`, shared bus keys,
  zero-glue Backend reference in `docs/renzora_backend_glue.rs`, guide
  [`docs/RENZORA.md`](docs/RENZORA.md).
- **Studio stack (S1–S9)** — `ramod_runtime` (mixer, voices, events, 3D, cpal ring),
  `ramod_editor` (egui Event Editor), `bevy_ramod`, `ramod_waveform` (GoldWave-style).
- **Bank pipeline** — editor assign asset→event, `Build bank` → `bank.json` + assets +
  optional **`Master.fsb`** (PCM16 via `Fsb5Builder`).
- **bevy_ramod** — `LoadBank`, `load_bank_dir`, spatial emitters, volume → Intensity.
- **Waveform** — import WAV/MP3/OGG/FLAC (Symphonia), DnD, selection, preview.
- **FSB offline restore** — `recovery/fsb.z{0,1,2}` + `build.rs` (base64/zlib) before network.
- Pure-Rust encoder/decoder feature path (`rusty_vorbis` / `lewton`).

### Fixed

- `src/fsb.rs` placeholder recovery path (do not ship stub without `cargo build`).

## [0.1.0] - 2026-10-15

### Added

- **`fsb`** — FSB5 container parser: header, per-sample headers, metadata chunks
  (`CHANNELS`, `FREQUENCY`, `LOOP`, `XMASEEK`, `DSPCOEFF`, `XWMADATA`,
  `VORBISDATA`), name table and sample payloads.
- **`fsb_write`** — FSB5 writer ([`Fsb5Builder`]), in-place [`BankEditor`] and the
  RIFF/WAVE reader ([`Wav`]).
- **`vorbis`** — Vorbis ↔ Ogg: the bundled 161-entry setup-header table, the Ogg
  muxer/demuxer, FSB → Ogg export (`build_ogg`) and Ogg → FSB import
  (`import_ogg`, `import_ogg_to_bank`).
- **`vorbis_encode`** — PCM → Vorbis encoding path.
- Spec-faithful layout: 32-byte-aligned sample data, name-table padding so the
  data section starts 32-byte aligned, and 32-byte padding of the data section.
