# S7.22 recovery

Mixer send/return is on `main` (`runtime/src/mixer.rs`).

If `runtime_core_b.rs` or `editor/src/audio_bridge.rs` look truncated:

```bash
# Restore core_b from last known-good + send APIs
git show 1e377e6:runtime/src/runtime_core_b.rs > runtime/src/runtime_core_b.rs
# Then add to list_bus_meters:
#   send_reverb: self.mixer.send_level(b.id, "Reverb"),
#   is_return: b.is_return,
# And method:
#   pub fn set_bus_send_level(&mut self, bus: BusId, target_name: &str, level: f32) {
#       self.mixer.set_send_level(bus, target_name, level);
#   }

# Restore audio_bridge
git show f33d4403:editor/src/audio_bridge.rs > editor/src/audio_bridge.rs
# Add set_bus_send_reverb calling rt.set_bus_send_level(..., "Reverb", level)
```

Or copy from project artifacts:
- `runtime_core_b_S722_RESTORE.rs`
- `audio_bridge_S722_RESTORE.rs`
