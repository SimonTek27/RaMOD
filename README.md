# RaMOD — Rust Audio mod

[![CI](https://github.com/SimonTek27/RaMOD/actions/workflows/ci.yml/badge.svg)](https://github.com/SimonTek27/RaMOD/actions/workflows/ci.yml)
[![License: GPLv3](https://img.shields.io/badge/license-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

**Open-source FSB5 audio middleware — foundation of an open-source FMOD-class stack for Bevy.**

RaMOD reads and writes **FSB5** sound banks and moves audio between the
FSB5 container and standard formats: parse banks, extract samples as WAV or Ogg
Vorbis, build banks from WAV files, edit banks in place, and encode PCM to
Vorbis — with **zero external crates** (Rust standard library only).

> **Long-term goal:** a complete open-source alternative to FMOD (runtime +
> authoring + Bevy plugin). See the design docs below.

**Design docs**

| Document | Role |
|----------|------|
| [DESIGN.md](DESIGN.md) | Vision, goals, principles |
| [ROADMAP.md](ROADMAP.md) | 50 operational items across 4 phases |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Technical blueprint (runtime, Bevy, editor) |

---

## Features

| | |
|---|---|
| **Parse** | FSB5 header, per-sample headers, metadata chunks (`CHANNELS`, `FREQUENCY`, `LOOP`, `VORBISDATA`, …), name table and payloads |
| **Codecs** | `PCM8/16/24/32`, `PCMFLOAT`, `VORBIS` |
| **Extract** | PCM → RIFF/WAVE; Vorbis → playable **Ogg Vorbis** (headers re-synthesised, packets re-muxed into CRC-correct Ogg pages) |
| **Build** | FSB5 banks from WAV files, with loop points and non-standard sample rates |
| **Edit** | rename, loop points, sample rate, replace audio — preserving everything else byte-for-byte |
| **Import** | any Ogg Vorbis file → FSB5 `VORBIS` sample (setup header matched by CRC32) |
| **Encode** | PCM → Vorbis by driving the stock `libfsbvorbis64.dll` encoder on Windows (pure-Rust encoder is Phase 0) |

## Quick start (library)

```toml
[dependencies]
ramod = { git = "https://github.com/SimonTek27/RaMOD" }
```

```rust
use ramod::Fsb5;

let bank = Fsb5::parse(&std::fs::read("Master.fsb")?)?;
for (i, sample) in bank.samples.iter().enumerate() {
    println!(
        "{i}: {}  ({} Hz, {} ch, {} frames)",
        sample.name, sample.frequency, sample.channels, sample.num_frames
    );
}

let ogg = ramod::build_ogg(&bank, 0)?;
std::fs::write("sample.ogg", ogg)?;
```

Build a bank from WAV data:

```rust
use ramod::{Fsb5Builder, SoundFormat, Wav};

let wav = Wav::parse(&std::fs::read("hit.wav")?)?;
let sample = wav.into_pcm_sample("hit");
let bank = Fsb5Builder::new(SoundFormat::Pcm16).with_sample(sample).build()?;
std::fs::write("hit.fsb", bank)?;
```

## CLI (Phase 0 — work in progress)

```sh
cargo run --bin ramod -- list path/to/bank.fsb
cargo run --bin ramod -- info path/to/bank.fsb
cargo run --bin ramod -- extract path/to/bank.fsb --index 0 --out sample.ogg
```

## Design notes

* **No dependencies.** Only `std` is used. The Vorbis *encoder* currently binds
  `libfsbvorbis64.dll` at runtime on Windows; a pure-Rust encoder is the top
  Phase 0 priority so all platforms work without proprietary binaries.
* **Self-verifying writer.** `Fsb5Builder` and `BankEditor` parse their own
  output before returning it.
* **Faithful editing.** Untouched metadata and payloads are preserved byte-for-byte.

## FSB5 format notes

The container layout was cross-checked against genuine commercial banks:

* **Header** (60 bytes, version ≥ 1): `FSB5` magic, version, sample count,
  sample-header size, name-table size, data size, mode, then the `zero` (8 B),
  `hash` (16 B) and `dummy` (8 B) fields.
* **Sample header**: a packed 64-bit word (`extraParams:1`, `frequency:4`,
  `twoChannels:1`, `dataOffset:28` in **16-byte units**, `samples:30`) followed
  by optional metadata chunks.
* **Alignment**: the data section starts on a **32-byte boundary** and every
  sample is 32-byte aligned.
* **`VORBISDATA`** holds a `u32` setup-header CRC32 plus a seek table.
* **Vorbis sample framing**: `u16` length + payload; zero length ends the stream.
* Setup headers are **not** embedded: a CRC32 keys one of 161 pre-built headers
  in `assets/vorbis_setup_headers.bin`.

## Regenerating the setup-header table

```sh
python tools/build_vorbis_setup_headers.py
```

## Examples

```sh
cargo run --example build_bank
```

## Running the tests

```sh
cargo test
```

## Continuous integration

Every push and pull request runs `cargo build`, `cargo test`, `cargo fmt --check`
and `cargo clippy -D warnings` on Linux, Windows and macOS. Encoder tests skip
themselves when the DLL encoder is unavailable.

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).

RaMOD is an independent, clean-room project and is not affiliated with, or
endorsed by, any vendor of the formats it interoperates with.
