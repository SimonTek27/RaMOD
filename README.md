# RaMOD — Rust Audio mod

[![CI](https://github.com/SimonTek27/RaMOD/actions/workflows/ci.yml/badge.svg)](https://github.com/SimonTek27/RaMOD/actions/workflows/ci.yml)
[![License: GPLv3](https://img.shields.io/badge/license-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

**An open-source FSB5 audio middleware, written in Rust.**

RaMOD reads and writes **FSB5** sound banks and moves audio between the
FSB5 container and standard formats: parse banks, extract samples as WAV or Ogg
Vorbis, build banks from WAV files, edit banks in place, and encode PCM to
Vorbis — with **zero external crates** (Rust standard library only).


## Features

| | |
|---|---|
| **Parse** | FSB5 header, per-sample headers, metadata chunks (`CHANNELS`, `FREQUENCY`, `LOOP`, `VORBISDATA`, …), name table and payloads |
| **Codecs** | `PCM8/16/24/32`, `PCMFLOAT`, `VORBIS` |
| **Extract** | PCM → RIFF/WAVE; Vorbis → playable **Ogg Vorbis** (headers re-synthesised, packets re-muxed into CRC-correct Ogg pages) |
| **Build** | FSB5 banks from WAV files, with loop points and non-standard sample rates |
| **Edit** | rename, loop points, sample rate, replace audio — preserving everything else byte-for-byte |
| **Import** | any Ogg Vorbis file → FSB5 `VORBIS` sample (setup header matched by CRC32) |
| **Encode** | PCM → Vorbis by driving the stock `libfsbvorbis64.dll` encoder (no Vorbis crate) |

## Quick start

```toml
[dependencies]
ramod = { git = "https://github.com/SimonTek27/RaMOD" }
```

```rust
use ramod::Fsb5;

// Parse a bank straight from a file.
let bank = Fsb5::parse(&std::fs::read("Master.fsb")?)?;
for (i, sample) in bank.samples.iter().enumerate() {
    println!(
        "{i}: {}  ({} Hz, {} ch, {} frames)",
        sample.name, sample.frequency, sample.channels, sample.num_frames
    );
}

// Rebuild a Vorbis sample as a playable Ogg file.
let ogg = ramod::build_ogg(&bank, 0)?;
std::fs::write("sample.ogg", ogg)?;
```

Build a bank from WAV data and re-serialise it:

```rust
use ramod::{Fsb5Builder, PcmSample, SoundFormat, Wav};

let wav = Wav::parse(&std::fs::read("hit.wav")?)?;
let sample = wav.into_pcm_sample("hit");
let bank = Fsb5Builder::new(SoundFormat::Pcm16).with_sample(sample).build()?;
std::fs::write("hit.fsb", bank)?;
```

Encode PCM to Vorbis (Windows; needs the `libfsbvorbis64.dll` encoder):

```toml
# ramod::encode_to_bank(&pcm_f32, channels, sample_rate, quality, name)
```

## Design notes

* **No dependencies.** Only `std` is used. The Vorbis *encoder* binds the stock
  `libfsbvorbis64.dll` at runtime through `LoadLibraryW`/`GetProcAddress`;
  every other path is portable.
* **Self-verifying writer.** [`Fsb5Builder`] and [`BankEditor`] parse their own
  output before returning it, so a successful build is guaranteed to be
  readable.
* **Faithful editing.** Metadata chunks, header fields and codec payloads the
  editor does not touch are preserved byte-for-byte.

## FSB5 format notes

The container layout was cross-checked against genuine commercial banks:

* **Header** (60 bytes, version ≥ 1): `FSB5` magic, version, sample count,
  sample-header size, name-table size, data size, mode, then the `zero` (8 B),
  `hash` (16 B) and `dummy` (8 B) fields.
* **Sample header**: a packed 64-bit word (`extraParams:1`, `frequency:4`,
  `twoChannels:1`, `dataOffset:28` in **16-byte units**, `samples:30`) followed
  by optional metadata chunks.
* **Alignment**: the data section starts on a **32-byte boundary**
  (`60 + sampleHeaderSize + nameTableSize` is a multiple of 32) and every sample
  is 32-byte aligned; the section is padded to 32.
* **`VORBISDATA`** holds a `u32` setup-header CRC32 plus a `(offset, granule)`
  seek table (the reference table is sparse; RaMOD writes one entry per packet —
  both valid).
* **Vorbis sample framing**: each packet is a `u16` length followed by exactly
  that many raw Vorbis bytes; a zero length terminates the stream. There is **no
  flags byte**.
* The format does not embed the Vorbis setup header: it stores a **CRC32** that
  keys one of 161 pre-built setup headers, bundled here in
  `assets/vorbis_setup_headers.bin`.

## Regenerating the setup-header table

```sh
python tools/build_vorbis_setup_headers.py
```

Reads `tools/reference_vorbis_headers.py` and writes
`assets/vorbis_setup_headers.bin`.

## Examples

```sh
cargo run --example build_bank
```

Builds a bank from a generated sine wave and reads it back.

## Running the tests

```sh
cargo test
```

## Continuous integration

Every push and pull request runs `cargo build`, `cargo test`, `cargo fmt
--check` and `cargo clippy -D warnings` on Linux, Windows and macOS (see
[`.github/workflows/ci.yml`](.github/workflows/ci.yml)). The encoder tests skip
themselves when the `libfsbvorbis64.dll` encoder is unavailable.

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).

RaMOD is an independent, clean-room project and is not affiliated with, or
endorsed by, any vendor of the formats it interoperates with.
