# S7.19 — AHDSR per instrument

**Status:** on `main`

## UI

1. Event Editor timeline → **click a clip**
2. Inspector shows Start / Len / Loop / Prob + **AHDSR** (A H D S R)

Units: seconds for A/H/D/R, sustain 0–1.

## Runtime

```
InstrumentEntry.{attack,hold,decay,sustain,release}
  → InstrumentDef.envelope (Ahdsr)
  → play_with_priority(..., Some(envelope))
  → VoicePool::advance_envelopes
```

## Verify

```bash
cargo run -p ramod_editor
# Click beep clip → A=0.2 S=0.5 R=0.3 → Play
```
