# FSB5 restore + binary bank export

## Restore parser

```bash
cargo build -p ramod
```

`build.rs` restores `src/fsb.rs` from:

1. `recovery/fsb.z0` + `fsb.z1` + `fsb.z2` (zlib+base64, offline)
2. `recovery/fsb.rs`
3. GitHub commit `d0e38a10` (network)

## Binary export (editor)

**File → Build bank** now writes:

```
build/Master/
  bank.json          # Bevy / runtime manifest
  assets/…           # copied sources
  Master.fsb         # FSB5 PCM16 binary (when samples available)
```

`Master.fsb` is built with `ramod::Fsb5Builder` (PCM16). Events without a
loadable asset file are skipped for the binary (still listed in bank.json).

## Verify

```bash
cargo build -p ramod
# should print: restored src/fsb.rs (~28k bytes)

cargo run -p ramod -- list build/Master/Master.fsb
```
