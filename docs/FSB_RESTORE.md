# FSB5 parser restore

## Automatic (preferred)

```bash
cargo build -p ramod
```

`build.rs` detects the stub and restores the full parser from:

1. `recovery/fsb.rs` (if present)
2. GitHub raw URL at commit `d0e38a10`

Then rewrites `src/fsb.rs`.

## Manual

```bash
git show d0e38a10e0ad438bbea79095292b3bbd0218c953:src/fsb.rs > src/fsb.rs
# or
curl -fsSL \
  https://raw.githubusercontent.com/SimonTek27/RaMOD/d0e38a10e0ad438bbea79095292b3bbd0218c953/src/fsb.rs \
  -o src/fsb.rs
```

## Verify

```bash
cargo test -p ramod --lib
cargo run -p ramod -- list path/to/bank.fsb
```
