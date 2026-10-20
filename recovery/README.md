# Recovery

## FSB5 parser

`cargo build -p ramod` auto-restores `src/fsb.rs` via `build.rs` from:

- this folder’s `fsb.rs` if you place a full copy here, or
- commit `d0e38a10` on GitHub (network).

Manual:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/SimonTek27/RaMOD/d0e38a10e0ad438bbea79095292b3bbd0218c953/src/fsb.rs \
  -o src/fsb.rs
```
