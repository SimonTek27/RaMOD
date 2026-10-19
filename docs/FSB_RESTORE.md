# Restore full FSB5 parser

## One command

```bash
git show d0e38a10e0ad438bbea79095292b3bbd0218c953:src/fsb.rs > src/fsb.rs
```

Or from this project's local recovery file (if you have the workspace artifacts):

```bash
cp artifacts/fsb.rs.RECOVERY src/fsb.rs
```

## Why

Automated pushes briefly replaced `src/fsb.rs` with a placeholder. The full
implementation (~35 KB: parse, sample_data, sample_to_wav, vorbis packets,
tests) is still in git history at **d0e38a10**.

## Verify

```bash
cargo test -p ramod --lib
cargo run -p ramod -- list some.fsb
```
