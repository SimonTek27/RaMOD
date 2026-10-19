# Phase S5 — Full Event Designer

**Status:** Foundation on `main` (2026-10-19)

## Goal

Competitive Event Editor for indie/mid workflows: multi-track, instruments, automation, AHDSR, markers, export.

## Done (foundation)

| ID | Item | Location |
|----|------|----------|
| S5.1 | Multi-track timeline model | `designer::TimelineTrack` |
| S5.2 | Instrument kinds: Single / Multi / Scatter | `InstrumentKind` |
| S5.3 | Automation curves (keyframes, linear eval) | `automation::AutomationCurve` |
| S5.4 | Probability + loop regions | `InstrumentDef.probability`, `LoopRegion` |
| S5.5 | AHDSR envelope | `envelope::Ahdsr` |
| S5.6 | Transition markers | `TransitionMarker` |
| S5.7 | 3D props on `DesignedEvent` | `is_3d` + `Attenuation` |
| S5.8 | Build → JSON bank manifest export | editor / `export` |

## Runtime modules

- `runtime/src/envelope.rs`
- `runtime/src/automation.rs`
- `runtime/src/designer.rs`

## Next wiring

1. `Runtime::register_designed_event` expands tracks → voices with AHDSR + probability
2. Editor Overview draws multi-track + keyframes
3. Build button writes `*.ramodbank.json`

```bash
cargo test -p ramod_runtime --no-default-features
```
