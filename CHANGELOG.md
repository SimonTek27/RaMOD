# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- **`vorbis_encode`** — PCM → Vorbis encoding through the stock
  `libfsbvorbis64.dll` via raw FFI (`LoadLibraryW`/`GetProcAddress`), with a
  graceful `EncoderUnavailable` fallback off Windows or when the DLL is absent.
- Spec-faithful layout: 32-byte-aligned sample data, name-table padding so the
  data section starts 32-byte aligned, and 32-byte padding of the data section.
- Zero external crates: the crate depends only on the Rust standard library.

[Unreleased]: https://github.com/SimonTek27/RaMOD/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/SimonTek27/RaMOD/releases/tag/v0.1.0
