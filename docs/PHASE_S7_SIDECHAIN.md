# S7.17 — Side-chain compressor

**Status:** on `main`

## Use case

Duck Music when SFX plays (dialogue / impacts priority).

## UI

Profiler bus row → **SC** (hover: Side-chain from SFX).
Typically add on **Music** bus.

Defaults: threshold −24 dB, ratio 6:1, makeup 0 dB, key = `SFX`.

## DSP

Envelope follower on key bus (~5 ms attack / 80 ms release) drives gain reduction on the destination bus.

## Verify

```bash
cargo run -p ramod_editor
# Event output_bus=Music → Play
# Profiler → Music → SC
# Play loud SFX event → Music ducks
```
