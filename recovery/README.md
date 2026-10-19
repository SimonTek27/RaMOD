# Recovery snapshots

If `src/fsb.rs` is ever overwritten, restore with:

```bash
cp recovery/fsb.rs src/fsb.rs
```

The copy here matches the known-good FSB5 parser (parse, sample_data, sample_to_wav, vorbis packets).
