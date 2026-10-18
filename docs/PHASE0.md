# Phase 0 — Progress

Tracking foundation work toward a production-ready pure-Rust FSB5 middleware.

## Done

- [x] Official design docs (`DESIGN.md`, `ROADMAP.md`, `ARCHITECTURE.md`)
- [x] CLI binary `ramod`
  - [x] `list` / `info` / `extract` / `build`
- [x] **Pure-Rust Vorbis encoder path** (`encoder-rust` feature, default)
  - Uses [`rusty_vorbis`](https://crates.io/crates/rusty_vorbis) 0.1 (Apache-2.0, zero deps, no C/FFI)
  - Setup CRC32 must match the bundled 161-header table
  - Windows DLL path retained behind `encoder-dll`

## Features

```toml
[features]
default = ["encoder-rust"]
encoder-rust = ["dep:rusty_vorbis"]  # pure Rust, all platforms
encoder-dll  = []                    # libfsbvorbis64.dll, Windows only
```

```bash
# default: pure-Rust encoder
cargo test

# DLL only (Windows legacy)
cargo test --no-default-features --features encoder-dll

# both (Rust first, DLL fallback on Windows)
cargo test --features encoder-rust,encoder-dll
```

## Known caveat — setup header table

FSB5 stores a **CRC32 of the Vorbis setup packet**, not the packet itself.
RaMOD ships 161 pre-built headers taken from commercial / reference banks.

`rusty_vorbis` embeds a libvorbis-compatible setup profile. If its setup CRC
is not in our table, encode returns `UnknownSetupHeader` and tests skip.

**Follow-up:**
1. Capture the setup packet from a successful `rusty_vorbis` encode and add it
   to `assets/vorbis_setup_headers.bin` via `tools/build_vorbis_setup_headers.py`.
2. Or constrain quality/blocksize so the CRC matches an existing entry.

## Still open

| # | Item | Notes |
|---|------|-------|
| 2 | Pure-Rust Vorbis **decoder** | `lewton` is the natural optional dependency |
| 3 | Richer error types | |
| 4 | Expanded fixtures | |
| 5 | cargo-fuzz | |
| 7 | crates.io | After setup-CRC interop verified |
| 9 | PCM24 / PCMFLOAT write | |

## CLI

```bash
cargo run --bin ramod -- list path/to/bank.fsb
cargo run --bin ramod -- extract bank.fsb --index 0 --out sample.wav
cargo run --bin ramod -- build --out out.fsb a.wav b.wav
```

See [ROADMAP.md](../ROADMAP.md) Phase 0 for the full checklist.
