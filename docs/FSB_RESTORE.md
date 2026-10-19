# FSB5 foundation restore

## Status

`src/fsb.rs` was briefly replaced by a placeholder during S0.4 error work.
It has been restored from the project recovery snapshot.

## Verify

```bash
cargo test -p ramod --lib
# or
cargo run -p ramod -- list path/to/bank.fsb
```

## Manual restore

```bash
cp recovery/fsb.rs src/fsb.rs
# or from local project artifacts:
# cp artifacts/fsb.rs.RECOVERY src/fsb.rs
```
