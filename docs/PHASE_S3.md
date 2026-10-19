# Phase S3 — Editor vertical slice

**Status:** On `main` (2026-10-19)

## Goal

Author an event in the egui editor, press Play, hear it via `ramod_runtime`.

## Done

| ID | Item |
|----|------|
| S3.1 | Project model JSON (`project.ramod.json`) — events / banks / assets |
| S3.2 | Events browser + add event; Overview track/instrument |
| S3.3 | Transport ▶ → `AudioBridge` → `ramod_runtime` |
| S3.4 | Deck panel parameter stub + fake curve |
| S3.5 | Live Update Off/On badge (protocol stub) |

## Run

```bash
git pull
cargo run -p ramod_editor --no-default-features
# or with default features of workspace members:
cargo run -p ramod_editor
```

**Note:** Play mixes in software (`open_device: false`). You will not hear speakers until the cpal callback is wired; the transport still advances and status shows Playing.

## File → Save

Writes `project.ramod.json` in the working directory.

## Follow-ups

1. Wire cpal so Play is audible on the default device
2. Load real WAV assets into PCM
3. FSB5 bank export/import after `src/fsb.rs` restore
4. Timeline multi-track editing (S5)
