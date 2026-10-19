# Phase S5 — Full Event Designer

**Status:** Foundation on `main` (2026-10-19)

## Goal

Competitive Event Editor for indie/mid workflows: multi-track, instruments, automation, AHDSR, markers, export.

## Done

| ID | Item |
|----|------|
| S5.1 | Multi-track timeline model (`TimelineTrack`) |
| S5.2 | Instrument kinds: Single / Multi / Scatter |
| S5.3 | Automation curves (keyframes, linear eval) |
| S5.4 | Probability on instruments + `LoopRegion` |
| S5.5 | AHDSR on voices (`envelope` + mix) |
| S5.6 | Transition markers (data model) |
| S5.7 | 3D props on `DesignedEvent` |
| S5.8 | `BankManifest::to_json` build export |

## API

```rust
let mut track = TimelineTrack::new("Audio 1");
track.instruments.push(InstrumentDef::single("beep", 0));
let ev = DesignedEvent::new("event:/sfx/beep").with_track(track);
rt.register_designed_event(ev, &[buf])?;
let voices = rt.play_designed("event:/sfx/beep", &[buf])?;
let json = rt.export_bank_manifest("Master").to_json();
```

## Modules

- `runtime/src/designer.rs`
- `runtime/src/automation.rs`
- `runtime/src/envelope.rs`
- `runtime/src/export.rs`

## Test

```bash
cargo test -p ramod_runtime --no-default-features
```

## Follow-ups

- Editor Overview multi-track drawing + drag instruments
- Automation applied during playback over time
- Transition marker runtime jumps
- FSB5 export after `fsb.rs` restore
