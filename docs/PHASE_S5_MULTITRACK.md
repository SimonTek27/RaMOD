# S5 — Multi-track Event Editor

**Status:** on `main` (authoring UI)

## Model

```text
EventEntry
  tracks[]: TrackEntry
    name, mute, solo
    instruments[]: InstrumentEntry
      name, asset_index, start_time, length, probability, looping
  length (timeline seconds)
```

Legacy single-asset events auto-gain one track when opened.

## UI

Central panel of `ramod_editor`:

- Timeline ruler (seconds)
- Track rows with clip blocks
- Mute / Solo
- **+ Track** / **+ Instrument**
- Instrument inspector (start, length, asset index)

## Bank export

`Build bank` writes `tracks` + `length` into each `BankBuildEvent` in `bank.json`.
Runtime still plays the primary asset path for now; multi-instrument mix-down is a follow-up (S5.runtime).

## Run

```bash
cargo run -p ramod_editor
# select event → edit tracks in the center panel
```
