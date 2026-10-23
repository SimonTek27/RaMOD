# S7.25 Nested group buses

## Done on `main`

| Piece | Status |
|--------|--------|
| `Bus.parent: Option<BusId>` in mixer | already |
| Hierarchical mix in `Mixer::finish` | already |
| `BusMeterInfo.parent_id` | **yes** |
| `Runtime::create_child_bus` / `set_bus_parent` | **yes** |
| Profiler **+Child** + indent | local `/tmp` — merge after full `audio_bridge` restore |

## How to create a child bus (runtime)

```rust
let sfx = rt.mixer.find_by_name("SFX").unwrap();
let child = rt.create_child_bus(sfx, "SFX_Footsteps");
```

## Restore full editor/runtime sources

```bash
cp artifacts/recovery/core_b.rs runtime/src/runtime_core_b.rs
cp artifacts/recovery/ab.rs     editor/src/audio_bridge.rs
# recovery copies already include parent_id + create_child_bus
```

Then wire ProfilerAction::CreateChildBus in `main.rs` (see recovery notes).

## Next
- S7.26 Discrete / labeled parameters
- Or finish Profiler UI merge for +Child
