# Recovery

## FSB5 parser (offline)

`cargo build -p ramod` restores `src/fsb.rs` via `build.rs`, in order:

1. **`fsb.z0` + `fsb.z1` + `fsb.z2`** — zlib + base64 of the full parser (no network)
2. **`fsb.rs`** — plaintext full source if present
3. **Network** — known-good commit `d0e38a10`

Manual:

```bash
cargo build -p ramod
# or
curl -fsSL \
  https://raw.githubusercontent.com/SimonTek27/RaMOD/d0e38a10e0ad438bbea79095292b3bbd0218c953/src/fsb.rs \
  -o src/fsb.rs
```

Verify:

```bash
cargo run -p ramod -- list path/to/bank.fsb
```
