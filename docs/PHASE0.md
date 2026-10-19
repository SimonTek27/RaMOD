# Phase 0 — Progress (S0 Foundation lock)

Tracking foundation work toward a production-ready pure-Rust FSB5 middleware.

## Done

- [x] Official design docs + Studio plan + UI reference
- [x] CLI binary `ramod` — `list` / `info` / `extract` / `build` / **`edit`**
- [x] **S0.1 Pure-Rust encoder path** (`encoder-rust`, default)
  - [`rusty_vorbis`](https://crates.io/crates/rusty_vorbis) 0.1
  - Setup profile CRC **`0xc2ea917f`** is already present in the 161-header table
  - Fix: drain via `VorbisEncoder::next_packet` (not `next`)
- [x] **S0.2 Pure-Rust decoder path** (`decoder-rust`, default)
  - [`lewton`](https://crates.io/crates/lewton) via `decode_sample_to_pcm` / `decode_ogg_to_pcm`
  - FSB5 → `build_ogg` → PCM `f32`
- [x] **S0.3 CLI `edit`**
  - rename, loop points, clear-loop, frequency via [`BankEditor`](../src/fsb_write.rs)
  - optional `--out` (default: overwrite input); re-parse sanity check after rebuild

## Features

```toml
[features]
default = ["encoder-rust", "decoder-rust"]
encoder-rust = ["dep:rusty_vorbis"]
decoder-rust = ["dep:lewton"]
encoder-dll  = []   # Windows libfsbvorbis64.dll fallback
```

```bash
cargo test
cargo test --no-default-features --features encoder-dll   # Windows legacy only
```

**Requires rustc ≥ 1.85** (rusty_vorbis MSRV).

## Still open (S0)

| ID | Item | Notes |
|----|------|-------|
| S0.3b | CLI `convert` | optional: format / batch (not blocking S1) |
| S0.4 | Richer errors + more fixtures + fuzz | |
| S0.5 | crates.io publish 0.2 | After CI green on 1.85+ |

## Encode note

`rusty_vorbis` embeds a fixed libvorbis-compatible setup (`setup_q4_stereo.bin`).
Its zlib CRC32 is **0xc2ea917f**, which is one of the 161 bundled headers —
so FSB5 banks produced this way remain playable by reference decoders.

## CLI

```bash
cargo run --bin ramod -- list path/to/bank.fsb
cargo run --bin ramod -- extract bank.fsb --index 0 --out sample.ogg
cargo run --bin ramod -- build --out out.fsb a.wav b.wav

# Metadata edit (payload unchanged)
cargo run --bin ramod -- edit bank.fsb --index 0 --name explosion
cargo run --bin ramod -- edit bank.fsb --index 0 --loop 0:12000 --out out.fsb
cargo run --bin ramod -- edit bank.fsb --index 0 --clear-loop
cargo run --bin ramod -- edit bank.fsb --index 0 --frequency 48000
```

See [STUDIO_DEV_PLAN.md](STUDIO_DEV_PLAN.md) Phase S0.
